//! Native vertical patrol, ROM `$591F9` and its motion helpers.

use super::staging::{BOUNCE_SPEED, FALL_SPEED};
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick, MotionProfileError};
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub(super) struct BobberContext {
    pub frame: FrameTick,
    pub player_ref_y: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum BobberEffect {
    None,
    PlayerRelation(Ordering),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum BobberError {
    UnknownState(u8),
    MotionProfile(MotionProfileError),
}

pub(super) fn update(
    actor: &mut Actor,
    context: BobberContext,
) -> Result<BobberEffect, BobberError> {
    actor.animate_two_frame_toggle(context.frame, 2);
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(86);
        actor.flash = FlashFlags::from_bits(16);
        let relation = context.player_ref_y.cmp(&actor.position.y);
        actor.state = if relation == Ordering::Greater { 2 } else { 1 };
        return Ok(BobberEffect::PlayerRelation(relation));
    }
    if !context.frame.every(4) {
        return Ok(BobberEffect::None);
    }
    let (profile, direction, next_state): (&[u8], i16, u8) = match actor.state {
        1 => (&BOUNCE_SPEED, -1, 3),
        2 => (&BOUNCE_SPEED, 1, 4),
        3 => (&FALL_SPEED, 1, 2),
        4 => (&FALL_SPEED, -1, 1),
        state => return Err(BobberError::UnknownState(state)),
    };
    if matches!(actor.state, 3 | 4) && actor.timer != 0 {
        actor.timer -= 1;
        return Ok(BobberEffect::None);
    }
    let distance = actor
        .motion_step(profile, 15)
        .map_err(BobberError::MotionProfile)?;
    actor.position.move_y(direction * i16::from(distance));
    if actor.motion_phase == 15 {
        actor.motion_phase = 0;
        actor.state = next_state;
    }
    Ok(BobberEffect::None)
}

#[cfg(test)]
#[path = "vertical_bobber_tests.rs"]
mod tests;
