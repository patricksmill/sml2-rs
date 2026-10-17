//! ROM `$5AAB9` (mode B anim 98): a shell that drops when stood on, then bounces.
//!
//! State 0 arms flash 16, contact 32, state 3, and a 48-step leg. State 3
//! waits for a rider, drops 8 pixels, and starts a 4-frame fuse. State 4 then
//! jumps 64 pixels up and left into the bounce. States 1/2 move 4 pixels per
//! frame, or 2 in easy mode, and swap with `state ^= 3`. Leaving state 1 plays
//! sound 10.
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

pub(super) fn update(actor: &mut Actor, easy: bool) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.flash = FlashFlags::from_bits(16);
            actor.contact_kind = ContactKind::from_raw(32);
            actor.state = 3;
            actor.hit_reaction = LEG;
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
            let remaining = actor.hit_reaction.wrapping_sub(step);
            actor.hit_reaction = remaining;
            let bounce = remaining == 0;
            if bounce {
                actor.hit_reaction = actor.travel_span();
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
            actor.position.move_x(-64);
            actor.position.move_y(-64);
            actor.sprite = 193;
            actor.state = 1;
            actor.contact_kind = ContactKind::from_raw(72);
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}
