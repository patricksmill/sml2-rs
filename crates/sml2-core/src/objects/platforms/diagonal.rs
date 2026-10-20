//! ROM `$58D34` (anim 45): a diagonal moving platform.
//!
//! State 0 arms contact 32, faces Mario (`$5B3C6`) and stores the half-span 24
//! in `$FFE8`. Motion runs only on even `$FFF0` frames. States 1/2 are the
//! short legs (up-left / down-right) and hand off to states 4/3. States 3/4
//! are the long legs at `2 * span` and toggle with `state ^= 7`. A stood-on
//! rider is carried on both axes.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

/// ROM `$58D40`: `ld a,24`.
pub(super) const SPAN: u8 = 24;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized {
        facing: Ordering,
    },
    Moved {
        carry_dx: Option<i16>,
        carry_dy: Option<i16>,
    },
    Idle,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(32);
        let relation = actor.face_toward(player_x);
        actor.motion_leg = SPAN;
        return Ok(Effect::Initialized { facing: relation });
    }
    // $58D45: odd `$FFF0` frames return before the state table.
    if !frame.every(2) {
        return Ok(Effect::Idle);
    }
    match actor.state {
        // $58D56: short up-left leg, then state 4.
        1 => {
            let effect = step(actor, -1, -1);
            finish_short(actor, 4);
            Ok(effect)
        }
        // $58D78: short down-right leg, then state 3.
        2 => {
            let effect = step(actor, 1, 1);
            finish_short(actor, 3);
            Ok(effect)
        }
        // $58D9A: long up-left leg; `state ^= 7` flips 3 <-> 4.
        3 => {
            let effect = step(actor, -1, -1);
            finish_long(actor);
            Ok(effect)
        }
        // $58DBF: long down-right leg, joining the same counter.
        4 => {
            let effect = step(actor, 1, 1);
            finish_long(actor);
            Ok(effect)
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn step(actor: &mut Actor, dx: i16, dy: i16) -> Effect {
    actor.position.move_x(dx);
    actor.position.move_y(dy);
    let riding = actor.interactions.stood_on();
    Effect::Moved {
        carry_dx: riding.then_some(dx),
        carry_dy: riding.then_some(dy),
    }
}

/// `$58D69`: count to the stored span, then enter `next`.
fn finish_short(actor: &mut Actor, next: u8) {
    actor.motion_phase = actor.motion_phase.wrapping_add(1);
    if actor.motion_phase == actor.motion_leg {
        actor.motion_phase = 0;
        actor.state = next;
    }
}

/// `$58DAA`: count to `2 * span`, then `state ^= 7`.
fn finish_long(actor: &mut Actor) {
    let cap = actor.motion_leg.wrapping_mul(2);
    actor.motion_phase = actor.motion_phase.wrapping_add(1);
    if actor.motion_phase == cap {
        actor.motion_phase = 0;
        actor.state ^= 7;
    }
}
