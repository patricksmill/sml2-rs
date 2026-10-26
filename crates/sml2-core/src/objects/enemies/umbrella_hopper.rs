//! ROM `$599A6` (mode A anim 86): an umbrella hopper that launches upward,
//! settles, then sways back and forth while gliding down until it lands and
//! launches again.
//!
//! State 0 arms contact 77 and enters the launch. State 5 rises two pixels
//! per call until `$FFE8` equals 64; state 6 drops three pixels per call
//! until it counts 9 down to zero and switches to sprite `$97`; state 7 drops
//! on the `$5B465` curve and faces Mario (`$5B3C6`) when its phase reaches 8.
//!
//! States 1-4 glide. X follows `$5A9D` by the `$FFE9` cursor (1 and 2 count
//! it up, 3 and 4 count it down) and Y follows `$5A8D` by the `$FFE8` cursor.
//! Every 16 Y steps the sway reverses: 1 -> 3 -> 2 -> 4 -> 1. Ground under
//! the hopper (`$3B40`) clears both cursors and the phase, restores sprite
//! `$96` and relaunches from state 5.
use crate::actor::{Actor, ContactKind, DEFEAT_RISE_STEPS};
use std::cmp::Ordering;

pub(super) const CONTACT: u8 = 77;
pub(super) const LAUNCH_SPRITE: u8 = 0x96;
pub(super) const GLIDE_SPRITE: u8 = 0x97;

/// `$59A8D`: glide descent per call.
const GLIDE_Y: [u8; 16] = [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1];
/// `$59A9D`: sway speed per call.
const SWAY_X: [u8; 16] = [3, 3, 3, 3, 3, 3, 2, 2, 1, 1, 0, 0, 0, 0, 0, 0];

const LAUNCH_HEIGHT: u8 = 0x40;
const SETTLE_DISTANCE: u8 = 9;
const SWAY_STEPS: u8 = 16;
const SETTLE_CURVE_END: u8 = 8;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    CursorOutOfRange(u8),
    Curve,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Armed,
    Moved,
    Landed,
    FacedMario { facing: Ordering },
}

pub trait World {
    /// `$3B40`: true when the tile under the actor is solid ground.
    fn grounded(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    player_x: u16,
    world: &mut impl World,
) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = 5;
            Ok(Effect::Armed)
        }
        // $599C4
        5 => {
            actor.position.y = actor.position.y.wrapping_sub(2);
            actor.motion_leg = actor.motion_leg.wrapping_add(2);
            if actor.motion_leg == LAUNCH_HEIGHT {
                actor.motion_leg = SETTLE_DISTANCE;
                actor.state = 6;
            }
            Ok(Effect::Moved)
        }
        // $599DA
        6 => {
            actor.position.y = actor.position.y.wrapping_add(3);
            actor.motion_leg = actor.motion_leg.wrapping_sub(3);
            if actor.motion_leg == 0 {
                actor.state = 7;
                actor.sprite = GLIDE_SPRITE;
            }
            Ok(Effect::Moved)
        }
        // $599EE
        7 => {
            let step = actor
                .motion_step(&DEFEAT_RISE_STEPS, SETTLE_CURVE_END)
                .map_err(|_| Error::Curve)?;
            actor.position.y = actor.position.y.wrapping_add(u16::from(step));
            if actor.motion_phase != SETTLE_CURVE_END {
                return Ok(Effect::Moved);
            }
            actor.motion_phase = 0;
            let facing = actor.face_toward(player_x);
            Ok(Effect::FacedMario { facing })
        }
        // $59A0F / $59A5D / $59A6D / $59A7D
        state @ 1..=4 => {
            let step = *SWAY_X
                .get(usize::from(actor.motion_counter()))
                .ok_or(Error::CursorOutOfRange(actor.motion_counter()))?;
            actor.position.x = if matches!(state, 2 | 3) {
                actor.position.x.wrapping_add(u16::from(step))
            } else {
                actor.position.x.wrapping_sub(u16::from(step))
            };
            let cursor = if state <= 2 {
                actor.motion_counter().wrapping_add(1)
            } else {
                actor.motion_counter().wrapping_sub(1)
            };
            actor.set_motion_counter(cursor);
            glide(actor, world)
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// `$59A1D`: land and relaunch, or descend and reverse the sway.
fn glide(actor: &mut Actor, world: &mut impl World) -> Result<Effect, Error> {
    if world.grounded(actor) {
        // $599FF
        actor.motion_leg = 0;
        actor.set_motion_counter(0);
        actor.motion_phase = 0;
        actor.state = 5;
        actor.sprite = LAUNCH_SPRITE;
        return Ok(Effect::Landed);
    }
    let step = *GLIDE_Y
        .get(usize::from(actor.motion_leg))
        .ok_or(Error::CursorOutOfRange(actor.motion_leg))?;
    actor.position.y = actor.position.y.wrapping_add(u16::from(step));
    actor.motion_leg = actor.motion_leg.wrapping_add(1);
    if actor.motion_leg != SWAY_STEPS {
        return Ok(Effect::Moved);
    }
    actor.motion_leg = 0;
    let cursor = actor.motion_counter();
    match actor.state {
        1 | 2 => {
            actor.state += 2;
            actor.set_motion_counter(cursor.wrapping_sub(1));
        }
        3 => {
            actor.state = 2;
            actor.set_motion_counter(cursor.wrapping_add(1));
        }
        _ => {
            actor.state = 1;
            actor.set_motion_counter(cursor.wrapping_add(1));
        }
    }
    Ok(Effect::Moved)
}
