//! Object slot and `$FFD0` staging byte layout.

pub const SLOT_COUNT: usize = 16;
pub const RECORD_LEN: usize = 6;
pub const STREAM_BASE: u16 = 0xAB06;

/// One OAM entry the boss render pass (`$3A14` -> bank 25 `$66000`) emitted,
/// in the console's own OAM byte space — the runtime maps it to screen pixels
/// as `(x - 8, y - 16)`, the same convention the level draw's cells use.
/// `y == 0` is the ROM's own "hide this entry" (it writes 0 over the Y byte
/// when the X byte is past the screen), not a real top-of-screen sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BossOamCell {
    pub y: u8,
    pub x: u8,
    pub tile: u8,
    pub attr: u8,
}

// --- slot / staging byte offsets (ram.asm `$FFD0-$FFEF` map) ---
pub const B_XHI: usize = 0;
pub const B_XLO: usize = 1;
pub const B_MARKER: usize = 2;
pub const B_YHI: usize = 3;
pub const B_YLO: usize = 4;
pub const B_DEF: usize = 5;
pub const B_RECHI: usize = 6;
pub const B_RECLO: usize = 7;
pub const B_STATE: usize = 8;
pub const B_SCRY: usize = 9;
pub const B_SCRX: usize = 10;
pub const B_SPRITE: usize = 11;
pub const B_DEFEAT: usize = 12;
pub const B_FLIP: usize = 13;
pub const B_FLASH: usize = 14;
pub const B_FSTATE: usize = 15;
pub const B_PHASE: usize = 16;
pub const B_PARKED: usize = 17;
pub const B_TYPE: usize = 18;
pub const B_HB_TOP: usize = 19;
pub const B_HB_BOTTOM: usize = 20;
pub const B_HB_LEFT: usize = 21;
pub const B_HB_RIGHT: usize = 22;
pub const B_FLAGS: usize = 23;
pub const B_T24: usize = 24;
pub const B_T25: usize = 25;
pub const B_ANIMFRAME: usize = 26;
pub const B_HITREACT: usize = 27;
pub const B_T28: usize = 28;
pub const B_MASK: usize = 31;

/// The fireball-vs-object pass's verdict (`$3007`): the ROM keeps it in
/// `sObjectHitResult`, and a non-zero value kills the ball.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireHit {
    /// No live object covered the ball's staged point — the ball flies on.
    None,
    /// A bit-2 type: the pass set `B_FLAGS` bit 1, which the interaction
    /// dispatch turns into the object's `defeat()`. No sfx of its own.
    Defeat,
    /// Any other live type: `sSfxRequest0 = 2` (the caller writes it) and the
    /// ball dies.
    Die,
}

/// `$5B451` fall-speed curve (ROM `$5B488`, 22 entries; `+16` indexes it).
pub(super) const FALL_SPEED: [u8; 22] = [
    0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x03, 0x02, 0x03, 0x03, 0x03,
    0x02, 0x03, 0x03, 0x02, 0x03, 0x03,
];
/// `$5B481` hit-reaction bounce curve (ROM `$5B4A4`, 16 entries).
pub(super) const BOUNCE_SPEED: [u8; 16] = [
    0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00,
];
/// `$5B47A` hover curve (ROM `$5B49E`, 22 entries; `+16` indexes it, cap 21)
/// — anim 8's odd-frame rise.
pub(super) const HOVER_SPEED: [u8; 22] = [
    0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02,
    0x01, 0x01, 0x01, 0x00, 0x00, 0x00,
];
/// `$5B545` spin wobble (ROM `$5B57A`, 4 entries; `+26` indexes it).
pub(super) const SPIN_WOBBLE: [u8; 4] = [0xFF, 0x01, 0xFF, 0x01];
/// `$74A1` — the `+24 == 0` step profile of the anim-94/95 floaters (`$7473`,
/// sentinel `$12`). One leg is 18 picker calls whose steps sum to 32 px; the
/// 19th byte is never reached (the caller's tail resets `+16` at the
/// sentinel). The `+24 != 0` side is `$7488`, whose base is `FALL_SPEED`
/// (its c=`$15` fall consumer reads 22 of the same bytes) — shared, so the
/// two consumers cannot drift.
pub(super) const STEP_74A1: [u8; 19] = [
    0x03, 0x02, 0x03, 0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x01,
    0x00, 0x00, 0x00,
];

/// `$74AB` (`$5B465`'s table, c = 8): the defeated enemy's rise curve, 2× per
/// frame in `$581CE`. `$74A1`/`$74A7` are its neighbours, used by the floaters.
pub(super) const DEATH_RISE_74AB: [u8; 8] = [0x02, 0x01, 0x02, 0x01, 0x01, 0x01, 0x00, 0x00];

#[inline]
pub fn x_of(b: &[u8; 32]) -> u16 {
    ((b[B_XHI] as u16) << 8) | b[B_XLO] as u16
}
#[inline]
pub fn set_x(b: &mut [u8; 32], v: u16) {
    b[B_XHI] = (v >> 8) as u8;
    b[B_XLO] = v as u8;
}
#[inline]
pub fn y_of(b: &[u8; 32]) -> u16 {
    ((b[B_YHI] as u16) << 8) | b[B_YLO] as u16
}
#[inline]
pub fn set_y(b: &mut [u8; 32], v: u16) {
    b[B_YHI] = (v >> 8) as u8;
    b[B_YLO] = v as u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlotState {
    #[default]
    Inactive,
    Standby,
    Active,
}

#[derive(Debug, Clone, Default)]
pub struct ObjSlot {
    /// The raw 32 slot bytes — the engine operates on bytes; every field is
    /// an offset (see the `B_*` constants).
    pub b: [u8; 32],
}

impl ObjSlot {
    pub fn x(&self) -> u16 {
        x_of(&self.b)
    }
    pub fn y(&self) -> u16 {
        y_of(&self.b)
    }
    pub fn state(&self) -> SlotState {
        match self.b[B_STATE] {
            0 => SlotState::Inactive,
            1 => SlotState::Standby,
            _ => SlotState::Active,
        }
    }
    pub fn active(&self) -> bool {
        self.state() == SlotState::Active
    }
    pub fn def(&self) -> u8 {
        self.b[B_DEF]
    }
    pub fn sprite(&self) -> u8 {
        self.b[B_SPRITE]
    }
    /// Record byte offset + 2 at spawn time (`0` when never spawned).
    pub fn rec_index(&self) -> u16 {
        ((self.b[B_RECHI] as u16) << 8) | self.b[B_RECLO] as u16
    }
}

/// The `$FFD0-$FFEF` staging copy + `$FFB7-BA` probe scratch.
#[derive(Clone, Default)]
pub struct Staging {
    pub b: [u8; 32],
    pub px: u16,
    pub py: u16,
}

impl Staging {
    pub fn x(&self) -> u16 {
        x_of(&self.b)
    }
    pub fn y(&self) -> u16 {
        y_of(&self.b)
    }
}

/// Mario state the engine reads (post player-update, pre object engine).
#[derive(Debug, Clone, Copy)]
pub struct MarioContact {
    pub x: u16,
    pub y: u16,
    /// `sMarioDead`.
    pub dead: u8,
    /// `sCurPowerup` (0 = small).
    pub powerup: u8,
    /// `sMarioDucking`.
    pub ducking: bool,
    /// `sSpinJump`.
    pub spin_jump: bool,
    /// `sStarTimerHigh`.
    pub star_high: u8,
    /// `sInvincibilityTimer`.
    pub invincibility: u8,
    /// `sFlutterTimer`.
    pub flutter: u8,
    /// `sVVelocityIndex`.
    pub vvel: u8,
    /// `sMarioOnGround`.
    pub on_ground: bool,
    /// `sMarioOnObject`.
    pub on_object: bool,
    /// `sMarioDirection` ($A22B): `$01` right / `$FF` left. The shell
    /// placement tails (`$864A`/`$584A9`) test it, not the position.
    pub dir_right: bool,
    /// `hKeysHeld` bit 1 (B).
    pub b_held: bool,
    /// `hKeysHeld` bit 0 (A).
    pub a_held: bool,
    /// `sSceneState`.
    pub scene_state: u8,
    /// `sMoonPhysics`.
    pub moon_physics: u8,
    /// `sCameraPanTimer` (the `$2062` scroll-Y nudge).
    pub pan: u8,
    /// `sPipeTravelDirection >= $10`: the frame loop skips `$2D7D` entirely
    /// while Mario is inside a pipe (`$05E4` line `ld a,[sPipeTravelDirection]`).
    pub travelling: bool,
    /// `sHVelocityIndex` ($A200): $80 idle, < $80 moving left, >= $80 moving right.
    pub hvel: u8,
}

impl Default for MarioContact {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            dead: 0,
            powerup: 0,
            ducking: false,
            spin_jump: false,
            star_high: 0,
            invincibility: 0,
            flutter: 0,
            vvel: 0,
            on_ground: false,
            on_object: false,
            dir_right: false,
            b_held: false,
            a_held: false,
            scene_state: 0,
            moon_physics: 0,
            pan: 0,
            travelling: false,
            hvel: 0x80,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotView {
    pub state: SlotState,
    pub x: u16,
    pub y: u16,
    pub def: u8,
    pub sprite: u8,
    pub rec_index: u16,
}

impl From<&ObjSlot> for SlotView {
    fn from(s: &ObjSlot) -> Self {
        Self {
            state: s.state(),
            x: s.x(),
            y: s.y(),
            def: s.def(),
            sprite: s.sprite(),
            rec_index: s.rec_index(),
        }
    }
}
