//! Native pipe bobber domain behavior, ROM `$584CB` (downward, anim 7/66) and `$584E0` (upward, anim 12).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FrameTick};

/// Direction in which the pipe bobber emerges from its pipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergenceDirection {
    /// Upward emergence (ROM `$584E0`, anim 12): emerges up out of floor pipe, sinks back down.
    Up,
    /// Downward emergence (ROM `$584CB`, anim 7/66): emerges down out of ceiling pipe, rises back up.
    Down,
}

/// Pipe bobber state machine ($584CB / $584E0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeBobberState {
    Waiting = 0,
    Emerging = 1,
    Retracting = 2,
    Resting = 3,
}

impl PipeBobberState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Emerging,
            2 => Self::Retracting,
            3 => Self::Resting,
            _ => Self::Waiting,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeBobberEffect {
    Waiting,
    Emerging,
    Retracting,
    Resting,
    Despawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipeBobberContext {
    pub frame: FrameTick,
    pub mario_travelling: bool,
    pub mario_ref_x: u16,
}

/// Computes the ROM `$5B4B4` horizontal distance between Mario and the actor.
#[inline]
pub fn dist_mario_x(mario_ref_x: u16, actor_x: u16) -> u8 {
    let d = (mario_ref_x as u8).wrapping_sub(actor_x as u8);
    if d & 0x80 != 0 {
        d.wrapping_neg()
    } else {
        d
    }
}

/// Domain update for Pipe Bobber (ROM $584CB / $584E0, anims 7, 12, 66).
pub fn update(
    actor: &mut Actor,
    context: PipeBobberContext,
    direction: EmergenceDirection,
) -> PipeBobberEffect {
    // ROM $584CB / $584E0: pipe travel check.
    if context.mario_travelling {
        return PipeBobberEffect::Despawn;
    }

    let state = PipeBobberState::from_raw(actor.state);
    match state {
        PipeBobberState::Waiting => {
            // $584F3: defeat mode $80 (invulnerable/inactive) while waiting in pipe.
            actor.defeat_mode = 0x80;
            if dist_mario_x(context.mario_ref_x, actor.position.x) < 0x18 {
                return PipeBobberEffect::Waiting;
            }
            actor.state = PipeBobberState::Emerging.to_raw();
            actor.motion_leg = 32;
            actor.set_travel_span(32);
            actor.contact_kind = ContactKind::from_raw(0x44);
            PipeBobberEffect::Emerging
        }
        PipeBobberState::Emerging => {
            // $5850C (Up) or $5851C (Down): sprite toggle every 4 frames.
            actor.animate_two_frame_toggle(context.frame, 4);
            match direction {
                EmergenceDirection::Up => actor.position.move_y(-1),
                EmergenceDirection::Down => actor.position.move_y(1),
            }
            actor.motion_leg = actor.motion_leg.wrapping_sub(1);
            if actor.motion_leg != 0 {
                return PipeBobberEffect::Emerging;
            }
            // $5852A: reload.
            actor.motion_leg = actor.travel_span();
            actor.state = PipeBobberState::Retracting.to_raw();
            actor.timer = 24;
            PipeBobberEffect::Retracting
        }
        PipeBobberState::Retracting => {
            // $5851C (Up) or $5850C (Down): sprite toggle every 4 frames.
            actor.animate_two_frame_toggle(context.frame, 4);
            match direction {
                EmergenceDirection::Up => actor.position.move_y(1),
                EmergenceDirection::Down => actor.position.move_y(-1),
            }
            actor.motion_leg = actor.motion_leg.wrapping_sub(1);
            if actor.motion_leg != 0 {
                return PipeBobberEffect::Retracting;
            }
            // $5852A: reload.
            actor.motion_leg = actor.travel_span();
            actor.state = PipeBobberState::Resting.to_raw();
            actor.timer = 24;
            PipeBobberEffect::Resting
        }
        PipeBobberState::Resting => {
            // $58537: countdown timer inside pipe.
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return PipeBobberEffect::Resting;
            }
            actor.state = PipeBobberState::Waiting.to_raw();
            PipeBobberEffect::Waiting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dist_mario_x_exact_calculations() {
        assert_eq!(dist_mario_x(100, 80), 20);
        assert_eq!(dist_mario_x(80, 100), 20);
        assert_eq!(dist_mario_x(0, 0), 0);
        assert_eq!(dist_mario_x(10, 24), 14);
        assert_eq!(dist_mario_x(24, 10), 14);
        // Wrapping difference: 250 and 10 have difference 16.
        assert_eq!(dist_mario_x(250, 10), 16);
        assert_eq!(dist_mario_x(10, 250), 16);
    }

    #[test]
    fn test_waiting_close_mario_remains_waiting() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        let ctx = PipeBobberContext {
            frame: FrameTick::new(0),
            mario_travelling: false,
            mario_ref_x: 110, // distance 10 < 24
        };
        let effect = update(&mut actor, ctx, EmergenceDirection::Up);
        assert_eq!(effect, PipeBobberEffect::Waiting);
        assert_eq!(actor.state, PipeBobberState::Waiting.to_raw());
        assert_eq!(actor.defeat_mode, 0x80);
    }

    #[test]
    fn test_waiting_far_mario_triggers_emergence() {
        let mut actor = Actor::default();
        actor.position.x = 100;
        let ctx = PipeBobberContext {
            frame: FrameTick::new(0),
            mario_travelling: false,
            mario_ref_x: 124, // distance 24 >= 24
        };
        let effect = update(&mut actor, ctx, EmergenceDirection::Up);
        assert_eq!(effect, PipeBobberEffect::Emerging);
        assert_eq!(actor.state, PipeBobberState::Emerging.to_raw());
        assert_eq!(actor.motion_leg, 32);
        assert_eq!(actor.travel_span(), 32);
        assert_eq!(actor.contact_kind.raw(), 0x44);
        assert_eq!(actor.defeat_mode, 0x80);
    }

    #[test]
    fn test_emerging_and_retracting_upward() {
        let mut actor = Actor::default();
        actor.position.y = 200;
        actor.state = PipeBobberState::Emerging.to_raw();
        actor.motion_leg = 32;
        actor.set_travel_span(32);
        actor.sprite = 0x10;

        // Step 1 through 31: emerges upward (y decreases)
        for i in 1..=31 {
            let ctx = PipeBobberContext {
                frame: FrameTick::new(i),
                mario_travelling: false,
                mario_ref_x: 500,
            };
            let effect = update(&mut actor, ctx, EmergenceDirection::Up);
            assert_eq!(effect, PipeBobberEffect::Emerging);
            assert_eq!(actor.position.y, 200 - i as u16);
            assert_eq!(actor.motion_leg, (32 - i) as u8);
            if i % 4 == 0 {
                let expected_bit = if (i / 4) % 2 == 1 { 1 } else { 0 };
                assert_eq!(actor.sprite & 1, expected_bit);
            }
        }

        // Step 32: reaches top, reloads for retraction
        let ctx = PipeBobberContext {
            frame: FrameTick::new(32),
            mario_travelling: false,
            mario_ref_x: 500,
        };
        let effect = update(&mut actor, ctx, EmergenceDirection::Up);
        assert_eq!(effect, PipeBobberEffect::Retracting);
        assert_eq!(actor.state, PipeBobberState::Retracting.to_raw());
        assert_eq!(actor.position.y, 200 - 32);
        assert_eq!(actor.motion_leg, 32);
        assert_eq!(actor.timer, 24);

        // Retract 31 steps (y increases)
        for i in 1..=31 {
            let ctx = PipeBobberContext {
                frame: FrameTick::new(32 + i),
                mario_travelling: false,
                mario_ref_x: 500,
            };
            let effect = update(&mut actor, ctx, EmergenceDirection::Up);
            assert_eq!(effect, PipeBobberEffect::Retracting);
            assert_eq!(actor.position.y, 200 - 32 + i as u16);
            assert_eq!(actor.motion_leg, (32 - i) as u8);
        }

        // Retraction step 32: reaches bottom, transitions to Resting
        let ctx = PipeBobberContext {
            frame: FrameTick::new(64),
            mario_travelling: false,
            mario_ref_x: 500,
        };
        let effect = update(&mut actor, ctx, EmergenceDirection::Up);
        assert_eq!(effect, PipeBobberEffect::Resting);
        assert_eq!(actor.state, PipeBobberState::Resting.to_raw());
        assert_eq!(actor.position.y, 200); // Back at exact starting y!
        assert_eq!(actor.motion_leg, 32);
        assert_eq!(actor.timer, 24);

        // Rest 23 frames
        for i in 1..=23 {
            let ctx = PipeBobberContext {
                frame: FrameTick::new(64 + i),
                mario_travelling: false,
                mario_ref_x: 500,
            };
            let effect = update(&mut actor, ctx, EmergenceDirection::Up);
            assert_eq!(effect, PipeBobberEffect::Resting);
            assert_eq!(actor.timer, (24 - i) as u8);
        }

        // Rest frame 24: transitions back to Waiting (state 0)
        let ctx = PipeBobberContext {
            frame: FrameTick::new(88),
            mario_travelling: false,
            mario_ref_x: 500,
        };
        let effect = update(&mut actor, ctx, EmergenceDirection::Up);
        assert_eq!(effect, PipeBobberEffect::Waiting);
        assert_eq!(actor.state, PipeBobberState::Waiting.to_raw());
    }

    #[test]
    fn test_emerging_and_retracting_downward() {
        let mut actor = Actor::default();
        actor.position.y = 100;
        actor.state = PipeBobberState::Emerging.to_raw();
        actor.motion_leg = 32;
        actor.set_travel_span(32);

        // Step 1: emerges downward (y increases)
        let ctx = PipeBobberContext {
            frame: FrameTick::new(1),
            mario_travelling: false,
            mario_ref_x: 500,
        };
        let effect = update(&mut actor, ctx, EmergenceDirection::Down);
        assert_eq!(effect, PipeBobberEffect::Emerging);
        assert_eq!(actor.position.y, 101);

        // Fast forward 31 remaining steps
        for i in 2..=32 {
            let ctx = PipeBobberContext {
                frame: FrameTick::new(i),
                mario_travelling: false,
                mario_ref_x: 500,
            };
            update(&mut actor, ctx, EmergenceDirection::Down);
        }
        assert_eq!(actor.state, PipeBobberState::Retracting.to_raw());
        assert_eq!(actor.position.y, 132);

        // Retract 32 steps (y decreases back toward ceiling)
        for i in 1..=32 {
            let ctx = PipeBobberContext {
                frame: FrameTick::new(32 + i),
                mario_travelling: false,
                mario_ref_x: 500,
            };
            update(&mut actor, ctx, EmergenceDirection::Down);
        }
        assert_eq!(actor.state, PipeBobberState::Resting.to_raw());
        assert_eq!(actor.position.y, 100); // Back at exact starting y!
    }

    #[test]
    fn test_pipe_travel_despawn() {
        let mut actor = Actor::default();
        actor.state = PipeBobberState::Emerging.to_raw();
        let ctx = PipeBobberContext {
            frame: FrameTick::new(5),
            mario_travelling: true,
            mario_ref_x: 500,
        };
        let effect = update(&mut actor, ctx, EmergenceDirection::Up);
        assert_eq!(effect, PipeBobberEffect::Despawn);
    }
}
