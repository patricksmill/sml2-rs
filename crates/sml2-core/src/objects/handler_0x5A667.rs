//! ROM `$5A667` (mode B anim 72): a bobbing walker.
//!
//! State 0 arms contact 77 and faces Mario. Later calls run the `$5B57E`
//! four-step sprite bob, then the shared `$58226`/`$58253`/`$58271`/`$58282`
//! walker bodies.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

pub(super) const CONTACT: u8 = 77;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Walk,
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(CONTACT);
        return Ok(Effect::Initialized {
            facing: actor.face_toward(player_x),
        });
    }
    let _ = actor.animate_four_step_bob_cadence(frame, 4);
    match actor.state {
        1..=4 => Ok(Effect::Walk),
        state => Err(Error::InvalidState(state)),
    }
}
