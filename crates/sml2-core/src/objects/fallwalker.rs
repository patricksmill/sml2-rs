//! Native fallwalker enemy domain behavior, ROM `$582D1` (anim 1, 56) and `$594EB` (anim 68, 52).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FrameTick};

/// Fallwalker movement state ($582FB: states 0..=2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallWalkerState {
    Init = 0,
    WalkLeft = 1,
    WalkRight = 2,
}

impl FallWalkerState {
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

pub struct FallWalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
    pub contact_kind: ContactKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallWalkerEffect {
    None,
    Initialized { facing: u8 },
    Falling,
    Turned { state: FallWalkerState },
    Moved { state: FallWalkerState },
    LedgeReversed { state: FallWalkerState },
}

pub trait FallWalkerWorld {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
    fn probe_ground(&mut self, actor: &Actor) -> bool;
    fn step_fall(&mut self, actor: &mut Actor) -> u8;
}

/// Domain update for FallWalker (ROM $582D1 / $594EB).
pub fn update<W>(
    actor: &mut Actor,
    context: FallWalkerContext,
    world: &mut W,
) -> FallWalkerEffect
where
    W: FallWalkerWorld,
{
    // $5B585: four-step bob at period 2 (fff0 & 1 == 0)
    let _ = actor.animate_four_step_bob_cadence(context.frame, 2);

    let state = FallWalkerState::from_raw(actor.state);
    match state {
        FallWalkerState::Init => {
            actor.contact_kind = context.contact_kind;
            // $582DD: fall until grounded, then face Mario ($582EE).
            if world.probe_ground(actor) {
                actor.motion_phase = 0;
                let (next_state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
                    std::cmp::Ordering::Greater => (FallWalkerState::WalkRight.to_raw(), true, 1),
                    std::cmp::Ordering::Less => (FallWalkerState::WalkLeft.to_raw(), false, 0xFF),
                    std::cmp::Ordering::Equal => (FallWalkerState::WalkLeft.to_raw(), false, 0),
                };
                actor.state = next_state;
                actor.sprite_flip.set_horizontal(flip);
                FallWalkerEffect::Initialized { facing }
            } else {
                let dy = world.step_fall(actor);
                actor.position.move_y(i16::from(dy));
                FallWalkerEffect::Falling
            }
        }
        FallWalkerState::WalkLeft | FallWalkerState::WalkRight => {
            patrol(actor, context.frame, world).unwrap_or(FallWalkerEffect::None)
        }
    }
}

/// Shared `$582FB`/`$5831A` patrol body, also entered directly by anims 65/66.
///
/// Even frames move one pixel and probe the leading wall (turning on contact);
/// odd frames probe the ground and reverse at a ledge. The caller owns the
/// sprite cadence: `$582D1`/`$594EB` run `$5B585`, while anims 65/66 run
/// `$5B511`. Returns `None` for a state outside the patrol pair.
pub(super) fn patrol<W>(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut W,
) -> Option<FallWalkerEffect>
where
    W: FallWalkerWorld,
{
    match FallWalkerState::from_raw(actor.state) {
        FallWalkerState::WalkLeft => Some(if frame.every(2) {
            actor.position.move_x(-1);
            if world.probe_left_wall(actor) {
                actor.state = FallWalkerState::WalkRight.to_raw();
                actor.sprite_flip.toggle_horizontal();
                FallWalkerEffect::Turned {
                    state: FallWalkerState::WalkRight,
                }
            } else {
                FallWalkerEffect::Moved {
                    state: FallWalkerState::WalkLeft,
                }
            }
        } else {
            // Odd frame: ledge reverse
            if !world.probe_ground(actor) {
                actor.state = FallWalkerState::WalkRight.to_raw();
                actor.sprite_flip.toggle_horizontal();
                FallWalkerEffect::LedgeReversed {
                    state: FallWalkerState::WalkRight,
                }
            } else {
                FallWalkerEffect::None
            }
        }),
        FallWalkerState::WalkRight => Some(if frame.every(2) {
            actor.position.move_x(1);
            if world.probe_right_wall(actor) {
                actor.state = FallWalkerState::WalkLeft.to_raw();
                actor.sprite_flip.toggle_horizontal();
                FallWalkerEffect::Turned {
                    state: FallWalkerState::WalkLeft,
                }
            } else {
                FallWalkerEffect::Moved {
                    state: FallWalkerState::WalkRight,
                }
            }
        } else {
            // Odd frame: ledge reverse
            if !world.probe_ground(actor) {
                actor.state = FallWalkerState::WalkLeft.to_raw();
                actor.sprite_flip.toggle_horizontal();
                FallWalkerEffect::LedgeReversed {
                    state: FallWalkerState::WalkLeft,
                }
            } else {
                FallWalkerEffect::None
            }
        }),
        FallWalkerState::Init => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockWorld {
        left_wall: bool,
        right_wall: bool,
        ground: bool,
        fall_dy: u8,
        stepped_fall: bool,
    }

    impl Default for MockWorld {
        fn default() -> Self {
            Self {
                left_wall: false,
                right_wall: false,
                ground: true,
                fall_dy: 0,
                stepped_fall: false,
            }
        }
    }

    impl FallWalkerWorld for MockWorld {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            self.left_wall
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            self.right_wall
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            self.ground
        }
        fn step_fall(&mut self, _actor: &mut Actor) -> u8 {
            self.stepped_fall = true;
            self.fall_dy
        }
    }

    #[test]
    fn fallwalker_init_airborne_falls_with_gravity_step() {
        let mut world = MockWorld {
            ground: false,
            fall_dy: 3,
            ..MockWorld::default()
        };
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.position.y = 200;

        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(1), // odd frame skips bob
                mario_ref_x: 150,
                contact_kind: ContactKind::from_raw(0x56),
            },
            &mut world,
        );

        assert_eq!(eff, FallWalkerEffect::Falling);
        assert_eq!(actor.contact_kind.raw(), 0x56);
        assert_eq!(actor.position.y, 203);
        assert_eq!(actor.state, 0);
        assert!(world.stepped_fall);
    }

    #[test]
    fn fallwalker_init_grounded_faces_mario() {
        let mut world = MockWorld {
            ground: true,
            ..MockWorld::default()
        };

        // Mario right -> state 2, flip horizontal, facing 1
        let mut actor = Actor::default();
        actor.position.x = 100;
        actor.motion_phase = 5;
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 150,
                contact_kind: ContactKind::from_raw(0x52),
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::Initialized { facing: 1 });
        assert_eq!(actor.contact_kind.raw(), 0x52);
        assert_eq!(actor.state, 2);
        assert_eq!(actor.motion_phase, 0);
        assert!(actor.sprite_flip.horizontal());

        // Mario left -> state 1, no flip, facing 0xFF
        let mut actor = Actor::default();
        actor.position.x = 100;
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
                contact_kind: ContactKind::from_raw(0x56),
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::Initialized { facing: 0xFF });
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());

        // Mario equal X -> state 1, no flip, facing 0
        let mut actor = Actor::default();
        actor.position.x = 100;
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 100,
                contact_kind: ContactKind::from_raw(0x56),
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::Initialized { facing: 0 });
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());
    }

    #[test]
    fn fallwalker_sprite_bob_runs_at_period_two() {
        let mut world = MockWorld::default();
        let mut actor = Actor::default();
        actor.state = 1;
        actor.sprite = 10;
        actor.animation_frame = 0;

        // Frame 0: even -> bobs (phase 0 -> 1, delta -2)
        update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(actor.animation_frame, 1);
        assert_eq!(actor.sprite, 11); // 10 + 1 (phase 1 delta)

        // Frame 1: odd -> no bob
        update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(actor.animation_frame, 1);
        assert_eq!(actor.sprite, 11);

        // Frame 2: even -> bobs (phase 1 -> 2, delta -1)
        update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(actor.animation_frame, 2);
        assert_eq!(actor.sprite, 10); // 11 - 1 (phase 2 delta)
    }

    #[test]
    fn fallwalker_walk_left_even_frame_moves_or_turns_on_wall() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.position.x = 100;

        // Even frame, no wall -> moves left (X-1)
        let mut world = MockWorld {
            left_wall: false,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::Moved { state: FallWalkerState::WalkLeft });
        assert_eq!(actor.position.x, 99);
        assert_eq!(actor.state, 1);

        // Even frame, left wall -> turns right (state 2, toggles flip)
        let mut world = MockWorld {
            left_wall: true,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(4),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::Turned { state: FallWalkerState::WalkRight });
        assert_eq!(actor.state, 2);
        assert!(actor.sprite_flip.horizontal());
    }

    #[test]
    fn fallwalker_walk_right_even_frame_moves_or_turns_on_wall() {
        let mut actor = Actor::default();
        actor.state = 2;
        actor.position.x = 100;
        actor.sprite_flip.set_horizontal(true);

        // Even frame, no wall -> moves right (X+1)
        let mut world = MockWorld {
            right_wall: false,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::Moved { state: FallWalkerState::WalkRight });
        assert_eq!(actor.position.x, 101);
        assert_eq!(actor.state, 2);

        // Even frame, right wall -> turns left (state 1, toggles flip)
        let mut world = MockWorld {
            right_wall: true,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(4),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::Turned { state: FallWalkerState::WalkLeft });
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());
    }

    #[test]
    fn fallwalker_odd_frame_ledge_reverses_at_cliff_edge() {
        // State 1 (WalkLeft) on odd frame with no ground -> reverses to state 2, flips horizontal
        let mut actor = Actor::default();
        actor.state = 1;
        actor.position.x = 100;
        let mut world = MockWorld {
            ground: false,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::LedgeReversed { state: FallWalkerState::WalkRight });
        assert_eq!(actor.state, 2);
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100, "does not change X on ledge reverse");

        // State 2 (WalkRight) on odd frame with no ground -> reverses to state 1, flips horizontal
        let mut actor = Actor::default();
        actor.state = 2;
        actor.sprite_flip.set_horizontal(true);
        actor.position.x = 100;
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(3),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::LedgeReversed { state: FallWalkerState::WalkLeft });
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100);

        // With ground present on odd frame -> does not reverse
        let mut actor = Actor::default();
        actor.state = 1;
        let mut world = MockWorld {
            ground: true,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            FallWalkerContext {
                frame: FrameTick(5),
                mario_ref_x: 0,
                contact_kind: ContactKind::NONE,
            },
            &mut world,
        );
        assert_eq!(eff, FallWalkerEffect::None);
        assert_eq!(actor.state, 1);
    }
}
