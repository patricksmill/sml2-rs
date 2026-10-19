//! Boss anim 18, ROM `$A2BE` (bank 2): drop, approach, rise and dissolve.
use crate::actor::{Actor, ContactKind, FrameTick, MotionProfileError, ScreenPosition};

pub(super) struct Inputs<'a> {
    pub frame: FrameTick,
    pub player_x: u8,
    pub remove_mode: u8,
    pub drop_profile: &'a [u8],
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum UpdateError {
    State(u8),
    Motion(MotionProfileError),
}

pub(super) fn update(actor: &mut Actor, input: Inputs<'_>) -> Result<Option<u8>, UpdateError> {
    if input.remove_mode == 2 {
        super::boss_reward::morph(actor, ScreenPosition { x: 88, y: 128 });
        return Ok(None);
    }
    match actor.state {
        0 => {
            if actor.sprite != 14 {
                actor.state = 3;
                actor.contact_kind = ContactKind::from_raw(64);
            }
        }
        1 | 2 => {
            if input.frame.every(2) {
                actor.screen.move_x(if actor.state == 1 { -3 } else { 3 });
                if actor.screen.horizontal_distance_to(input.player_x) < 16 {
                    actor.state = 4;
                }
            }
        }
        3 => {
            let step = actor
                .motion_step(input.drop_profile, 21)
                .map_err(UpdateError::Motion)?;
            actor.screen.move_y(i16::from(step));
            if actor.screen.y >= 128 {
                actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            }
        }
        4 => {
            if input.frame.every(4) {
                actor.sprite = actor.sprite.wrapping_add(1);
                if actor.sprite == 62 {
                    actor.state = 5;
                    actor.motion_leg = 48;
                }
            }
        }
        5 => {
            actor.animate_two_frame_toggle(input.frame, 4);
            actor.screen.move_y(-2);
            actor.motion_leg = actor.motion_leg.wrapping_sub(2);
            if actor.motion_leg == 0 {
                actor.state = 6;
                actor.timer = 6;
                return Ok(Some(13));
            }
        }
        6 => {
            if input.frame.every(4) {
                if actor.sprite != 65 {
                    actor.sprite = actor.sprite.wrapping_add(1);
                } else {
                    actor.timer = actor.timer.wrapping_sub(1);
                    if actor.timer == 0 {
                        actor.state = 0;
                        actor.contact_kind = ContactKind::NONE;
                        actor.sprite = 14;
                    }
                }
            }
        }
        state => return Err(UpdateError::State(state)),
    }
    Ok(None)
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
