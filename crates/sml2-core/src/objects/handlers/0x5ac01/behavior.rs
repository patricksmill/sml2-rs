//! ROM `$5AC01` (mode B anim 101): the post-defeat fade.
//!
//! State 0 starts a 56-frame timer. On expiry, easy mode removes the object and
//! decrements the boss-defeated counter; otherwise it decrements the definition,
//! runs the `$863C` spawn reset, sets contact 73 and restores the state from the
//! visibility byte (`$FFEF`).
use crate::actor::{Actor, ContactKind};

const WAIT: u8 = 56;
pub(super) const CONTACT: u8 = 73;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Rebirth,
    RemoveBoss,
}

pub(super) fn update(actor: &mut Actor, easy_mode: bool) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.state = 1;
            actor.timer = WAIT;
            Ok(Effect::Continue)
        }
        1 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            if easy_mode {
                return Ok(Effect::RemoveBoss);
            }
            // $3AB4 = `$85DC`: decrement the definition, then $863C.
            actor.definition = actor.definition.wrapping_sub(1);
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            Ok(Effect::Rebirth)
        }
        state => Err(Error::InvalidState(state)),
    }
}
