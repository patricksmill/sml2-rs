//! ROM `$59BE1` (mode A anim 89): a 32-frame flash, then removal.
use crate::actor::{Actor, FlashFlags};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Remove,
}

pub(super) fn update(actor: &mut Actor) -> Result<Effect, Error> {
    match actor.state {
        // $59BE8
        0 => {
            actor.flash = FlashFlags::from_bits(16);
            actor.state = 1;
            actor.timer = 32;
            Ok(Effect::Continue)
        }
        // $59BF5
        1 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            Ok(Effect::Remove)
        }
        state => Err(Error::InvalidState(state)),
    }
}
