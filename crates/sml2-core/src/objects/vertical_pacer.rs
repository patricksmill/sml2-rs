//! Native vertical pacer enemy domain behavior, ROM `$59E06` (anim 95 Mode A, anim 7 Mode B).
//!
//! Paces vertically: moves 48 pixels up on even frames, pauses for 16 even frames,
//! then moves 48 pixels down on even frames, pauses for 16 even frames, and repeats.
//! Animates low sprite bit on 4-frame cadence ($5B51A).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

pub const TRAVEL_BUDGET_PX: u8 = 48; // ROM $59E1A / $59E40: ld a, 48
pub const PAUSE_FRAMES: u8 = 16;     // ROM $59E0E / $59E44: ld a, 16
pub const CONTACT_KIND: u8 = 76;     // ROM $59E12: ld a, 76 (0x4C)

/// Vertical pacer movement state ($59E09 / $5E2C / $5E4B: states 0..=2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalPacerState {
    Init = 0,
    MoveUp = 1,
    MoveDown = 2,
}

impl VerticalPacerState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::MoveUp,
            2 => Self::MoveDown,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct VerticalPacerContext {
    pub frame: FrameTick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalPacerEffect {
    None,
    Initialized,
    Moved { state: VerticalPacerState },
    Turned { state: VerticalPacerState },
}

/// Domain update for VerticalPacer (ROM $59E06).
pub fn update(
    actor: &mut Actor,
    context: VerticalPacerContext,
) -> VerticalPacerEffect {
    // ROM $5B51A: toggle low bit of sprite every fourth frame
    actor.animate_two_frame_toggle(context.frame, 4);

    let state = VerticalPacerState::from_raw(actor.state);
    match state {
        VerticalPacerState::Init => {
            actor.flash = FlashFlags::from_bits(0x10);
            actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);
            actor.state = VerticalPacerState::MoveUp.to_raw();
            actor.motion_leg = TRAVEL_BUDGET_PX;
            VerticalPacerEffect::Initialized
        }
        VerticalPacerState::MoveUp => {
            // ROM $59E1F..$59E23: odd-frame gate (moves on even frames only)
            if context.frame.0 & 1 != 0 {
                return VerticalPacerEffect::None;
            }
            if actor.timer != 0 {
                actor.timer = actor.timer.wrapping_sub(1);
                VerticalPacerEffect::None
            } else {
                actor.position.move_y(-1);
                actor.motion_leg = actor.motion_leg.wrapping_sub(1);
                if actor.motion_leg == 0 {
                    actor.motion_leg = TRAVEL_BUDGET_PX;
                    actor.timer = PAUSE_FRAMES;
                    actor.state = VerticalPacerState::MoveDown.to_raw();
                    VerticalPacerEffect::Turned {
                        state: VerticalPacerState::MoveDown,
                    }
                } else {
                    VerticalPacerEffect::Moved {
                        state: VerticalPacerState::MoveUp,
                    }
                }
            }
        }
        VerticalPacerState::MoveDown => {
            // ROM $59E1F..$59E23: odd-frame gate (moves on even frames only)
            if context.frame.0 & 1 != 0 {
                return VerticalPacerEffect::None;
            }
            if actor.timer != 0 {
                actor.timer = actor.timer.wrapping_sub(1);
                VerticalPacerEffect::None
            } else {
                actor.position.move_y(1);
                actor.motion_leg = actor.motion_leg.wrapping_sub(1);
                if actor.motion_leg == 0 {
                    actor.motion_leg = TRAVEL_BUDGET_PX;
                    actor.timer = PAUSE_FRAMES;
                    actor.state = VerticalPacerState::MoveUp.to_raw();
                    VerticalPacerEffect::Turned {
                        state: VerticalPacerState::MoveUp,
                    }
                } else {
                    VerticalPacerEffect::Moved {
                        state: VerticalPacerState::MoveDown,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_pacer_init_sets_flash_type_and_budget() {
        let mut actor = Actor::default();
        actor.position.y = 200;

        let eff = update(&mut actor, VerticalPacerContext { frame: FrameTick(1) });
        assert_eq!(eff, VerticalPacerEffect::Initialized);
        assert_eq!(actor.state, VerticalPacerState::MoveUp.to_raw());
        assert_eq!(actor.flash.bits(), 0x10);
        assert_eq!(actor.contact_kind, ContactKind::from_raw(CONTACT_KIND));
        assert_eq!(actor.motion_leg, 48);
        assert_eq!(actor.timer, 0);
    }

    #[test]
    fn vertical_pacer_odd_frame_gate_skips_movement() {
        let mut actor = Actor::default();
        actor.position.y = 200;
        update(&mut actor, VerticalPacerContext { frame: FrameTick(1) });

        // Odd frame 1 -> skipped
        let eff = update(&mut actor, VerticalPacerContext { frame: FrameTick(1) });
        assert_eq!(eff, VerticalPacerEffect::None);
        assert_eq!(actor.position.y, 200);
        assert_eq!(actor.motion_leg, 48);

        // Even frame 2 -> steps Y
        let eff = update(&mut actor, VerticalPacerContext { frame: FrameTick(2) });
        assert_eq!(eff, VerticalPacerEffect::Moved { state: VerticalPacerState::MoveUp });
        assert_eq!(actor.position.y, 199);
        assert_eq!(actor.motion_leg, 47);
    }

    #[test]
    fn vertical_pacer_sprite_toggle_cadence() {
        let mut actor = Actor::default();
        actor.sprite = 20;

        // Frame 0 -> toggles
        update(&mut actor, VerticalPacerContext { frame: FrameTick(0) });
        assert_eq!(actor.sprite, 21);

        // Frame 1 -> no toggle
        update(&mut actor, VerticalPacerContext { frame: FrameTick(1) });
        assert_eq!(actor.sprite, 21);

        // Frame 4 -> toggles back
        update(&mut actor, VerticalPacerContext { frame: FrameTick(4) });
        assert_eq!(actor.sprite, 20);
    }

    #[test]
    fn vertical_pacer_full_movement_and_pause_cycle() {
        let mut actor = Actor::default();
        actor.position.y = 200;

        // Init
        update(&mut actor, VerticalPacerContext { frame: FrameTick(1) });

        // Move Up for 47 steps
        for step in 1..=47 {
            let frame = u64::from(step) * 2; // even frames
            let eff = update(&mut actor, VerticalPacerContext { frame: FrameTick(frame) });
            assert_eq!(eff, VerticalPacerEffect::Moved { state: VerticalPacerState::MoveUp });
            assert_eq!(actor.position.y, 200 - step);
            assert_eq!(actor.motion_leg, 48 - step as u8);
        }

        // 48th step: reaches 0, reloads budget to 48, sets pause timer = 16, state = MoveDown
        let eff = update(&mut actor, VerticalPacerContext { frame: FrameTick(96) });
        assert_eq!(eff, VerticalPacerEffect::Turned { state: VerticalPacerState::MoveDown });
        assert_eq!(actor.position.y, 200 - 48);
        assert_eq!(actor.motion_leg, 48);
        assert_eq!(actor.timer, 16);
        assert_eq!(actor.state, VerticalPacerState::MoveDown.to_raw());

        // 16 pause steps on even frames: timer decrements 16 -> 0
        for p in 1u64..=16 {
            let frame = 96 + p * 2;
            let eff = update(&mut actor, VerticalPacerContext { frame: FrameTick(frame) });
            assert_eq!(eff, VerticalPacerEffect::None);
            assert_eq!(actor.position.y, 200 - 48);
            assert_eq!(actor.timer, (16 - p) as u8);
        }

        // Now timer is 0: next even frame begins moving Down
        let eff = update(&mut actor, VerticalPacerContext { frame: FrameTick(130) });
        assert_eq!(eff, VerticalPacerEffect::Moved { state: VerticalPacerState::MoveDown });
        assert_eq!(actor.position.y, 200 - 48 + 1);
        assert_eq!(actor.motion_leg, 47);
    }
}
