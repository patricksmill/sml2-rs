//! Native horizontal moving platform domain behavior, ROM `$58C6A` (anim 42 Mode A, levels 5, 9).
//!
//! Horizontally oscillating platform:
//! - Initialized with contact kind $20 (solid on-object support).
//! - Starts moving toward or away from Mario for 36 pixels (half-span).
//! - Then oscillates indefinitely back and forth across a 72-pixel full span (centered on spawn).
//! - Carries Mario horizontally when stood on (via $03CC0 / $03CF1).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind};

pub const HALF_SPAN_PX: u8 = 36; // ROM $58C76: ld a, 36
#[allow(dead_code)]
pub const FULL_SPAN_PX: u8 = 72; // ROM $58CC3: add a (36 * 2 = 72)
pub const CONTACT_KIND: u8 = 32; // ROM $58C6F: ld a, 32 (0x20)

/// Horizontal platform state ($58C6A / $58C7B: states 0..=4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalPlatformState {
    Init = 0,
    InitialLeft = 1,
    InitialRight = 2,
    FullLeft = 3,
    FullRight = 4,
}

impl HorizontalPlatformState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::InitialLeft,
            2 => Self::InitialRight,
            3 => Self::FullLeft,
            4 => Self::FullRight,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct HorizontalPlatformContext {
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalPlatformEffect {
    Initialized {
        facing: u8,
    },
    Moved {
        state: HorizontalPlatformState,
        carry_dx: Option<i16>,
    },
    Turned {
        state: HorizontalPlatformState,
        carry_dx: Option<i16>,
    },
}

/// Domain update for HorizontalPlatform (ROM $58C6A).
pub fn update(actor: &mut Actor, context: HorizontalPlatformContext) -> HorizontalPlatformEffect {
    let state = HorizontalPlatformState::from_raw(actor.state);
    if state == HorizontalPlatformState::Init {
        // ROM $58C6F: ld a, 32 ; ldh [$FFE2], a (contact_kind = 0x20)
        actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);

        // ROM $5B3C6: face toward Mario
        let (next_state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
            std::cmp::Ordering::Greater => (HorizontalPlatformState::InitialRight, true, 1),
            std::cmp::Ordering::Less => (HorizontalPlatformState::InitialLeft, false, 0xFF),
            std::cmp::Ordering::Equal => (HorizontalPlatformState::InitialLeft, false, 0),
        };
        actor.state = next_state.to_raw();
        actor.sprite_flip.set_horizontal(flip);

        // ROM $58C76: ld a, 36 ; ldh [$FFE8], a
        actor.motion_leg = HALF_SPAN_PX;
        actor.motion_phase = 0;
        return HorizontalPlatformEffect::Initialized { facing };
    }

    let stood_on = actor.interactions.stood_on();

    match state {
        HorizontalPlatformState::Init => unreachable!(),
        HorizontalPlatformState::InitialLeft => {
            // ROM $58C85: ld b, 1 ; X -= 1 ; carry left if stood_on
            actor.position.move_x(-1);
            let carry_dx = if stood_on { Some(-1) } else { None };

            actor.motion_phase = actor.motion_phase.wrapping_add(1);
            if actor.motion_phase >= actor.motion_leg {
                actor.motion_phase = 0;
                actor.state = HorizontalPlatformState::FullRight.to_raw();
                HorizontalPlatformEffect::Turned {
                    state: HorizontalPlatformState::FullRight,
                    carry_dx,
                }
            } else {
                HorizontalPlatformEffect::Moved {
                    state: HorizontalPlatformState::InitialLeft,
                    carry_dx,
                }
            }
        }
        HorizontalPlatformState::InitialRight => {
            // ROM $58C9F: ld b, 1 ; X += 1 ; carry right if stood_on
            actor.position.move_x(1);
            let carry_dx = if stood_on { Some(1) } else { None };

            actor.motion_phase = actor.motion_phase.wrapping_add(1);
            if actor.motion_phase >= actor.motion_leg {
                actor.motion_phase = 0;
                actor.state = HorizontalPlatformState::FullLeft.to_raw();
                HorizontalPlatformEffect::Turned {
                    state: HorizontalPlatformState::FullLeft,
                    carry_dx,
                }
            } else {
                HorizontalPlatformEffect::Moved {
                    state: HorizontalPlatformState::InitialRight,
                    carry_dx,
                }
            }
        }
        HorizontalPlatformState::FullLeft => {
            // ROM $58CB9: ld b, 1 ; X -= 1 ; carry left if stood_on
            actor.position.move_x(-1);
            let carry_dx = if stood_on { Some(-1) } else { None };

            actor.motion_phase = actor.motion_phase.wrapping_add(1);
            let threshold = actor.motion_leg.wrapping_mul(2);
            if actor.motion_phase >= threshold {
                actor.motion_phase = 0;
                // ROM $58CD1: xor $07 (3 ^ 7 = 4)
                actor.state = HorizontalPlatformState::FullRight.to_raw();
                HorizontalPlatformEffect::Turned {
                    state: HorizontalPlatformState::FullRight,
                    carry_dx,
                }
            } else {
                HorizontalPlatformEffect::Moved {
                    state: HorizontalPlatformState::FullLeft,
                    carry_dx,
                }
            }
        }
        HorizontalPlatformState::FullRight => {
            // ROM $58CD6: ld b, 1 ; X += 1 ; carry right if stood_on
            actor.position.move_x(1);
            let carry_dx = if stood_on { Some(1) } else { None };

            actor.motion_phase = actor.motion_phase.wrapping_add(1);
            let threshold = actor.motion_leg.wrapping_mul(2);
            if actor.motion_phase >= threshold {
                actor.motion_phase = 0;
                // ROM $58CD1: xor $07 (4 ^ 7 = 3)
                actor.state = HorizontalPlatformState::FullLeft.to_raw();
                HorizontalPlatformEffect::Turned {
                    state: HorizontalPlatformState::FullLeft,
                    carry_dx,
                }
            } else {
                HorizontalPlatformEffect::Moved {
                    state: HorizontalPlatformState::FullRight,
                    carry_dx,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{InteractionFlags, Position, ScreenPosition};

    fn make_test_actor(x: u16, y: u16) -> Actor {
        let mut actor = Actor::default();
        actor.position = Position { x, y };
        actor.screen = ScreenPosition { x: 80, y: 72 };
        actor
    }

    #[test]
    fn test_init_facing_left_and_right() {
        // Mario to left
        let mut actor = make_test_actor(100, 80);
        let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 60 });
        assert_eq!(eff, HorizontalPlatformEffect::Initialized { facing: 0xFF });
        assert_eq!(actor.state, HorizontalPlatformState::InitialLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.contact_kind.raw(), CONTACT_KIND);
        assert_eq!(actor.motion_leg, HALF_SPAN_PX);
        assert_eq!(actor.motion_phase, 0);

        // Mario to right
        let mut actor = make_test_actor(100, 80);
        let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 140 });
        assert_eq!(eff, HorizontalPlatformEffect::Initialized { facing: 1 });
        assert_eq!(actor.state, HorizontalPlatformState::InitialRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.motion_leg, HALF_SPAN_PX);
    }

    #[test]
    fn test_initial_left_movement_and_turn_to_full_right() {
        let mut actor = make_test_actor(100, 80);
        actor.state = HorizontalPlatformState::InitialLeft.to_raw();
        actor.motion_leg = HALF_SPAN_PX;

        for step in 1..=35 {
            let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 0 });
            assert_eq!(
                eff,
                HorizontalPlatformEffect::Moved {
                    state: HorizontalPlatformState::InitialLeft,
                    carry_dx: None,
                }
            );
            assert_eq!(actor.position.x, 100 - step);
            assert_eq!(actor.motion_phase, step as u8);
        }

        // 36th step: reaches boundary (100 - 36 = 64), turns to FullRight
        let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 0 });
        assert_eq!(
            eff,
            HorizontalPlatformEffect::Turned {
                state: HorizontalPlatformState::FullRight,
                carry_dx: None,
            }
        );
        assert_eq!(actor.position.x, 64);
        assert_eq!(actor.motion_phase, 0);
        assert_eq!(actor.state, HorizontalPlatformState::FullRight.to_raw());
    }

    #[test]
    fn test_full_right_and_full_left_oscillation() {
        assert_eq!(FULL_SPAN_PX, 72);
        assert_eq!(HALF_SPAN_PX * 2, FULL_SPAN_PX);

        let mut actor = make_test_actor(64, 80);
        actor.state = HorizontalPlatformState::FullRight.to_raw();
        actor.motion_leg = HALF_SPAN_PX;

        // Move 71 steps right
        for step in 1..=71 {
            let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 0 });
            assert_eq!(
                eff,
                HorizontalPlatformEffect::Moved {
                    state: HorizontalPlatformState::FullRight,
                    carry_dx: None,
                }
            );
            assert_eq!(actor.position.x, 64 + step);
            assert_eq!(actor.motion_phase, step as u8);
        }

        // 72nd step: reaches right boundary (64 + 72 = 136), turns to FullLeft
        let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 0 });
        assert_eq!(
            eff,
            HorizontalPlatformEffect::Turned {
                state: HorizontalPlatformState::FullLeft,
                carry_dx: None,
            }
        );
        assert_eq!(actor.position.x, 136);
        assert_eq!(actor.motion_phase, 0);
        assert_eq!(actor.state, HorizontalPlatformState::FullLeft.to_raw());

        // Move 71 steps left
        for step in 1..=71 {
            let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 0 });
            assert_eq!(
                eff,
                HorizontalPlatformEffect::Moved {
                    state: HorizontalPlatformState::FullLeft,
                    carry_dx: None,
                }
            );
            assert_eq!(actor.position.x, 136 - step);
        }

        // 72nd step left: reaches left boundary (136 - 72 = 64), turns to FullRight
        let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 0 });
        assert_eq!(
            eff,
            HorizontalPlatformEffect::Turned {
                state: HorizontalPlatformState::FullRight,
                carry_dx: None,
            }
        );
        assert_eq!(actor.position.x, 64);
        assert_eq!(actor.motion_phase, 0);
        assert_eq!(actor.state, HorizontalPlatformState::FullRight.to_raw());
    }

    #[test]
    fn test_mario_carry_when_stood_on() {
        let mut actor = make_test_actor(100, 80);
        actor.state = HorizontalPlatformState::FullRight.to_raw();
        actor.motion_leg = HALF_SPAN_PX;
        actor.interactions = InteractionFlags::from_bits(0x08); // stood_on (bit 3)

        let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 0 });
        assert_eq!(
            eff,
            HorizontalPlatformEffect::Moved {
                state: HorizontalPlatformState::FullRight,
                carry_dx: Some(1),
            }
        );

        actor.state = HorizontalPlatformState::FullLeft.to_raw();
        let eff = update(&mut actor, HorizontalPlatformContext { mario_ref_x: 0 });
        assert_eq!(
            eff,
            HorizontalPlatformEffect::Moved {
                state: HorizontalPlatformState::FullLeft,
                carry_dx: Some(-1),
            }
        );
    }
}
