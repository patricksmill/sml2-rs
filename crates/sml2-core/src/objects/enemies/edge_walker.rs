//! Native edge walker enemy domain behavior, ROM `$59BAF` (anim 93 Mode A & B, levels 5, 22).
//!
//! Patrol walker that stays on platforms:
//! - Walks along ground on even frames (1 px/frame).
//! - Reverses direction when encountering a wall or ledge.
//! - Animates four-step bob at cadence 4 ($5B57E).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FrameTick};

pub const CONTACT_KIND: u8 = 77; // ROM $59BB7: ld a, 77 (0x4D)

/// Edge walker movement state ($59BB2 / $59BC0: states 0..=2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeWalkerState {
    Init = 0,
    WalkLeft = 1,
    WalkRight = 2,
}

impl EdgeWalkerState {
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

pub struct EdgeWalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeWalkerEffect {
    None,
    Initialized { facing: u8 },
    Turned { state: EdgeWalkerState },
    Moved { state: EdgeWalkerState },
    LedgeReversed { state: EdgeWalkerState },
}

pub trait EdgeWalkerWorld {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
    fn probe_ground(&mut self, actor: &Actor) -> bool;
}

/// Domain update for EdgeWalker (ROM $59BAF).
pub fn update<W>(actor: &mut Actor, context: EdgeWalkerContext, world: &mut W) -> EdgeWalkerEffect
where
    W: EdgeWalkerWorld,
{
    // ROM $59BAF: call $5B57E (four-step bob at period 4)
    let _ = actor.animate_four_step_bob_cadence(context.frame, 4);

    let state = EdgeWalkerState::from_raw(actor.state);
    match state {
        EdgeWalkerState::Init => {
            actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);

            // ROM $5B3C6: face toward Mario
            let (next_state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
                std::cmp::Ordering::Greater => (EdgeWalkerState::WalkRight, true, 1),
                std::cmp::Ordering::Less => (EdgeWalkerState::WalkLeft, false, 0xFF),
                std::cmp::Ordering::Equal => (EdgeWalkerState::WalkLeft, false, 0),
            };
            actor.state = next_state.to_raw();
            actor.sprite_flip.set_horizontal(flip);
            EdgeWalkerEffect::Initialized { facing }
        }
        EdgeWalkerState::WalkLeft => {
            if context.frame.every(2) {
                // Even frame: step left 1 px and probe left wall
                actor.position.move_x(-1);
                if world.probe_left_wall(actor) {
                    actor.state = EdgeWalkerState::WalkRight.to_raw();
                    actor.sprite_flip.toggle_horizontal();
                    EdgeWalkerEffect::Turned {
                        state: EdgeWalkerState::WalkRight,
                    }
                } else {
                    EdgeWalkerEffect::Moved {
                        state: EdgeWalkerState::WalkLeft,
                    }
                }
            } else {
                // Odd frame: probe ground; turn around if ledge
                if !world.probe_ground(actor) {
                    actor.state = EdgeWalkerState::WalkRight.to_raw();
                    actor.sprite_flip.toggle_horizontal();
                    EdgeWalkerEffect::LedgeReversed {
                        state: EdgeWalkerState::WalkRight,
                    }
                } else {
                    EdgeWalkerEffect::None
                }
            }
        }
        EdgeWalkerState::WalkRight => {
            if context.frame.every(2) {
                // Even frame: step right 1 px and probe right wall
                actor.position.move_x(1);
                if world.probe_right_wall(actor) {
                    actor.state = EdgeWalkerState::WalkLeft.to_raw();
                    actor.sprite_flip.toggle_horizontal();
                    EdgeWalkerEffect::Turned {
                        state: EdgeWalkerState::WalkLeft,
                    }
                } else {
                    EdgeWalkerEffect::Moved {
                        state: EdgeWalkerState::WalkRight,
                    }
                }
            } else {
                // Odd frame: probe ground; turn around if ledge
                if !world.probe_ground(actor) {
                    actor.state = EdgeWalkerState::WalkLeft.to_raw();
                    actor.sprite_flip.toggle_horizontal();
                    EdgeWalkerEffect::LedgeReversed {
                        state: EdgeWalkerState::WalkLeft,
                    }
                } else {
                    EdgeWalkerEffect::None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{Position, ScreenPosition};

    struct MockWorld {
        left_wall: bool,
        right_wall: bool,
        ground: bool,
    }

    impl MockWorld {
        fn new_solid_ground() -> Self {
            Self {
                left_wall: false,
                right_wall: false,
                ground: true,
            }
        }
    }

    impl EdgeWalkerWorld for MockWorld {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            self.left_wall
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            self.right_wall
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            self.ground
        }
    }

    fn make_test_actor(x: u16, y: u16) -> Actor {
        let mut actor = Actor::default();
        actor.position = Position { x, y };
        actor.screen = ScreenPosition { x: 80, y: 72 };
        actor.sprite = 90;
        actor
    }

    #[test]
    fn test_init_facing_left_and_right() {
        let mut world = MockWorld::new_solid_ground();

        // Mario left
        let mut actor = make_test_actor(100, 80);
        let eff = update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 60,
            },
            &mut world,
        );
        assert_eq!(eff, EdgeWalkerEffect::Initialized { facing: 0xFF });
        assert_eq!(actor.state, EdgeWalkerState::WalkLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.contact_kind.raw(), CONTACT_KIND);

        // Mario right
        let mut actor = make_test_actor(100, 80);
        let eff = update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 140,
            },
            &mut world,
        );
        assert_eq!(eff, EdgeWalkerEffect::Initialized { facing: 1 });
        assert_eq!(actor.state, EdgeWalkerState::WalkRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
    }

    #[test]
    fn test_walk_movement_and_wall_turnaround() {
        let mut world = MockWorld::new_solid_ground();
        let mut actor = make_test_actor(100, 80);
        actor.state = EdgeWalkerState::WalkLeft.to_raw();

        // Frame 0 (even): steps left 1 px to 99
        let eff = update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 60,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            EdgeWalkerEffect::Moved {
                state: EdgeWalkerState::WalkLeft
            }
        );
        assert_eq!(actor.position.x, 99);

        // Frame 1 (odd): ground probe passes, no position change
        let eff = update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 60,
            },
            &mut world,
        );
        assert_eq!(eff, EdgeWalkerEffect::None);
        assert_eq!(actor.position.x, 99);

        // Frame 2 (even): hits left wall -> turns around to WalkRight
        world.left_wall = true;
        let eff = update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 60,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            EdgeWalkerEffect::Turned {
                state: EdgeWalkerState::WalkRight
            }
        );
        assert_eq!(actor.state, EdgeWalkerState::WalkRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
    }

    #[test]
    fn test_ledge_turnaround() {
        let mut world = MockWorld::new_solid_ground();
        let mut actor = make_test_actor(100, 80);
        actor.state = EdgeWalkerState::WalkRight.to_raw();
        actor.sprite_flip.set_horizontal(true);

        // Frame 0: moves right 1 px to 101
        update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 120,
            },
            &mut world,
        );
        assert_eq!(actor.position.x, 101);

        // Frame 1: ledge reached (no ground) -> turns around to WalkLeft
        world.ground = false;
        let eff = update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 120,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            EdgeWalkerEffect::LedgeReversed {
                state: EdgeWalkerState::WalkLeft
            }
        );
        assert_eq!(actor.state, EdgeWalkerState::WalkLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
    }

    #[test]
    fn test_four_step_bob_cadence_4() {
        let mut world = MockWorld::new_solid_ground();
        let mut actor = make_test_actor(100, 80);
        actor.state = EdgeWalkerState::WalkRight.to_raw();
        actor.sprite = 90;

        // Frame 0: advances bob (period 4)
        update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 100,
            },
            &mut world,
        );
        assert_eq!(actor.animation_frame, 1);
        assert_eq!(actor.sprite, 91); // 90 + (+1) = 91

        // Frames 1..=3: no bob advance
        for f in 1..=3 {
            update(
                &mut actor,
                EdgeWalkerContext {
                    frame: FrameTick(f),
                    mario_ref_x: 100,
                },
                &mut world,
            );
            assert_eq!(actor.animation_frame, 1);
        }

        // Frame 4: advances bob
        update(
            &mut actor,
            EdgeWalkerContext {
                frame: FrameTick(4),
                mario_ref_x: 100,
            },
            &mut world,
        );
        assert_eq!(actor.animation_frame, 2);
        assert_eq!(actor.sprite, 90); // 91 + (-1) = 90
    }
}
