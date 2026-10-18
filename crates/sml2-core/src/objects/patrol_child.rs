//! Boss anim 15, ROM `$A029` (bank 2 CPU `$6029`).
use crate::actor::{
    Actor, ContactKind, FrameTick, MotionProfileError, RiseProfile, ScreenPosition,
};
use std::cmp::Ordering;

pub(super) struct Inputs {
    pub frame: FrameTick,
    pub player: ScreenPosition,
    pub remove_mode: u8,
    pub reward_slot: bool,
    pub hit_processed: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Effects {
    pub target_relation: Option<Ordering>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum UpdateError {
    InvalidState(u8),
    Motion(MotionProfileError),
}

pub(super) fn update(actor: &mut Actor, input: Inputs) -> Result<Effects, UpdateError> {
    let mut effects = Effects::default();
    if input.remove_mode == 2 {
        if input.reward_slot {
            super::boss_reward::morph(actor, ScreenPosition { x: 88, y: 112 });
        }
        return Ok(effects);
    }
    if input.hit_processed {
        return Ok(effects);
    }
    match actor.state {
        0 => {
            if actor.sprite != 14 {
                actor.state = 3;
                actor.contact_kind = ContactKind::from_raw(69);
            }
        }
        3 => {
            let step = actor
                .rise_step(RiseProfile::Low)
                .map_err(UpdateError::Motion)?;
            // `$A064` adds this profile, despite the shared helper's rise name.
            actor.screen.move_y(i16::from(step));
            if actor.motion_phase == 15 {
                actor.motion_phase = 0;
                actor.sprite = 50;
                effects.target_relation = Some(input.player.x.cmp(&actor.screen.x));
                actor.state = if actor.face_toward_screen(input.player.x).is_right() {
                    2
                } else {
                    1
                };
                actor.set_travel_span(32);
            }
        }
        1 | 2 => {
            actor.animate_two_frame_toggle(input.frame, 4);
            if !input.frame.every(4) {
                return Ok(effects);
            }
            actor.screen.move_x(if actor.state == 1 { -1 } else { 1 });
            if actor.screen.x >= 184 {
                actor.sprite = 14;
                actor.state = 0;
            } else {
                let relation = input.player.y.cmp(&actor.screen.y);
                effects.target_relation = Some(relation);
                actor
                    .screen
                    .move_y(if relation == Ordering::Less { -1 } else { 1 });
            }
        }
        state => return Err(UpdateError::InvalidState(state)),
    }
    Ok(effects)
}

#[cfg(test)]
#[path = "patrol_child_tests.rs"]
mod tests;
