//! ROM `$59487` (anim 67): a vertical bob between two timed rests.
//!
//! `$5B5DF` cycles the sprite by the `$7603` deltas every fourth frame. State 0
//! arms contact 77, defeat mode 128 and a 128-frame rest. State 1 then climbs
//! on the `$5B481` curve (`BOUNCE_SPEED`, 15 steps) while `+24` accumulates the
//! distance; phase 15 resets the curve and starts a 48-frame rest. State 2
//! descends one pixel every even frame, draining `+24`; at zero it starts a
//! 64-frame rest and climbs again.
use super::staging::BOUNCE_SPEED;
use crate::actor::{Actor, AnimationFrameError, ContactKind, FrameTick, MotionProfileError};

// ROM `$7603`: four sprite deltas for `$5B5DF`.
const SPRITE_DELTAS: [i8; 4] = [-3, 1, 1, 1];

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

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<(), Error> {
    // $5B5DF: runs before the state gate, every fourth frame.
    actor.animate_sprite_delta_cycle(frame, 4, &SPRITE_DELTAS)?;
    match actor.state {
        // $5948F: arm the bob.
        0 => {
            actor.state = 1;
            actor.contact_kind = ContactKind::from_raw(77);
            actor.defeat_mode = 128;
            actor.timer = 128;
        }
        // $594A4: rest, then climb on the bounce curve.
        1 => {
            if actor.timer != 0 {
                actor.timer -= 1;
            } else {
                let dy = actor.motion_step(&BOUNCE_SPEED, 15)?;
                actor.position.move_y(-i16::from(dy));
                actor.motion_leg = actor.motion_leg.wrapping_add(dy);
                if actor.motion_phase == 15 {
                    actor.motion_phase = 0;
                    actor.timer = 48;
                    actor.state = 2;
                }
            }
        }
        // $594C9: rest, then descend one pixel every even frame.
        2 => {
            if actor.timer != 0 {
                actor.timer -= 1;
            } else if frame.every(2) {
                actor.position.move_y(1);
                actor.motion_leg = actor.motion_leg.wrapping_sub(1);
                if actor.motion_leg == 0 {
                    actor.timer = 64;
                    actor.state = 1;
                }
            }
        }
        state => return Err(Error::InvalidState(state)),
    }
    Ok(())
}
