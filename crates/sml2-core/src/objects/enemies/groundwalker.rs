//! Native groundwalker enemy domain behavior, ROM `$59BC5` (anim 88).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

/// Groundwalker movement state ($5BD1 / $5D05 / $5D41: states 0..=2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundWalkerState {
    Init = 0,
    WalkLeft = 1,
    WalkRight = 2,
}

impl GroundWalkerState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::WalkLeft,
            2 => Self::WalkRight,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct GroundWalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundWalkerEffect {
    None,
    Initialized { facing: u8 },
    Turned { state: GroundWalkerState },
    Moved { state: GroundWalkerState },
    Paused,
}

pub trait GroundWalkerWorld {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
}

/// Domain update for GroundWalker (ROM $59BC5, anim 88).
pub fn update<W>(
    actor: &mut Actor,
    context: GroundWalkerContext,
    world: &mut W,
) -> GroundWalkerEffect
where
    W: GroundWalkerWorld,
{
    // $5B585: four-step bob at period 2
    let _ = actor.animate_four_step_bob_cadence(context.frame, 2);

    let state = GroundWalkerState::from_raw(actor.state);
    match state {
        GroundWalkerState::Init => {
            actor.flash = FlashFlags::from_bits(0x10);
            actor.contact_kind = ContactKind::from_raw(0x4E);
            let (next_state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
                std::cmp::Ordering::Greater => (GroundWalkerState::WalkRight.to_raw(), true, 1),
                std::cmp::Ordering::Less => (GroundWalkerState::WalkLeft.to_raw(), false, 0xFF),
                std::cmp::Ordering::Equal => (GroundWalkerState::WalkLeft.to_raw(), false, 0),
            };
            actor.state = next_state;
            actor.sprite_flip.set_horizontal(flip);
            actor.timer = 1;
            GroundWalkerEffect::Initialized { facing }
        }
        GroundWalkerState::WalkLeft => {
            if world.probe_left_wall(actor) {
                actor.sprite_flip.toggle_horizontal();
                actor.state = GroundWalkerState::WalkRight.to_raw();
                GroundWalkerEffect::Turned {
                    state: GroundWalkerState::WalkRight,
                }
            } else {
                pause_step(actor, true)
            }
        }
        GroundWalkerState::WalkRight => {
            if world.probe_right_wall(actor) {
                actor.sprite_flip.toggle_horizontal();
                actor.state = GroundWalkerState::WalkLeft.to_raw();
                GroundWalkerEffect::Turned {
                    state: GroundWalkerState::WalkLeft,
                }
            } else {
                pause_step(actor, false)
            }
        }
    }
}

fn pause_step(actor: &mut Actor, left: bool) -> GroundWalkerEffect {
    if actor.timer != 0 {
        actor.timer = actor.timer.wrapping_sub(1);
        if actor.timer != 0 {
            return GroundWalkerEffect::None;
        }
        actor.set_travel_span(0x30);
        return GroundWalkerEffect::None;
    }

    if left {
        actor.position.move_x(-1);
    } else {
        actor.position.move_x(1);
    }

    let budget = actor.travel_span().wrapping_sub(1);
    actor.set_travel_span(budget);
    if budget != 0 {
        return GroundWalkerEffect::Moved {
            state: if left {
                GroundWalkerState::WalkLeft
            } else {
                GroundWalkerState::WalkRight
            },
        };
    }

    actor.timer = 0x10;
    GroundWalkerEffect::Paused
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockWorld {
        left_wall: bool,
        right_wall: bool,
    }

    impl Default for MockWorld {
        fn default() -> Self {
            Self {
                left_wall: false,
                right_wall: false,
            }
        }
    }

    impl GroundWalkerWorld for MockWorld {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            self.left_wall
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            self.right_wall
        }
    }

    #[test]
    fn groundwalker_init_faces_mario_and_sets_flash_and_type() {
        let mut world = MockWorld::default();

        // Mario east -> state 2, flip horizontal, facing 1
        let mut actor = Actor::default();
        actor.position.x = 100;
        let eff = update(
            &mut actor,
            GroundWalkerContext {
                frame: FrameTick(1), // odd: no bob
                mario_ref_x: 108,
            },
            &mut world,
        );
        assert_eq!(eff, GroundWalkerEffect::Initialized { facing: 1 });
        assert_eq!(actor.flash.bits(), 0x10);
        assert_eq!(actor.contact_kind.raw(), 0x4E);
        assert_eq!(actor.state, 2);
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.timer, 1);

        // Mario west -> state 1, no flip, facing 0xFF
        let mut actor = Actor::default();
        actor.position.x = 100;
        let eff = update(
            &mut actor,
            GroundWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 92,
            },
            &mut world,
        );
        assert_eq!(eff, GroundWalkerEffect::Initialized { facing: 0xFF });
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.timer, 1);
    }

    #[test]
    fn groundwalker_cadence_counts_down_then_steps_48_times() {
        let mut world = MockWorld::default();
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.state = 1;
        actor.timer = 1;

        // Call 1: timer 1 -> 0, arms budget $30, no move
        let eff = update(
            &mut actor,
            GroundWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(eff, GroundWalkerEffect::None);
        assert_eq!(actor.position.x, 100);
        assert_eq!(actor.timer, 0);
        assert_eq!(actor.travel_span(), 0x30);

        // 47 steps: each moves X - 1
        for _ in 0..47 {
            let eff = update(
                &mut actor,
                GroundWalkerContext {
                    frame: FrameTick(1),
                    mario_ref_x: 0,
                },
                &mut world,
            );
            assert_eq!(
                eff,
                GroundWalkerEffect::Moved {
                    state: GroundWalkerState::WalkLeft
                }
            );
        }
        assert_eq!(actor.position.x, 53);
        assert_eq!(actor.travel_span(), 1);

        // 48th step: reaches 0 budget, sets timer to 0x10
        let eff = update(
            &mut actor,
            GroundWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(eff, GroundWalkerEffect::Paused);
        assert_eq!(actor.position.x, 52);
        assert_eq!(actor.timer, 0x10);
        assert_eq!(actor.travel_span(), 0);
    }

    #[test]
    fn groundwalker_wall_flips_direction() {
        // Left wall hit in state 1 -> turns to state 2, flips horizontal
        let mut world = MockWorld {
            left_wall: true,
            ..MockWorld::default()
        };
        let mut actor = Actor::default();
        actor.state = 1;
        actor.position.x = 100;
        actor.timer = 5;
        actor.set_travel_span(0x30);

        let eff = update(
            &mut actor,
            GroundWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            GroundWalkerEffect::Turned {
                state: GroundWalkerState::WalkRight
            }
        );
        assert_eq!(actor.state, 2);
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100);
        assert_eq!(actor.timer, 5);
        assert_eq!(actor.travel_span(), 0x30);

        // Right wall hit in state 2 -> turns to state 1, unflipped
        let mut world = MockWorld {
            right_wall: true,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            GroundWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            GroundWalkerEffect::Turned {
                state: GroundWalkerState::WalkLeft
            }
        );
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100);
    }
}
