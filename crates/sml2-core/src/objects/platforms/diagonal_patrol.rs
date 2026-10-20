//! ROM `$58CE0` (anim 44): diagonal patrol platform.
//!
//! State 0 arms contact 32, faces Mario (`$5B3C6`), and sets the leg counter
//! (`$FFE8`) to 24 with reload 48 in `$FFE9`. States 1/2 run only on even
//! `$FFF0` frames: state 1 steps (−1, +1); state 2 steps (+1, −1), carrying a
//! stood-on rider on both axes. When the leg counter hits zero it reloads from
//! `$FFE9` and toggles with `state ^= 3`.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

/// ROM `$58CEC`: `ld a,24`.
const LEG: u8 = 24;
/// ROM `$58CF0`: `add a` after loading 24 → 48 in `$FFE9`.
const RELOAD: u8 = 48;

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
    if actor.state != 0 && !frame.every(2) {
        return Ok(Effect::Idle);
    }
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            let relation = actor.face_toward(player_x);
            actor.motion_leg = LEG;
            actor.timer = RELOAD;
            Ok(Effect::Initialized { facing: relation })
        }
        1 => step(actor, -1, 1),
        2 => step(actor, 1, -1),
        state => Err(Error::InvalidState(state)),
    }
}

fn step(actor: &mut Actor, dx: i16, dy: i16) -> Result<Effect, Error> {
    actor.position.move_x(dx);
    actor.position.move_y(dy);
    let riding = actor.interactions.stood_on();
    let carry_dx = riding.then_some(dx);
    let carry_dy = riding.then_some(dy);
    actor.motion_leg = actor.motion_leg.wrapping_sub(1);
    if actor.motion_leg != 0 {
        return Ok(Effect::Moved { carry_dx, carry_dy });
    }
    actor.motion_leg = actor.timer;
    actor.state ^= 3;
    Ok(Effect::Moved { carry_dx, carry_dy })
}
