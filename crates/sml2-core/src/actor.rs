//! Native actor state. Slot byte layout is confined to the conversion boundary.

#[path = "actor_motion.rs"]
mod motion;
pub use motion::RiseProfile;

#[path = "actor_screen.rs"]
mod screen;

#[path = "actor_ending_animation.rs"]
mod ending_animation;
pub use ending_animation::EndingBobError;

#[path = "actor_sprite_sequence.rs"]
mod sprite_sequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub fn move_x(&mut self, displacement: i16) {
        self.x = self.x.wrapping_add_signed(displacement);
    }

    pub fn move_y(&mut self, displacement: i16) {
        self.y = self.y.wrapping_add_signed(displacement);
    }
}

/// Screen coordinates wrap independently from world coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenPosition {
    pub x: u8,
    pub y: u8,
}

impl ScreenPosition {
    pub fn move_x(&mut self, displacement: i16) {
        self.x = self.x.wrapping_add(displacement as u8);
    }

    pub fn move_y(&mut self, displacement: i16) {
        self.y = self.y.wrapping_add(displacement as u8);
    }
}

/// Source directions test zero/nonzero; reversal toggles only bit zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Facing(u8);

impl Facing {
    pub const LEFT: Self = Self(0);
    pub const RIGHT: Self = Self(1);

    pub const fn from_raw(value: u8) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u8 {
        self.0
    }

    pub const fn is_right(self) -> bool {
        self.0 != 0
    }

    pub fn reverse(&mut self) {
        self.0 ^= 1;
    }

    pub fn displacement(self, distance: u8) -> i16 {
        if self.is_right() {
            i16::from(distance)
        } else {
            -i16::from(distance)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContactKind(u8);

impl ContactKind {
    pub const NONE: Self = Self(0);
    pub const WARIO_CHILD: Self = Self(0x40);

    pub const fn from_raw(value: u8) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> u8 {
        self.0
    }
    pub const fn is_item(self) -> bool {
        self.0 & 0x80 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpriteFlipFlags(u8);

impl SpriteFlipFlags {
    pub const fn from_bits(value: u8) -> Self {
        Self(value)
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    pub const fn horizontal(self) -> bool {
        self.0 & 0x20 != 0
    }
    pub const fn vertical(self) -> bool {
        self.0 & 0x40 != 0
    }
    pub fn toggle_horizontal(&mut self) {
        self.0 ^= 0x20;
    }
    pub fn set_horizontal(&mut self, enabled: bool) {
        self.0 = (self.0 & !0x20) | if enabled { 0x20 } else { 0 };
    }
    pub fn toggle_vertical(&mut self) {
        self.0 ^= 0x40;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlashFlags(u8);

impl FlashFlags {
    pub const fn from_bits(value: u8) -> Self {
        Self(value)
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    pub fn clear(&mut self) {
        self.0 = 0;
    }
    pub fn toggle_palette(&mut self) {
        self.0 ^= 0x10;
    }

    /// ROM `$B40E`: toggle the palette every fourth frame.
    pub fn animate_palette(&mut self, frame: FrameTick) {
        self.flash_palette_on_tick(frame, 4);
    }

    pub fn flash_palette_on_tick(&mut self, frame: FrameTick, period: u64) {
        if frame.every(period) {
            self.toggle_palette();
        }
    }
}

/// Interaction results are separate from the actor's contact kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InteractionFlags(u8);

impl InteractionFlags {
    pub const fn from_bits(value: u8) -> Self {
        Self(value)
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    pub const fn stomped(self) -> bool {
        self.0 & 0x01 != 0
    }
    pub const fn projectile_defeat(self) -> bool {
        self.0 & 0x02 != 0
    }
    pub const fn star_defeat(self) -> bool {
        self.0 & 0x04 != 0
    }
    pub const fn touched(self) -> bool {
        self.0 & 0x10 != 0
    }
    pub const fn struck_from_below(self) -> bool {
        self.0 & 0x20 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hitbox {
    pub top: i8,
    pub bottom: i8,
    pub left: i8,
    pub right: i8,
}

/// Native simulation frame counter used for cadence decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameTick(pub u64);

impl FrameTick {
    pub const fn every(self, period: u64) -> bool {
        assert!(period > 0, "frame cadence period must be positive");
        self.0 % period == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionProfileError {
    pub phase: u8,
    pub cap: u8,
    pub profile_len: usize,
}

/// An incremented animation phase outside the source's four-entry table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationFrameError {
    pub frame: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Actor {
    pub position: Position,
    pub marker: u8,
    pub definition: u8,
    pub spawn_record: u16,
    pub activity: u8,
    pub screen: ScreenPosition,
    pub sprite: u8,
    pub defeat_mode: u8,
    pub sprite_flip: SpriteFlipFlags,
    pub flash: FlashFlags,
    pub state: u8,
    pub motion_phase: u8,
    pub parked_contact: u8,
    pub contact_kind: ContactKind,
    pub hitbox: Hitbox,
    pub interactions: InteractionFlags,
    pub motion_leg: u8,
    pub travel_direction: Facing,
    pub animation_frame: u8,
    pub hit_reaction: u8,
    pub timer: u8,
    reserved29: u8,
    reserved30: u8,
    pub visibility_mask: u8,
}

impl Actor {
    pub fn from_raw_slot(slot: &[u8; 32]) -> Self {
        Self {
            position: Position {
                x: u16::from_be_bytes([slot[0], slot[1]]),
                y: u16::from_be_bytes([slot[3], slot[4]]),
            },
            marker: slot[2],
            definition: slot[5],
            spawn_record: u16::from_be_bytes([slot[6], slot[7]]),
            activity: slot[8],
            screen: ScreenPosition {
                x: slot[10],
                y: slot[9],
            },
            sprite: slot[11],
            defeat_mode: slot[12],
            sprite_flip: SpriteFlipFlags::from_bits(slot[13]),
            flash: FlashFlags::from_bits(slot[14]),
            state: slot[15],
            motion_phase: slot[16],
            parked_contact: slot[17],
            contact_kind: ContactKind::from_raw(slot[18]),
            hitbox: Hitbox {
                top: slot[19] as i8,
                bottom: slot[20] as i8,
                left: slot[21] as i8,
                right: slot[22] as i8,
            },
            interactions: InteractionFlags::from_bits(slot[23]),
            motion_leg: slot[24],
            travel_direction: Facing::from_raw(slot[25]),
            animation_frame: slot[26],
            hit_reaction: slot[27],
            timer: slot[28],
            reserved29: slot[29],
            reserved30: slot[30],
            visibility_mask: slot[31],
        }
    }

    pub fn to_raw_slot(&self) -> [u8; 32] {
        let [x_high, x_low] = self.position.x.to_be_bytes();
        let [y_high, y_low] = self.position.y.to_be_bytes();
        let [record_high, record_low] = self.spawn_record.to_be_bytes();
        [
            x_high,
            x_low,
            self.marker,
            y_high,
            y_low,
            self.definition,
            record_high,
            record_low,
            self.activity,
            self.screen.y,
            self.screen.x,
            self.sprite,
            self.defeat_mode,
            self.sprite_flip.bits(),
            self.flash.bits(),
            self.state,
            self.motion_phase,
            self.parked_contact,
            self.contact_kind.raw(),
            self.hitbox.top as u8,
            self.hitbox.bottom as u8,
            self.hitbox.left as u8,
            self.hitbox.right as u8,
            self.interactions.bits(),
            self.motion_leg,
            self.travel_direction.raw(),
            self.animation_frame,
            self.hit_reaction,
            self.timer,
            self.reserved29,
            self.reserved30,
            self.visibility_mask,
        ]
    }

    /// Read the current phase, then increment unless it equals the source cap.
    /// A phase above the cap still increments, matching the equality test.
    pub fn motion_step(&mut self, profile: &[u8], cap: u8) -> Result<u8, MotionProfileError> {
        if usize::from(cap) >= profile.len() || usize::from(self.motion_phase) >= profile.len() {
            return Err(MotionProfileError {
                phase: self.motion_phase,
                cap,
                profile_len: profile.len(),
            });
        }
        let step = profile[usize::from(self.motion_phase)];
        if self.motion_phase != cap {
            self.motion_phase = self.motion_phase.wrapping_add(1);
        }
        Ok(step)
    }

    /// ROM `$B2EF/$B302`: alternate the low sprite bit at the chosen cadence.
    pub fn animate_two_frame_toggle(&mut self, frame: FrameTick, period: u64) {
        self.animate_sprite_blink(frame, period, 1);
    }

    /// ROM `$B321` uses mask `$1F`; only the sprite changes on cadence ticks.
    pub fn animate_sprite_blink(&mut self, frame: FrameTick, period: u64, sprite_mask: u8) {
        if frame.every(period) {
            self.sprite ^= sprite_mask;
        }
    }

    /// ROM `$B339`: advance before applying the four-step sprite bob.
    /// Only equality with four resets the phase; byte overflow also reaches zero.
    /// An invalid table index leaves the advanced phase and the unchanged sprite.
    pub fn animate_four_step_bob(&mut self, frame: FrameTick) -> Result<(), AnimationFrameError> {
        if !frame.every(4) {
            return Ok(());
        }
        self.animation_frame = self.animation_frame.wrapping_add(1);
        if self.animation_frame == 4 {
            self.animation_frame = 0;
        }
        // ROM `$B356..$B359`: FE 01 FF 02.
        const DELTAS: [i8; 4] = [-2, 1, -1, 2];
        let delta = DELTAS
            .get(usize::from(self.animation_frame))
            .ok_or(AnimationFrameError {
                frame: self.animation_frame,
            })?;
        self.sprite = self.sprite.wrapping_add_signed(*delta);
        Ok(())
    }

    pub fn move_screen_horizontally(&mut self, distance: u8) {
        self.screen
            .move_x(self.travel_direction.displacement(distance));
    }
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;
