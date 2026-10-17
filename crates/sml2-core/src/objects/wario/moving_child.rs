//! Wario's moving child, shared by boss animations 21 and 24.

use crate::actor::{Actor, ContactKind, Facing, FrameTick, MotionProfileError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ChildState {
    Initialize,
    Patrol,
    Falling,
    Respawning,
}

impl TryFrom<u8> for ChildState {
    type Error = ChildError;

    fn try_from(state: u8) -> Result<Self, Self::Error> {
        match state {
            0 => Ok(Self::Initialize),
            1 => Ok(Self::Patrol),
            2 => Ok(Self::Falling),
            3 => Ok(Self::Respawning),
            other => Err(ChildError::UnsupportedState(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RespawnEdge {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ChildContext {
    pub frame: FrameTick,
    pub transition_pending: bool,
    pub respawn_edge: RespawnEdge,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ChildEffects {
    pub consume_transition: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChildError {
    UnsupportedState(u8),
    InvalidFallProfile(MotionProfileError),
}

pub(super) fn update_child(
    actor: &mut Actor,
    context: ChildContext,
    fall_profile: &[u8],
) -> Result<ChildEffects, ChildError> {
    let mut effects = ChildEffects::default();
    match ChildState::try_from(actor.state)? {
        ChildState::Initialize => actor.state = ChildState::Patrol as u8,
        ChildState::Patrol => {
            if context.frame.every(4) {
                actor.move_screen_horizontally(3);
                if actor.screen.x >= 176 {
                    actor.travel_direction.reverse();
                }
            }
            if context.transition_pending {
                actor.state = ChildState::Falling as u8;
                actor.contact_kind = ContactKind::WARIO_CHILD;
                effects.consume_transition = true;
            }
        }
        ChildState::Falling => {
            actor.flash.animate_palette(context.frame);
            let distance = actor
                .motion_step(fall_profile, 36)
                .map_err(ChildError::InvalidFallProfile)?;
            actor.screen.move_y(i16::from(distance));
            if actor.motion_phase == 36 {
                actor.flash.clear();
                actor.motion_phase = 0;
                actor.state = ChildState::Respawning as u8;
            }
        }
        ChildState::Respawning if context.frame.every(8) => {
            actor.sprite = actor.sprite.wrapping_add(1);
            if actor.sprite == 99 {
                actor.screen.y = 56;
                actor.sprite = 96;
                actor.contact_kind = ContactKind::NONE;
                actor.state = ChildState::Initialize as u8;
                actor.travel_direction = Facing::LEFT;
                actor.screen.x = match context.respawn_edge {
                    RespawnEdge::Left => {
                        actor.travel_direction = Facing::RIGHT;
                        0
                    }
                    RespawnEdge::Right => 167,
                };
            }
        }
        ChildState::Respawning => {}
    }
    Ok(effects)
}

// Preserve the existing byte-oriented fixtures at the compatibility boundary.
#[cfg(test)]
#[path = "moving_child_native_tests.rs"]
mod native_tests;
#[cfg(test)]
#[path = "moving_child_tests.rs"]
mod tests;
