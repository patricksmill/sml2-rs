//! ROM `$5A236` (mode B anim 62): a conveyor platform.
//!
//! State 0 arms contact 32 and advances to state 1. State 1 cycles a
//! three-step sprite delta (`$75D3`: −2, +1, +1) every fourth frame, and on
//! even frames carries a stood-on rider one pixel to the right (`$3CC0`).
use crate::actor::{Actor, ContactKind, FrameTick};

const DELTAS: [i8; 3] = [-2, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Carried,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(32);
        actor.state = 1;
        return Ok(Effect::Continue);
    }
    if actor.state != 1 {
        return Err(Error::InvalidState(actor.state));
    }
    if frame.every(4) {
        actor.animation_frame = actor.animation_frame.wrapping_add(1);
        if actor.animation_frame == 3 {
            actor.animation_frame = 0;
        }
        let delta = DELTAS[usize::from(actor.animation_frame)];
        actor.sprite = actor.sprite.wrapping_add_signed(delta);
    }
    if !frame.every(2) || !actor.interactions.stood_on() {
        return Ok(Effect::Continue);
    }
    Ok(Effect::Carried)
}
