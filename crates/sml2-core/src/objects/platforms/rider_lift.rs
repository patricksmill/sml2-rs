//! ROM `$591A3` (anim 55): a rider-controlled lift.
//!
//! State 0 toggles the sprite every eighth frame, arms contact 32 and flash 16,
//! and starts state 1 when Mario stands on it: `+24`/`+25` = 96, sprite 98,
//! sfx 22. State 1 rises one pixel per call while ridden (`$58E38`), stopping at
//! `+24 == 0`; stepping off switches to state 2 and a 16-frame wait. State 2
//! then descends one pixel per call until `+24 == +25` (`$58E57`), and stepping
//! back on returns to state 1. `$5B5B6` cycles the sprite on states 1/2.
use crate::actor::{Actor, AnimationFrameError, ContactKind, FlashFlags, FrameTick};

/// ROM `$75D3`: the three sprite deltas for `$5B5B6`.
const SPRITE_DELTAS: [i8; 3] = [-2, 1, 1];
const LIFT_HEIGHT: u8 = 96;
const STEP_OFF_WAIT: u8 = 16;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Animation(AnimationFrameError),
}
impl From<AnimationFrameError> for Error {
    fn from(error: AnimationFrameError) -> Self {
        Self::Animation(error)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Waiting,
    Started,
    Rising { carry_dy: Option<i16> },
    StepOffWait,
    WaitingToDescend,
    Descending,
}

pub(super) trait World {
    fn request_sfx1(&mut self, id: u8);
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<Effect, Error> {
    match actor.state {
        // $591A8: arm the lift and wait for a rider.
        0 => {
            actor.animate_two_frame_toggle(frame, 8);
            actor.contact_kind = ContactKind::from_raw(32);
            actor.flash = FlashFlags::from_bits(16);
            if !actor.interactions.stood_on() {
                return Ok(Effect::Waiting);
            }
            actor.state = 1;
            actor.motion_leg = LIFT_HEIGHT;
            actor.set_travel_span(LIFT_HEIGHT);
            actor.sprite = 98;
            world.request_sfx1(22);
            Ok(Effect::Started)
        }
        // $591D2 -> $58E38: rise while ridden, or wait 16 frames after step-off.
        1 => {
            actor.animate_sprite_delta_cycle(frame, 2, &SPRITE_DELTAS)?;
            if !actor.interactions.stood_on() {
                actor.state = 2;
                actor.timer = STEP_OFF_WAIT;
                return Ok(Effect::StepOffWait);
            }
            if actor.motion_leg == 0 {
                return Ok(Effect::Rising { carry_dy: None });
            }
            actor.motion_leg -= 1;
            actor.position.move_y(-1);
            Ok(Effect::Rising { carry_dy: Some(-1) })
        }
        // $591D8 -> $58E57: resume on a rider, else wait then descend.
        2 => {
            actor.animate_sprite_delta_cycle(frame, 2, &SPRITE_DELTAS)?;
            if actor.interactions.stood_on() {
                actor.state = 1;
                actor.timer = 0;
                return Ok(Effect::Started);
            }
            if actor.timer != 0 {
                actor.timer -= 1;
                if actor.timer != 0 {
                    return Ok(Effect::WaitingToDescend);
                }
            }
            if actor.motion_leg == actor.travel_span() {
                return Ok(Effect::Descending);
            }
            actor.motion_leg = actor.motion_leg.wrapping_add(1);
            actor.position.move_y(1);
            // $58E72 calls only the object mover ($3D3D). Mario has stepped
            // off; unlike the rise path, this never calls a rider mover.
            Ok(Effect::Descending)
        }
        state => Err(Error::InvalidState(state)),
    }
}
