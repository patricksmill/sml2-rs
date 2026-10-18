//! Boss anim 8, ROM `$98C8` (bank 2 CPU `$58C8`).
use crate::actor::{Actor, ContactKind, FrameTick, MotionProfileError};

pub(super) struct Inputs<'a> {
    pub remove_mode: u8,
    pub sequence_active: bool,
    pub door_phase: u8,
    pub frame: FrameTick,
    pub fall_profile: &'a [u8],
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Effects {
    pub carry_dy: Option<i16>,
    pub clear_sequence: bool,
    pub sound: Option<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum UpdateError {
    InvalidState(u8),
    Motion(MotionProfileError),
}

pub(super) fn update(actor: &mut Actor, input: Inputs<'_>) -> Result<Effects, UpdateError> {
    let effects = Effects::default();
    // This override precedes every state, including the waiting state.
    if input.remove_mode == 2 {
        actor.screen.y = 116;
        actor.screen.x = 88;
        actor.definition = 4;
        actor.sprite = 9;
        actor.state = 0;
        actor.motion_phase = 0;
        actor.contact_kind = ContactKind::NONE;
        return Ok(effects);
    }
    if actor.state == 0 {
        if input.sequence_active && input.door_phase == 2 {
            actor.state = 1;
            actor.motion_leg = 64;
            actor.timer = 64;
            actor.sprite = 25;
        }
        return Ok(effects);
    }
    update_motion(
        actor,
        input.remove_mode,
        input.frame,
        input.fall_profile,
        116,
    )
}

/// Shared door states `$986C/$9883/$98A4` and the two sill-height tails.
pub(super) fn update_motion(
    actor: &mut Actor,
    remove_mode: u8,
    frame: FrameTick,
    fall_profile: &[u8],
    sill: u8,
) -> Result<Effects, UpdateError> {
    let mut effects = Effects::default();
    match actor.state {
        1 => {
            if frame.every(4) {
                actor.sprite_flip.toggle_horizontal();
            }
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.state = 2;
                actor.sprite = 19;
                effects.sound = Some(5);
            }
        }
        2 => {
            if remove_mode != 0 {
                actor.contact_kind = ContactKind::NONE;
            } else {
                actor.screen.move_y(-2);
                if actor.interactions.stood_on() {
                    effects.carry_dy = Some(-2);
                }
                actor.motion_leg = actor.motion_leg.wrapping_sub(2);
                if actor.motion_leg == 0 {
                    actor.state = 3;
                    actor.timer = 32;
                }
            }
        }
        3 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                effects.clear_sequence = true;
                actor.state = 4;
            }
        }
        4 => {
            if actor.screen.y >= sill {
                actor.screen.y = sill;
                actor.state = 0;
            } else {
                let step = actor
                    .motion_step(fall_profile, 21)
                    .map_err(UpdateError::Motion)?;
                actor.screen.move_y(i16::from(step));
            }
        }
        state => return Err(UpdateError::InvalidState(state)),
    }
    Ok(effects)
}

#[cfg(test)]
#[path = "central_door_tests.rs"]
mod tests;
