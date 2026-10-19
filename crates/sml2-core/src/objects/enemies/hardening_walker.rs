//! ROM `$5B06D` (mode B anim 89): a walker that rises, falls, hardens and dies.
//!
//! State 0 faces Mario, arms contact 72 and lifts 16 px. States 1/2 walk one
//! pixel and run the vertical oscillator: `+24 == 0` rises on the twelve-step
//! `$74A7` curve; otherwise it falls on `$5B451` and, on the own-Y ground probe,
//! becomes sprite 113 with state 3 and a 16-frame timer. State 3 then removes.
use super::falling_dropper::step_fall_speed_cap;
use crate::actor::{Actor, ContactKind, MotionProfileError};

const CONTACT: u8 = 72;
const LANDED_SPRITE: u8 = 113;
const LANDED_TIMER: u8 = 16;
/// ROM `$74A7`, selected by `$5B46C` with cap twelve.
const RISE_74A7: [u8; 13] = [3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0, 0];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Motion(MotionProfileError),
}
impl From<MotionProfileError> for Error {
    fn from(error: MotionProfileError) -> Self {
        Self::Motion(error)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Remove,
}

pub(super) trait World {
    fn probe_ground(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    player_x: u16,
    world: &mut impl World,
) -> Result<Effect, Error> {
    match actor.state {
        // $5B078
        0 => {
            actor.position.move_y(-16);
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.face_toward(player_x);
            Ok(Effect::Continue)
        }
        // $5B085
        1 => {
            actor.position.move_x(-1);
            vertical(actor, world)
        }
        // $5B08C
        2 => {
            actor.position.move_x(1);
            vertical(actor, world)
        }
        // $5B0C6
        3 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            Ok(Effect::Remove)
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// `$5B091`: rise on `$74A7` (cap 12) or fall on `$5B451` until landing.
fn vertical(actor: &mut Actor, world: &mut impl World) -> Result<Effect, Error> {
    if actor.motion_leg != 0 {
        if world.probe_ground(actor) {
            actor.sprite = LANDED_SPRITE;
            actor.state = 3;
            actor.timer = LANDED_TIMER;
            return Ok(Effect::Continue);
        }
        let speed = step_fall_speed_cap(actor, 12);
        actor.position.move_y(i16::from(speed));
        return Ok(Effect::Continue);
    }
    let step = actor.motion_step(&RISE_74A7, 12)?;
    actor.position.move_y(-i16::from(step));
    if actor.motion_phase != 12 {
        return Ok(Effect::Continue);
    }
    actor.motion_phase = 0;
    actor.motion_leg = 1;
    Ok(Effect::Continue)
}
