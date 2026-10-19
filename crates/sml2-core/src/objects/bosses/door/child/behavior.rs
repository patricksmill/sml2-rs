//! Door boss child, anim 9 (ROM `$9927`, bank 2 CPU `$5927`).
use crate::actor::{Actor, ContactKind, FrameTick};

#[path = "profiles.rs"]
mod profiles;

pub(super) struct Inputs {
    pub frame: FrameTick,
    pub door_phase: u8,
    pub sequence_active: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Effects {
    pub activate_sequence: bool,
    pub counted_remove: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum UpdateError {
    InvalidState(u8),
    InvalidPhase(u8),
}

pub(super) fn update(actor: &mut Actor, input: Inputs) -> Result<Effects, UpdateError> {
    actor.animate_two_frame_toggle(input.frame, 4);
    let mut effects = Effects::default();
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::WARIO_CHILD;
            actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            actor.screen.move_y(-28);
            // Init tests the entire flip byte; subsequent movement uses state.
            actor.screen.move_x(if actor.sprite_flip.bits() == 0 {
                -16
            } else {
                16
            });
        }
        1 | 2 => {
            let phase = usize::from(actor.motion_phase);
            let (horizontal, vertical) = profiles::for_door_phase(input.door_phase);
            let dx = *horizontal
                .get(phase)
                .ok_or(UpdateError::InvalidPhase(actor.motion_phase))?;
            if dx == 0x80 {
                actor.sprite = actor.sprite.wrapping_add(2);
                actor.state = 3;
                actor.timer = 32;
            } else {
                // For phase 0 (and non-1/2/3 values), `$99A2` returns
                // the same distance selected by the horizontal profile.
                let dy = match vertical {
                    Some(profile) => *profile
                        .get(phase)
                        .ok_or(UpdateError::InvalidPhase(actor.motion_phase))?,
                    None => dx,
                };
                actor.screen.move_x(if actor.state == 1 {
                    -i16::from(dx)
                } else {
                    i16::from(dx)
                });
                actor.screen.move_y(i16::from(dy));
                actor.motion_phase = actor.motion_phase.wrapping_add(1);
            }
        }
        3 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                if input.door_phase != 0 {
                    effects.activate_sequence = true;
                    actor.state = 4;
                } else {
                    effects.counted_remove = true;
                }
            }
        }
        4 => effects.counted_remove = !input.sequence_active,
        state => return Err(UpdateError::InvalidState(state)),
    }
    Ok(effects)
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
