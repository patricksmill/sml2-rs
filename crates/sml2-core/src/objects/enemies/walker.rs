//! Native walker enemy domain behavior, ROM `$58201` (anims 9, 59).
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

/// Walker movement state ($5821E: states 1..=4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkerState {
    Init = 0,
    WalkLeft = 1,
    WalkRight = 2,
    FallLeft = 3,
    FallRight = 4,
}

impl WalkerState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::WalkLeft,
            2 => Self::WalkRight,
            3 => Self::FallLeft,
            4 => Self::FallRight,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct WalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkerEffect {
    None,
    Initialized { facing: u8 },
    Turned { state: WalkerState },
    Moved { state: WalkerState },
    StartedFalling { state: WalkerState },
    Landed { state: WalkerState },
}

pub trait WalkerWorld {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
    fn probe_ground(&mut self, actor: &Actor) -> bool;
    fn step_fall(&mut self, actor: &mut Actor) -> u8;
}

/// Domain update for Walker (ROM $58201).
pub fn update<W>(actor: &mut Actor, context: WalkerContext, world: &mut W) -> WalkerEffect
where
    W: WalkerWorld,
{
    // $5B51A: Alternate sprite bit 0 on every 4th frame (FFF0 & 3 == 0)
    actor.animate_two_frame_toggle(context.frame, 4);

    let state = WalkerState::from_raw(actor.state);
    if state == WalkerState::Init {
        // $5B3C6: face Mario ($5B3E0)
        let (state, flip, facing) = match context.mario_ref_x.cmp(&actor.position.x) {
            std::cmp::Ordering::Greater => (WalkerState::WalkRight.to_raw(), true, 1),
            std::cmp::Ordering::Less => (WalkerState::WalkLeft.to_raw(), false, 0xFF),
            std::cmp::Ordering::Equal => (WalkerState::WalkLeft.to_raw(), false, 0),
        };
        actor.state = state;
        actor.sprite_flip.set_horizontal(flip);
        actor.flash = FlashFlags::from_bits(16);
        actor.contact_kind = ContactKind::from_raw(0x4E);
        return WalkerEffect::Initialized { facing };
    }

    // $58215: Main movement loop runs only on even simulation frames
    if !context.frame.every(2) {
        return WalkerEffect::None;
    }

    let leg = match state {
        WalkerState::WalkLeft => WalkLeg::WalkLeft,
        WalkerState::WalkRight => WalkLeg::WalkRight,
        WalkerState::FallLeft => WalkLeg::FallLeft,
        WalkerState::FallRight => WalkLeg::FallRight,
        WalkerState::Init => unreachable!(),
    };
    let step = step_walk(actor, leg, world);
    let state = WalkerState::from_raw(actor.state);
    match step {
        WalkStep::Turned => WalkerEffect::Turned { state },
        WalkStep::Moved => WalkerEffect::Moved { state },
        WalkStep::StartedFalling => WalkerEffect::StartedFalling { state },
        WalkStep::Landed => WalkerEffect::Landed { state },
    }
}

/// One leg of the `$5821E` walk table: which way the actor walks and whether
/// it is falling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkLeg {
    WalkLeft,
    WalkRight,
    FallLeft,
    FallRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkStep {
    Turned,
    Moved,
    StartedFalling,
    Landed,
}

/// `$58226..$58293`: one pixel of walking or falling. The legs move the frame
/// state relative to its current value (turn +-1, fall +2, land -2), so hosts
/// whose walk legs start above state 1 (the hopper) share the same body.
pub fn step_walk<W>(actor: &mut Actor, leg: WalkLeg, world: &mut W) -> WalkStep
where
    W: WalkerWorld,
{
    let leftward = matches!(leg, WalkLeg::WalkLeft | WalkLeg::FallLeft);
    let blocked = if leftward {
        world.probe_left_wall(actor)
    } else {
        world.probe_right_wall(actor)
    };
    if blocked {
        actor.state = if leftward {
            actor.state.wrapping_add(1)
        } else {
            actor.state.wrapping_sub(1)
        };
        actor.sprite_flip.toggle_horizontal();
        return WalkStep::Turned;
    }
    actor.position.move_x(if leftward { -1 } else { 1 });
    if matches!(leg, WalkLeg::WalkLeft | WalkLeg::WalkRight) {
        if world.probe_ground(actor) {
            return WalkStep::Moved;
        }
        actor.state = actor.state.wrapping_add(2);
        return WalkStep::StartedFalling;
    }
    let dy = world.step_fall(actor);
    actor.position.move_y(i16::from(dy));
    if !world.probe_ground(actor) {
        return WalkStep::Moved;
    }
    actor.state = actor.state.wrapping_sub(2);
    WalkStep::Landed
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

    impl WalkerWorld for MockWorld {
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
    fn walker_initialization_faces_mario_and_configures_actor() {
        let mut world = MockWorld::default();

        // Mario to the right -> state 2, flip horizontal
        let mut actor = Actor::default();
        actor.position.x = 100;
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 150,
            },
            &mut world,
        );
        assert_eq!(eff, WalkerEffect::Initialized { facing: 1 });
        assert_eq!(actor.state, 2);
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.flash.bits(), 16);
        assert_eq!(actor.contact_kind.raw(), 0x4E);

        // Mario to the left -> state 1, no flip, facing 0xFF
        let mut actor = Actor::default();
        actor.position.x = 100;
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(eff, WalkerEffect::Initialized { facing: 0xFF });
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.flash.bits(), 16);
        assert_eq!(actor.contact_kind.raw(), 0x4E);

        // Mario equal X -> state 1, no flip, facing 0
        let mut actor = Actor::default();
        actor.position.x = 100;
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 100,
            },
            &mut world,
        );
        assert_eq!(eff, WalkerEffect::Initialized { facing: 0 });
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());
    }

    #[test]
    fn walker_animation_toggles_low_sprite_bit_every_fourth_frame() {
        let mut world = MockWorld::default();
        let mut actor = Actor::default();
        actor.state = 1;
        actor.sprite = 10;

        // Frame 0: 0 % 4 == 0 -> toggles
        update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(actor.sprite, 11);

        // Frame 1: 1 % 4 != 0 -> no toggle
        update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(actor.sprite, 11);

        // Frame 2: 2 % 4 != 0 -> no toggle
        update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(actor.sprite, 11);

        // Frame 3: 3 % 4 != 0 -> no toggle
        update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(3),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(actor.sprite, 11);

        // Frame 4: 4 % 4 == 0 -> toggles back
        update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(4),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(actor.sprite, 10);
    }

    #[test]
    fn walker_movement_is_gated_to_even_frames() {
        let mut world = MockWorld::default();
        let mut actor = Actor::default();
        actor.state = 1;
        actor.position.x = 100;

        // Odd frame: movement does not run
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(eff, WalkerEffect::None);
        assert_eq!(actor.position.x, 100);

        // Even frame: movement runs
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::Moved {
                state: WalkerState::WalkLeft
            }
        );
        assert_eq!(actor.position.x, 99);
    }

    #[test]
    fn walker_walk_left_turns_at_wall_or_falls_at_ledge() {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.position.x = 100;

        // Wall hit: turns to WalkRight, toggles flip, does not move X
        let mut world = MockWorld {
            left_wall: true,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::Turned {
                state: WalkerState::WalkRight
            }
        );
        assert_eq!(actor.state, 2);
        assert!(actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100);

        // Ground missing: moves X and enters FallLeft
        let mut actor = Actor::default();
        actor.state = 1;
        actor.position.x = 100;
        let mut world = MockWorld {
            ground: false,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::StartedFalling {
                state: WalkerState::FallLeft
            }
        );
        assert_eq!(actor.state, 3);
        assert_eq!(actor.position.x, 99);
    }

    #[test]
    fn walker_walk_right_turns_at_wall_or_falls_at_ledge() {
        let mut actor = Actor::default();
        actor.state = 2;
        actor.sprite_flip.set_horizontal(true);
        actor.position.x = 100;

        // Wall hit: turns to WalkLeft, toggles flip, does not move X
        let mut world = MockWorld {
            right_wall: true,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::Turned {
                state: WalkerState::WalkLeft
            }
        );
        assert_eq!(actor.state, 1);
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.position.x, 100);

        // Ground missing: moves X and enters FallRight
        let mut actor = Actor::default();
        actor.state = 2;
        actor.position.x = 100;
        let mut world = MockWorld {
            ground: false,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::StartedFalling {
                state: WalkerState::FallRight
            }
        );
        assert_eq!(actor.state, 4);
        assert_eq!(actor.position.x, 101);
    }

    #[test]
    fn walker_falling_applies_gravity_and_lands_or_turns() {
        // FallLeft with wall hit -> turns to FallRight
        let mut actor = Actor::default();
        actor.state = 3;
        actor.position.x = 100;
        let mut world = MockWorld {
            left_wall: true,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::Turned {
                state: WalkerState::FallRight
            }
        );
        assert_eq!(actor.state, 4);
        assert!(actor.sprite_flip.horizontal());

        // FallLeft without ground -> moves X - 1, moves Y + dy
        let mut actor = Actor::default();
        actor.state = 3;
        actor.position.x = 100;
        actor.position.y = 50;
        let mut world = MockWorld {
            ground: false,
            fall_dy: 3,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::Moved {
                state: WalkerState::FallLeft
            }
        );
        assert_eq!(actor.state, 3);
        assert_eq!(actor.position.x, 99);
        assert_eq!(actor.position.y, 53);

        // FallLeft with ground -> lands and returns to WalkLeft (state 1)
        let mut actor = Actor::default();
        actor.state = 3;
        actor.position.x = 100;
        actor.position.y = 50;
        let mut world = MockWorld {
            ground: true,
            fall_dy: 2,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::Landed {
                state: WalkerState::WalkLeft
            }
        );
        assert_eq!(actor.state, 1);
        assert_eq!(actor.position.x, 99);
        assert_eq!(actor.position.y, 52);

        // FallRight with ground -> lands and returns to WalkRight (state 2)
        let mut actor = Actor::default();
        actor.state = 4;
        actor.position.x = 100;
        actor.position.y = 50;
        let mut world = MockWorld {
            ground: true,
            fall_dy: 4,
            ..MockWorld::default()
        };
        let eff = update(
            &mut actor,
            WalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 0,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WalkerEffect::Landed {
                state: WalkerState::WalkRight
            }
        );
        assert_eq!(actor.state, 2);
        assert_eq!(actor.position.x, 101);
        assert_eq!(actor.position.y, 54);
    }
}
