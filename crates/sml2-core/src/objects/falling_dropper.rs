//! Native Falling Dropper domain behavior, ROM `$595B2` (anim 78 Mode A).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind};
use super::pipe_bobber::dist_mario_x;
use super::staging::FALL_SPEED;

/// Falling Dropper state machine (ROM $595B2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallingDropperState {
    Waiting = 0,
    Wobbling = 1,
    Falling = 2,
}

impl FallingDropperState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Wobbling,
            2 => Self::Falling,
            _ => Self::Waiting,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallingDropperEffect {
    Waiting,
    TriggerDrop { sfx: u8 },
    Wobbling,
    Falling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallingDropperContext {
    pub mario_ref_x: u16,
}

/// Fall-step helper for ROM `$5B451`: retrieves speed from `FALL_SPEED` (cap 21),
/// increments `motion_phase` up to 21, and returns speed.
#[inline]
pub fn step_fall_speed(actor: &mut Actor) -> u8 {
    let idx = (actor.motion_phase as usize).min(21);
    let speed = FALL_SPEED[idx];
    if actor.motion_phase != 21 {
        actor.motion_phase = actor.motion_phase.wrapping_add(1);
    }
    speed
}

/// Domain update for Falling Dropper (ROM $595B2, anim 78).
pub fn update(
    actor: &mut Actor,
    context: FallingDropperContext,
) -> FallingDropperEffect {
    let state = FallingDropperState::from_raw(actor.state);
    match state {
        FallingDropperState::Waiting => {
            // $595B7: contact kind = 72 (0x48)
            actor.contact_kind = ContactKind::from_raw(72);
            // $595BB: proximity check (distance < 56 px / 0x38)
            let dist = dist_mario_x(context.mario_ref_x, actor.position.x);
            if dist >= 0x38 {
                return FallingDropperEffect::Waiting;
            }

            // $595C1..$595CB: transition to Wobbling, 16-frame countdown, SFX 21
            actor.state = FallingDropperState::Wobbling.to_raw();
            actor.timer = 16;
            FallingDropperEffect::TriggerDrop { sfx: 21 }
        }
        FallingDropperState::Wobbling => {
            // $595D7: decrement timer
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                // $595F0: transition to Falling
                actor.state = FallingDropperState::Falling.to_raw();
                return FallingDropperEffect::Falling;
            }

            // $595E0..$595EC: wobble left/right by 3 pixels
            actor.travel_direction.reverse();
            if actor.travel_direction.is_right() {
                actor.position.move_x(3);
            } else {
                actor.position.move_x(-3);
            }
            FallingDropperEffect::Wobbling
        }
        FallingDropperState::Falling => {
            // $595F5..$595FA: fall along 2x FALL_SPEED curve
            let speed = step_fall_speed(actor);
            actor.position.move_y((speed as i16) * 2);
            FallingDropperEffect::Falling
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waiting_idle_when_mario_far() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        actor.position.y = 80;
        actor.state = FallingDropperState::Waiting.to_raw();

        // Mario far away: dx = 60 >= 56
        let eff = update(&mut actor, FallingDropperContext { mario_ref_x: 260 });
        assert_eq!(eff, FallingDropperEffect::Waiting);
        assert_eq!(actor.state, FallingDropperState::Waiting.to_raw());
        assert_eq!(actor.contact_kind.raw(), 72);
        assert_eq!(actor.position.x, 200);
        assert_eq!(actor.position.y, 80);
    }

    #[test]
    fn test_proximity_trigger_transitions_to_wobble() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        actor.position.y = 80;
        actor.state = FallingDropperState::Waiting.to_raw();

        // Mario triggers within 55 px (< 56)
        let eff = update(&mut actor, FallingDropperContext { mario_ref_x: 255 });
        assert_eq!(eff, FallingDropperEffect::TriggerDrop { sfx: 21 });
        assert_eq!(actor.state, FallingDropperState::Wobbling.to_raw());
        assert_eq!(actor.timer, 16);
    }

    #[test]
    fn test_wobble_oscillates_x_and_transitions_to_fall() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        actor.position.y = 80;
        actor.state = FallingDropperState::Wobbling.to_raw();
        actor.timer = 16;

        // Frame 1: timer becomes 15, travel_direction 0 -> 1 (+3 X: 203)
        let eff1 = update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        assert_eq!(eff1, FallingDropperEffect::Wobbling);
        assert_eq!(actor.timer, 15);
        assert_eq!(actor.position.x, 203);

        // Frame 2: timer becomes 14, travel_direction 1 -> 0 (-3 X: 200)
        let eff2 = update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        assert_eq!(eff2, FallingDropperEffect::Wobbling);
        assert_eq!(actor.timer, 14);
        assert_eq!(actor.position.x, 200);

        // Advance 13 more frames (frames 3..=15, timer 13 down to 1)
        for _ in 3..=15 {
            update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        }
        assert_eq!(actor.timer, 1);
        assert_eq!(actor.position.x, 203, "after odd number of wobbles (15), x is 203");
        assert_eq!(actor.state, FallingDropperState::Wobbling.to_raw());

        // Frame 16: timer becomes 0 -> transitions to Falling (no X shift)
        let eff16 = update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        assert_eq!(eff16, FallingDropperEffect::Falling);
        assert_eq!(actor.state, FallingDropperState::Falling.to_raw());
        assert_eq!(actor.position.x, 203, "no X change on transition frame");
    }

    #[test]
    fn test_falling_acceleration_curve() {
        let mut actor = Actor::default();
        actor.position.x = 200;
        actor.position.y = 80;
        actor.state = FallingDropperState::Falling.to_raw();
        actor.motion_phase = 0;

        // Phase 0: FALL_SPEED[0] = 0 -> dy = 0, phase becomes 1
        update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        assert_eq!(actor.position.y, 80);
        assert_eq!(actor.motion_phase, 1);

        // Phase 1: FALL_SPEED[1] = 0 -> dy = 0, phase becomes 2
        update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        assert_eq!(actor.position.y, 80);
        assert_eq!(actor.motion_phase, 2);

        // Phase 2: FALL_SPEED[2] = 1 -> dy = 2, phase becomes 3
        update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        assert_eq!(actor.position.y, 82);
        assert_eq!(actor.motion_phase, 3);

        // Phase 3: FALL_SPEED[3] = 1 -> dy = 2, phase becomes 4
        update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        assert_eq!(actor.position.y, 84);
        assert_eq!(actor.motion_phase, 4);

        // Fast forward through phase 21 cap
        for _ in 4..30 {
            update(&mut actor, FallingDropperContext { mario_ref_x: 200 });
        }
        assert_eq!(actor.motion_phase, 21, "capped at phase 21");
    }
}
