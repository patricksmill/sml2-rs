//! Shared post-defeat fall, boss anim 3, ROM `$9323` (bank 2 CPU `$5323`): every
//! defeated boss flips upside down, hops, falls off the screen and clears its slot.
use crate::actor::{Actor, ContactKind, RiseProfile};

/// Below this screen Y the defeated boss has fallen out of view.
const OFFSCREEN_Y: u8 = 0xD0;
/// Frames the primary boss waits off screen before its slot clears.
const CLEAR_WAIT: u8 = 208;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Effects {
    pub sound0: Option<u8>,
    pub music: Option<u8>,
    /// The fight's remove mode becomes 2 (`$AF26`).
    pub remove_mode_two: bool,
    pub finish: Option<Finish>,
}

/// How the slot ends: `$B15B` blanks it; `$B157` also counts the definition off.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Finish {
    Blank,
    Counted,
}

#[derive(Debug)]
pub(super) enum UpdateError {
    UnsupportedState,
    InvalidMotionProfile,
}

/// `primary` is the first boss slot; other slots fall without the wait and music.
pub(super) fn update(actor: &mut Actor, primary: bool, fall: &[u8]) -> Result<Effects, UpdateError> {
    let mut effects = Effects::default();
    match actor.state {
        0 => {
            actor.screen.move_y(-32);
            actor.sprite_flip.toggle_vertical();
            actor.motion_phase = 0;
            actor.contact_kind = ContactKind::NONE;
            actor.state = 1;
            effects.sound0 = Some(if primary { 13 } else { 23 });
        }
        // `$5354`: hop up on the cap-15 rise curve.
        1 => {
            let step = actor
                .rise_step(RiseProfile::Low)
                .map_err(|_| UpdateError::InvalidMotionProfile)?;
            actor.screen.move_y(-i16::from(step));
            if actor.motion_phase == RiseProfile::Low.cap() {
                actor.state = 2;
                actor.motion_phase = 0;
            }
        }
        // `$5366`: fall until off screen.
        2 => {
            let step = actor
                .motion_step(fall, 21)
                .map_err(|_| UpdateError::InvalidMotionProfile)?;
            actor.screen.move_y(i16::from(step));
            if actor.screen.y < OFFSCREEN_Y {
                return Ok(effects);
            }
            if !primary {
                *actor = Actor::default();
                effects.remove_mode_two = true;
                effects.finish = Some(Finish::Counted);
                return Ok(effects);
            }
            actor.state = 3;
            actor.timer = CLEAR_WAIT;
            effects.music = Some(12);
        }
        // `$5385`: wait, then clear the slot.
        3 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                *actor = Actor::default();
                effects.remove_mode_two = true;
                effects.finish = Some(Finish::Blank);
            }
        }
        _ => return Err(UpdateError::UnsupportedState),
    }
    Ok(effects)
}

// Compatibility adapter: slot conversion and the engine's slot bookkeeping.
use crate::object::ObjectTables;
use crate::objects::{ObjectSim, Staging};

impl ObjectSim {
    pub(in crate::objects) fn handler_boss_anim3(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let state = actor.state;
        let result = update(&mut actor, self.boss_slot_ptr_lo == 0, &tables.boss_picker_profile);
        st.b = actor.to_raw_slot();
        let effects = match result {
            Ok(effects) => effects,
            Err(UpdateError::UnsupportedState) => {
                return self.note_unported(0x5354 + u32::from(state) * 2);
            }
            Err(UpdateError::InvalidMotionProfile) => return self.note_unported(0x9323),
        };
        if let Some(sound) = effects.sound0 {
            self.sfx0 = sound;
        }
        if let Some(music) = effects.music {
            self.music_request = Some(music);
        }
        if effects.remove_mode_two {
            self.remove_mode = 2;
        }
        match effects.finish {
            Some(Finish::Blank) => self.done = 1,
            Some(Finish::Counted) => {
                self.boss_def_count = self.boss_def_count.wrapping_sub(1);
                self.done = 1;
            }
            None => {}
        }
    }
}
