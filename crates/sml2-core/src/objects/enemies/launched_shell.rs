//! ROM `$5AAB9` (mode B anim 98) and the shared body of `$5AB4B` (anim 99).
//!
//! `launch_dx` is −64 for anim 98 and −48 for anim 99. Everything else is the
//! same routine: state 0 arms flash 16, contact 32, state 3, and a 48-step
//! leg. State 3 drops 8 pixels when stood on. State 4 then jumps up 64 pixels
//! and left by `launch_dx`. States 1/2 bounce, 4 pixels per frame or 2 in
//! easy mode. Leaving the downward leg plays sound 10.
use crate::actor::{Actor, ContactKind, FlashFlags};

const LEG: u8 = 48;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Bounce,
}

pub(super) fn update(actor: &mut Actor, easy: bool, launch_dx: i16) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.flash = FlashFlags::from_bits(16);
            actor.contact_kind = ContactKind::from_raw(32);
            actor.state = 3;
            actor.motion_leg = LEG;
            actor.set_travel_span(LEG);
            Ok(Effect::Continue)
        }
        1 | 2 => {
            let step = if easy { 2 } else { 4 };
            let dy = if actor.state == 1 {
                i16::from(step)
            } else {
                -i16::from(step)
            };
            actor.position.move_y(dy);
            let remaining = actor.motion_leg.wrapping_sub(step);
            actor.motion_leg = remaining;
            let bounce = remaining == 0;
            if bounce {
                actor.motion_leg = actor.travel_span();
                let leaving_fall = actor.state == 1;
                actor.state ^= 3;
                if leaving_fall {
                    return Ok(Effect::Bounce);
                }
            }
            Ok(Effect::Continue)
        }
        3 => {
            if !actor.interactions.stood_on() {
                return Ok(Effect::Continue);
            }
            actor.position.move_y(8);
            actor.sprite = 221;
            actor.state = 4;
            actor.timer = 4;
            Ok(Effect::Continue)
        }
        4 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            actor.defeat_mode = 128;
            actor.position.move_x(launch_dx);
            actor.position.move_y(-64);
            actor.sprite = 193;
            actor.state = 1;
            actor.contact_kind = ContactKind::from_raw(72);
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}
