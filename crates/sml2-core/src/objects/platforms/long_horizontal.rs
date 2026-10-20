//! ROM `$58C41` (anim 43): horizontal platform like `$58C52`, but the leg
//! length is 60 in `$FFE8` (`motion_leg`) and state 0 omits flash 16.
//!
//! States 1/2 run the short legs; 3/4 alternate at `2 * span` (`state ^= 7`).
//! `$3CF1`/`$3CC0` carry a stood-on rider by the same pixel.
use crate::actor::{Actor, ContactKind};
use std::cmp::Ordering;

/// ROM `$58C4D`: `ld a,60`.
pub(super) const SPAN: u8 = 60;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Moved { carry_dx: Option<i16> },
}

pub(super) fn update(actor: &mut Actor, player_x: u16) -> Result<Effect, Error> {
    match actor.state {
        // $58C46: contact 32, face by X, span in $FFE8.
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            let relation = actor.face_toward(player_x);
            actor.motion_leg = SPAN;
            Ok(Effect::Initialized { facing: relation })
        }
        1 => {
            let (carry, done) = step(actor, -1, actor.motion_leg);
            if done {
                actor.state = 4;
            }
            Ok(Effect::Moved { carry_dx: carry })
        }
        2 => {
            let (carry, done) = step(actor, 1, actor.motion_leg);
            if done {
                actor.state = 3;
            }
            Ok(Effect::Moved { carry_dx: carry })
        }
        3 => {
            let cap = actor.motion_leg.wrapping_mul(2);
            let (carry, done) = step(actor, -1, cap);
            if done {
                actor.state ^= 7;
            }
            Ok(Effect::Moved { carry_dx: carry })
        }
        4 => {
            let cap = actor.motion_leg.wrapping_mul(2);
            let (carry, done) = step(actor, 1, cap);
            if done {
                actor.state ^= 7;
            }
            Ok(Effect::Moved { carry_dx: carry })
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn step(actor: &mut Actor, dx: i16, cap: u8) -> (Option<i16>, bool) {
    actor.position.move_x(dx);
    let carry = actor.interactions.stood_on().then_some(dx);
    actor.motion_phase = actor.motion_phase.wrapping_add(1);
    let done = actor.motion_phase == cap;
    if done {
        actor.motion_phase = 0;
    }
    (carry, done)
}
