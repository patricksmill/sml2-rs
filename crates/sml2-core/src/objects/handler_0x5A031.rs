//! ROM `$5A031` (mode B anim 36): a side-wall bouncer.
//!
//! Entry cycles the sprite (`$5B5B6`, deltas -2/+1/+1). State 0 arms contact 72,
//! picks the walk direction from flip bit 5 and snaps 16 px sideways. States 1/2
//! walk one pixel per frame into their leading wall (removing on contact) and run
//! the vertical oscillator: `+24 == 0` rises on the `$74AB` eight-step curve and
//! toggles at phase 8; otherwise it falls on the `$7488` curve (also capped at 8)
//! until the own-Y ground probe lands it.
use super::staging::FALL_SPEED;
use crate::actor::{Actor, AnimationFrameError, ContactKind, FrameTick, MotionProfileError};
use crate::actor::DEFEAT_RISE_STEPS;

const CONTACT: u8 = 72;
/// ROM `$75D3`: the three sprite deltas for `$5B5B6`.
const SPRITE_DELTAS: [i8; 3] = [-2, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Animation(AnimationFrameError),
    Motion(MotionProfileError),
}
impl From<AnimationFrameError> for Error {
    fn from(error: AnimationFrameError) -> Self {
        Self::Animation(error)
    }
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
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
    fn probe_ground(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<Effect, Error> {
    // $5B5B6: sprite cycle before the state dispatch, on even frames.
    actor.animate_sprite_delta_cycle(frame, 2, &SPRITE_DELTAS)?;
    match actor.state {
        // $5A039: contact 72, $3CB1 direction, then a 16-px side snap.
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            if actor.sprite_flip.bits() != 0 {
                actor.position.move_x(16);
            } else {
                actor.position.move_x(-16);
            }
            Ok(Effect::Continue)
        }
        // $5A055: left wall ends the object; otherwise step left and oscillate.
        1 => {
            if world.probe_left_wall(actor) {
                return Ok(Effect::Remove);
            }
            actor.position.move_x(-1);
            vertical(actor, world)
        }
        // $5A06F: right wall ends the object; otherwise step right and oscillate.
        2 => {
            if world.probe_right_wall(actor) {
                return Ok(Effect::Remove);
            }
            actor.position.move_x(1);
            vertical(actor, world)
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// `$5A07E`: rise/fly toggle. `+24 == 0` is the rise leg; the fall leg ends on
/// the own-Y ground probe. Both reset `+16` and flip `+24` on completion.
fn vertical(actor: &mut Actor, world: &mut impl World) -> Result<Effect, Error> {
    if actor.motion_leg == 0 {
        let step = actor.motion_step(&DEFEAT_RISE_STEPS, 8)?;
        actor.position.move_y(-i16::from(step));
        if actor.motion_phase != 8 {
            return Ok(Effect::Continue);
        }
    } else {
        if !world.probe_ground(actor) {
            let step = actor.motion_step(&FALL_SPEED, 8)?;
            actor.position.move_y(i16::from(step));
            return Ok(Effect::Continue);
        }
    }
    actor.motion_phase = 0;
    actor.motion_leg ^= 1;
    Ok(Effect::Continue)
}
