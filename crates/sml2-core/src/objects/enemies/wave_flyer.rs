//! Native wave flyer enemy domain behavior, ROM `$59F5E` (anim 85 Mode B, levels 16, 17, 23).
//!
//! Undulating/wave flyer: moves horizontally at 2 px per step while climbing/diving
//! vertically at 1 px per step on an exact 4-frame cadence ($FFF0 & 3 == 0).
//! Animates low sprite bit on the same 4-frame toggle cadence ($5B51A).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

pub const STEP_BUDGET: u8 = 48; // ROM $59F71 / $59F8F: ld a, 48
pub const FLASH_FLAGS: u8 = 0x10; // ROM $59F66: ld a, 16
pub const CONTACT_KIND: u8 = 72; // ROM $59F6A: ld a, 72 (0x48)
pub const HORIZONTAL_STEP_PX: i16 = 2; // ROM $59F83 / $59FA3: ld b, 2
pub const VERTICAL_STEP_PX: i16 = 1; // ROM $59FAA: ld b, 1

/// Wave flyer horizontal state ($59F64 / $59F7B: states 0..=2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveFlyerState {
    Init = 0,
    FlyLeft = 1,
    FlyRight = 2,
}

impl WaveFlyerState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::FlyLeft,
            2 => Self::FlyRight,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

/// Wave flyer vertical flight phase ($FFE8 / slot[24]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalDirection {
    Climbing = 0,
    Diving = 1,
}

impl VerticalDirection {
    pub const fn from_raw(value: u8) -> Self {
        if value == 0 {
            Self::Climbing
        } else {
            Self::Diving
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct WaveFlyerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveFlyerEffect {
    CadenceSkipped,
    Initialized {
        facing: u8,
    },
    Moved {
        state: WaveFlyerState,
        vertical: VerticalDirection,
        budget: u8,
    },
    ClimbCompleted,
    TurnedAround {
        state: WaveFlyerState,
    },
}

/// Domain update for WaveFlyer (ROM $59F5E).
pub fn update(actor: &mut Actor, context: WaveFlyerContext) -> WaveFlyerEffect {
    // ROM $59F5E: call $5B51A (sprite bit 0 toggle on 4-frame cadence)
    actor.animate_two_frame_toggle(context.frame, 4);

    let state = WaveFlyerState::from_raw(actor.state);
    if state == WaveFlyerState::Init {
        actor.flash = FlashFlags::from_bits(FLASH_FLAGS);
        actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);

        // ROM $5B3C6: face toward Mario
        let (next_state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
            std::cmp::Ordering::Greater => (WaveFlyerState::FlyRight, true, 1),
            std::cmp::Ordering::Less => (WaveFlyerState::FlyLeft, false, 0xFF),
            std::cmp::Ordering::Equal => (WaveFlyerState::FlyLeft, false, 0),
        };
        actor.state = next_state.to_raw();
        actor.sprite_flip.set_horizontal(flip);
        actor.set_travel_span(STEP_BUDGET);
        return WaveFlyerEffect::Initialized { facing };
    }

    // ROM $59F76: ldh a, [$FFF0] ; and $03 ; ret nz
    if !context.frame.every(4) {
        return WaveFlyerEffect::CadenceSkipped;
    }

    // Horizontal movement: state 1 moves X by -2, state 2 moves X by +2
    match state {
        WaveFlyerState::FlyLeft => {
            actor.position.move_x(-HORIZONTAL_STEP_PX);
        }
        WaveFlyerState::FlyRight => {
            actor.position.move_x(HORIZONTAL_STEP_PX);
        }
        WaveFlyerState::Init => unreachable!(),
    }

    // ROM $59F88: ldh a, [$FFE9] ; sub b (b=2)
    let remaining = actor.travel_span().wrapping_sub(2);
    if remaining != 0 {
        actor.set_travel_span(remaining);
        let vertical = VerticalDirection::from_raw(actor.motion_leg);
        match vertical {
            VerticalDirection::Climbing => {
                actor.position.move_y(-VERTICAL_STEP_PX);
            }
            VerticalDirection::Diving => {
                actor.position.move_y(VERTICAL_STEP_PX);
            }
        }
        WaveFlyerEffect::Moved {
            state,
            vertical,
            budget: remaining,
        }
    } else {
        // Budget exhausted: reload 48
        actor.set_travel_span(STEP_BUDGET);
        let vertical = VerticalDirection::from_raw(actor.motion_leg);
        match vertical {
            VerticalDirection::Climbing => {
                // Apex reached: transition climbing (0) -> diving (1)
                actor.motion_leg = VerticalDirection::Diving.to_raw();
                WaveFlyerEffect::ClimbCompleted
            }
            VerticalDirection::Diving => {
                // Trough reached: transition diving (1) -> climbing (0), toggle facing and state
                actor.motion_leg = VerticalDirection::Climbing.to_raw();
                actor.sprite_flip.toggle_horizontal();
                let next_state = match state {
                    WaveFlyerState::FlyLeft => WaveFlyerState::FlyRight,
                    WaveFlyerState::FlyRight => WaveFlyerState::FlyLeft,
                    WaveFlyerState::Init => WaveFlyerState::FlyLeft,
                };
                actor.state = next_state.to_raw();
                WaveFlyerEffect::TurnedAround { state: next_state }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{Position, ScreenPosition};

    fn make_test_actor(x: u16, y: u16) -> Actor {
        let mut actor = Actor::default();
        actor.position = Position { x, y };
        actor.screen = ScreenPosition { x: 80, y: 72 };
        actor.sprite = 150;
        actor
    }

    #[test]
    fn test_init_facing_left_and_right() {
        // Mario to left
        let mut actor = make_test_actor(100, 80);
        let ctx = WaveFlyerContext {
            frame: FrameTick::new(0),
            mario_ref_x: 60,
        };
        let effect = update(&mut actor, ctx);
        assert_eq!(effect, WaveFlyerEffect::Initialized { facing: 0xFF });
        assert_eq!(actor.state, WaveFlyerState::FlyLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.flash.bits(), FLASH_FLAGS);
        assert_eq!(actor.contact_kind.raw(), CONTACT_KIND);
        assert_eq!(actor.travel_span(), STEP_BUDGET);
        assert_eq!(actor.motion_leg, VerticalDirection::Climbing.to_raw());

        // Mario to right
        let mut actor = make_test_actor(100, 80);
        let ctx = WaveFlyerContext {
            frame: FrameTick::new(0),
            mario_ref_x: 140,
        };
        let effect = update(&mut actor, ctx);
        assert_eq!(effect, WaveFlyerEffect::Initialized { facing: 1 });
        assert_eq!(actor.state, WaveFlyerState::FlyRight.to_raw());
        assert!(actor.sprite_flip.horizontal());

        // Mario at same X
        let mut actor = make_test_actor(100, 80);
        let ctx = WaveFlyerContext {
            frame: FrameTick::new(0),
            mario_ref_x: 100,
        };
        let effect = update(&mut actor, ctx);
        assert_eq!(effect, WaveFlyerEffect::Initialized { facing: 0 });
        assert_eq!(actor.state, WaveFlyerState::FlyLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
    }

    #[test]
    fn test_sprite_toggle_cadence() {
        let mut actor = make_test_actor(100, 80);
        actor.sprite = 150;
        actor.state = WaveFlyerState::FlyRight.to_raw();

        // Frame 0: toggles
        update(
            &mut actor,
            WaveFlyerContext {
                frame: FrameTick::new(0),
                mario_ref_x: 100,
            },
        );
        assert_eq!(actor.sprite, 151);

        // Frame 1, 2, 3: no toggle
        for f in 1..=3 {
            update(
                &mut actor,
                WaveFlyerContext {
                    frame: FrameTick::new(f),
                    mario_ref_x: 100,
                },
            );
            assert_eq!(actor.sprite, 151);
        }

        // Frame 4: toggles
        update(
            &mut actor,
            WaveFlyerContext {
                frame: FrameTick::new(4),
                mario_ref_x: 100,
            },
        );
        assert_eq!(actor.sprite, 150);
    }

    #[test]
    fn test_cadence_skips_movement_on_non_multiples_of_4() {
        let mut actor = make_test_actor(100, 80);
        actor.state = WaveFlyerState::FlyRight.to_raw();
        actor.set_travel_span(48);

        for f in [1, 2, 3, 5, 6, 7, 9, 10, 11] {
            let effect = update(
                &mut actor,
                WaveFlyerContext {
                    frame: FrameTick::new(f),
                    mario_ref_x: 100,
                },
            );
            assert_eq!(effect, WaveFlyerEffect::CadenceSkipped);
            assert_eq!(actor.position.x, 100);
            assert_eq!(actor.position.y, 80);
            assert_eq!(actor.travel_span(), 48);
        }
    }

    #[test]
    fn test_full_wave_flight_cycle() {
        let mut actor = make_test_actor(100, 100);
        actor.state = WaveFlyerState::FlyRight.to_raw();
        actor.sprite_flip.set_horizontal(true);
        actor.set_travel_span(48);
        actor.motion_leg = VerticalDirection::Climbing.to_raw();

        // Step 1 (frame 4): X += 2, Y -= 1, budget = 46
        let eff = update(
            &mut actor,
            WaveFlyerContext {
                frame: FrameTick::new(4),
                mario_ref_x: 100,
            },
        );
        assert_eq!(
            eff,
            WaveFlyerEffect::Moved {
                state: WaveFlyerState::FlyRight,
                vertical: VerticalDirection::Climbing,
                budget: 46,
            }
        );
        assert_eq!(actor.position.x, 102);
        assert_eq!(actor.position.y, 99);

        // Steps 2..=23: 22 more climbing steps
        for step in 2..=23 {
            let f = step * 4;
            let eff = update(
                &mut actor,
                WaveFlyerContext {
                    frame: FrameTick::new(f),
                    mario_ref_x: 100,
                },
            );
            assert!(matches!(eff, WaveFlyerEffect::Moved { .. }));
        }
        assert_eq!(actor.position.x, 100 + 23 * 2); // 146
        assert_eq!(actor.position.y, 100 - 23); // 77
        assert_eq!(actor.travel_span(), 2);

        // Step 24 (frame 96): Apex reached!
        // X moves +2 to 148, budget hits 0 and resets to 48, motion_leg becomes Diving.
        // Y does NOT move (remains 77).
        let eff = update(
            &mut actor,
            WaveFlyerContext {
                frame: FrameTick::new(96),
                mario_ref_x: 100,
            },
        );
        assert_eq!(eff, WaveFlyerEffect::ClimbCompleted);
        assert_eq!(actor.position.x, 148);
        assert_eq!(actor.position.y, 77);
        assert_eq!(actor.travel_span(), 48);
        assert_eq!(actor.motion_leg, VerticalDirection::Diving.to_raw());

        // Step 25 (diving step 1, frame 100): X += 2 to 150, Y += 1 to 78, budget = 46
        let eff = update(
            &mut actor,
            WaveFlyerContext {
                frame: FrameTick::new(100),
                mario_ref_x: 100,
            },
        );
        assert_eq!(
            eff,
            WaveFlyerEffect::Moved {
                state: WaveFlyerState::FlyRight,
                vertical: VerticalDirection::Diving,
                budget: 46,
            }
        );
        assert_eq!(actor.position.x, 150);
        assert_eq!(actor.position.y, 78);

        // Steps 26..=47: 22 more diving steps
        for step in 26..=47 {
            let f = step * 4;
            let eff = update(
                &mut actor,
                WaveFlyerContext {
                    frame: FrameTick::new(f),
                    mario_ref_x: 100,
                },
            );
            assert!(matches!(eff, WaveFlyerEffect::Moved { .. }));
        }
        assert_eq!(actor.position.x, 148 + 23 * 2); // 194
        assert_eq!(actor.position.y, 77 + 23); // 100
        assert_eq!(actor.travel_span(), 2);

        // Step 48 (frame 192): Trough reached & turnaround!
        // X moves +2 to 196 (total 96 px right!).
        // Budget hits 0, resets to 48.
        // motion_leg flips to Climbing.
        // state flips to FlyLeft.
        // sprite_flip flips to false.
        // Y does NOT move (remains 100).
        let eff = update(
            &mut actor,
            WaveFlyerContext {
                frame: FrameTick::new(192),
                mario_ref_x: 100,
            },
        );
        assert_eq!(
            eff,
            WaveFlyerEffect::TurnedAround {
                state: WaveFlyerState::FlyLeft
            }
        );
        assert_eq!(actor.position.x, 196);
        assert_eq!(actor.position.y, 100);
        assert_eq!(actor.travel_span(), 48);
        assert_eq!(actor.motion_leg, VerticalDirection::Climbing.to_raw());
        assert_eq!(actor.state, WaveFlyerState::FlyLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());

        // Step 49 (climbing left, frame 196): X -= 2 to 194, Y -= 1 to 99, budget = 46
        let eff = update(
            &mut actor,
            WaveFlyerContext {
                frame: FrameTick::new(196),
                mario_ref_x: 100,
            },
        );
        assert_eq!(
            eff,
            WaveFlyerEffect::Moved {
                state: WaveFlyerState::FlyLeft,
                vertical: VerticalDirection::Climbing,
                budget: 46,
            }
        );
        assert_eq!(actor.position.x, 194);
        assert_eq!(actor.position.y, 99);
    }

    #[test]
    fn test_raw_slot_roundtrip() {
        let mut slot = [0u8; 32];
        slot[0] = 0x01; // X hi
        slot[1] = 0x00; // X lo (256)
        slot[3] = 0x00; // Y hi
        slot[4] = 0x50; // Y lo (80)
        slot[11] = 140; // sprite
        slot[15] = 0; // state = Init

        let mut actor = Actor::from_raw_slot(&slot);
        let ctx = WaveFlyerContext {
            frame: FrameTick::new(0),
            mario_ref_x: 300,
        };
        let eff = update(&mut actor, ctx);
        assert_eq!(eff, WaveFlyerEffect::Initialized { facing: 1 });

        let out_slot = actor.to_raw_slot();
        assert_eq!(out_slot[15], WaveFlyerState::FlyRight.to_raw());
        assert_eq!(out_slot[13] & 0x20, 0x20); // flip_h
        assert_eq!(out_slot[14], FLASH_FLAGS);
        assert_eq!(out_slot[18], CONTACT_KIND);
        assert_eq!(out_slot[25], STEP_BUDGET);
    }
}
