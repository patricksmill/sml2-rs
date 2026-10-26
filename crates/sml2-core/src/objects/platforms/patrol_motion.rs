//! Shared ROM $58C7B patrol for the $58C41 and $58C52 platform entries.
use crate::actor::Actor;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

pub(super) fn patrol(actor: &mut Actor) -> Result<Option<i16>, Error> {
    let span = actor.motion_leg;
    let (dx, cap, next_state) = match actor.state {
        1 => (-1, span, 4),
        2 => (1, span, 3),
        3 => (-1, span.wrapping_mul(2), 4),
        4 => (1, span.wrapping_mul(2), 3),
        state => return Err(Error::InvalidState(state)),
    };
    actor.position.move_x(dx);
    let carry = actor.interactions.stood_on().then_some(dx);
    actor.motion_phase = actor.motion_phase.wrapping_add(1);
    if actor.motion_phase == cap {
        actor.motion_phase = 0;
        actor.state = next_state;
    }
    Ok(carry)
}
