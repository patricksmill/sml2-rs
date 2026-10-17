//! Native Wario trajectory child (anim 27, ROM `$0AFE0`).

use crate::actor::{Actor, ContactKind, FrameTick};

/// Horizontal trajectory displacement profile (`$B039..$B07D`, 69 bytes).
/// Byte 68 is sentinel `0x80` (128) which resets phase to 58.
pub const HORIZONTAL_PROFILE: [u8; 69] = [
    0x01, 0x02, 0x02, 0x02, 0x03, 0x03, 0x01, 0x01, 0x03, 0x03, 0x02, 0x02, 0x02, 0x01, 0x01, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x02, 0x02, 0x02, 0x01, 0x01, 0x01, 0x01, 0x02,
    0x02, 0x02, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x02, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x00, 0x80,
];

/// Vertical trajectory signed displacement profile (`$B07E..$B0C1`, 68 bytes).
pub const VERTICAL_PROFILE: [i8; 68] = [
    -4, -4, -3, -2, -2, 1, 0, 0, 1, 2, 2,
    3, 4, 4, 4, 4, 4, 4, -4, -3, -3, -3, -3, -3, -2, -2, -1,
    0, 0, 0, 0, 1, 2, 2, 3, 3, 3, 3, 3, 4, -3, -3, -3,
    -2, -2, -2, -1, 0, 0, 0, 0, 1, 2, 2, 2, 3, 3, 3, -3,
    -2, -2, -1, 0, 0, 1, 2, 2, 3,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrajectoryChildEffects {
    None,
    CountedRemove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrajectoryChildError {
    UnsupportedState(u8),
    InvalidPhase(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct TrajectoryChildContext {
    pub frame: FrameTick,
}

pub fn update_trajectory_child(
    actor: &mut Actor,
    context: TrajectoryChildContext,
) -> Result<TrajectoryChildEffects, TrajectoryChildError> {
    actor.animate_four_frame_cycle(context.frame, 2);
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::WARIO_CHILD;
            actor.state = if actor.sprite_flip.horizontal() { 2 } else { 1 };
            actor.sprite = 104;
            actor.animation_frame = 0;
            actor.screen.y = actor.screen.y.wrapping_sub(16);
            Ok(TrajectoryChildEffects::None)
        }
        1 | 2 => {
            let phase = actor.motion_phase as usize;
            if phase >= HORIZONTAL_PROFILE.len() {
                return Err(TrajectoryChildError::InvalidPhase(actor.motion_phase));
            }
            let dx = HORIZONTAL_PROFILE[phase];
            if dx == 0x80 {
                actor.motion_phase = 58;
                return Ok(TrajectoryChildEffects::None);
            }
            if actor.state == 1 {
                actor.screen.x = actor.screen.x.wrapping_sub(dx);
            } else {
                actor.screen.x = actor.screen.x.wrapping_add(dx);
            }
            if actor.screen.x >= 176 {
                return Ok(TrajectoryChildEffects::CountedRemove);
            }
            let dy = VERTICAL_PROFILE[phase];
            actor.screen.y = actor.screen.y.wrapping_add(dy as u8);
            actor.motion_phase = actor.motion_phase.wrapping_add(1);
            Ok(TrajectoryChildEffects::None)
        }
        other => Err(TrajectoryChildError::UnsupportedState(other)),
    }
}
