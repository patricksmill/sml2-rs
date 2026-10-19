//! ROM `$595FE` (anim 79): vertical oscillating platform.
//!
//! State 0 arms contact 72, nudges world X left by 4, and loads a 64-step leg
//! with a 32-frame sprite timer. States 1/2 alternate Y motion by 2 px; the
//! sprite timer must reach zero (`$5B68B`) before each step. The leg reloads and
//! `state ^= 3` when the leg expires.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

pub(super) const CONTACT: u8 = 72;
pub(super) const LEG: u8 = 64;
pub(super) const SPRITE_TIMER: u8 = 32;
const STEP: i16 = 2;
const NUDGE: i16 = -4;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Moved { dy: i16 },
    Waiting,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = 1;
            actor.position.move_x(NUDGE);
            actor.hit_reaction = LEG;
            actor.timer = SPRITE_TIMER;
            Ok(Effect::Initialized)
        }
        1 | 2 => {
            if actor.timer != 0 {
                if frame.every(2) {
                    actor.flash.toggle_palette();
                }
                actor.timer -= 1;
                if actor.timer != 0 {
                    return Ok(Effect::Waiting);
                }
                actor.flash = FlashFlags::from_bits(16);
            }
            let dy = if actor.state == 1 { STEP } else { -STEP };
            actor.position.move_y(dy);
            actor.hit_reaction = actor.hit_reaction.wrapping_sub(2);
            if actor.hit_reaction != 0 {
                return Ok(Effect::Moved { dy });
            }
            actor.hit_reaction = LEG;
            actor.timer = SPRITE_TIMER;
            actor.state ^= 3;
            Ok(Effect::Moved { dy })
        }
        state => Err(Error::InvalidState(state)),
    }
}
