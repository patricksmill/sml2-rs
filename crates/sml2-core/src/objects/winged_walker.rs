//! Native winged walker enemy domain behavior, ROM `$58F4C` (anim 51 Mode A & B, levels 5, 22).
//!
//! Winged walker (ant/beetle):
//! - Walks along ground (states 1/2) with 4-step walk wobble on even frames.
//! - When hitting a wall or walking off a ledge, takes off (state 5).
//! - Flies straight up for 40 pixels while flapping wings.
//! - Faces Mario and hovers in place for 32 frames (states 3/4).
//! - Swoops downward diagonally toward Mario (X ±2 on even frames, Y +1 on odd frames).
//! - Upon hitting a wall or landing on solid ground, reverts to walking on ground.
//!
//! Follows the idiomatic Rust porting guide: typed actor state, domain methods,
//! zero raw hex offsets or register plumbing.

use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

pub const CONTACT_KIND: u8 = 77; // ROM $58F54: ld a, 77 (0x4D)
pub const FLASH_FLAGS: u8 = 0x10; // ROM $58F58: ld a, 16
pub const FLY_UP_ALTITUDE_PX: u8 = 40; // ROM $58FBD: cp $28 (40 decimal)
pub const HOVER_FRAMES: u8 = 32; // ROM $58FCC: ld a, 32
pub const SPRITE_WALK: u8 = 86; // ROM $59006: ld a, 86
pub const SPRITE_FLY: u8 = 88; // ROM $58F84: ld a, 88

/// Winged walker state ($58F4C / $58F5D: states 0..=5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WingedWalkerState {
    Init = 0,
    WalkLeft = 1,
    WalkRight = 2,
    SwoopLeft = 3,
    SwoopRight = 4,
    FlyUp = 5,
}

impl WingedWalkerState {
    pub const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::WalkLeft,
            2 => Self::WalkRight,
            3 => Self::SwoopLeft,
            4 => Self::SwoopRight,
            5 => Self::FlyUp,
            _ => Self::Init,
        }
    }

    pub const fn to_raw(self) -> u8 {
        self as u8
    }
}

pub struct WingedWalkerContext {
    pub frame: FrameTick,
    pub mario_ref_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WingedWalkerEffect {
    Initialized {
        facing: u8,
    },
    Walked {
        state: WingedWalkerState,
    },
    TakeoffTriggered,
    FlyingUp {
        altitude: u8,
    },
    Hovering {
        timer: u8,
    },
    Swooping {
        state: WingedWalkerState,
    },
    Landed {
        state: WingedWalkerState,
        facing: u8,
    },
}

pub trait WingedWalkerWorld {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_right_wall(&mut self, actor: &Actor) -> bool;
    fn probe_ground(&mut self, actor: &Actor) -> bool;
    fn probe_swoop_left_wall(&mut self, actor: &Actor) -> bool;
    fn probe_swoop_right_wall(&mut self, actor: &Actor) -> bool;
}

/// Domain update for WingedWalker (ROM $58F4C).
pub fn update<W>(
    actor: &mut Actor,
    context: WingedWalkerContext,
    world: &mut W,
) -> WingedWalkerEffect
where
    W: WingedWalkerWorld,
{
    let state = WingedWalkerState::from_raw(actor.state);
    if state == WingedWalkerState::Init {
        actor.contact_kind = ContactKind::from_raw(CONTACT_KIND);
        actor.flash = FlashFlags::from_bits(FLASH_FLAGS);

        // ROM $5B3C6: face toward Mario
        let (next_state, flip, facing) = face_mario(context.mario_ref_x, actor.position.x);
        actor.state = next_state.to_raw();
        actor.sprite_flip.set_horizontal(flip);
        return WingedWalkerEffect::Initialized { facing };
    }

    match state {
        WingedWalkerState::Init => unreachable!(),
        WingedWalkerState::WalkLeft => {
            // ROM $5B54C: walk wobble animation on even frames
            animate_four_step_walk(actor, context.frame);

            if context.frame.every(2) {
                // Even frame: step left and probe left wall ($84EA)
                actor.position.move_x(-1);
                if world.probe_left_wall(actor) {
                    takeoff(actor);
                    WingedWalkerEffect::TakeoffTriggered
                } else {
                    WingedWalkerEffect::Walked { state }
                }
            } else {
                // Odd frame: probe floor ($8358)
                if !world.probe_ground(actor) {
                    takeoff(actor);
                    WingedWalkerEffect::TakeoffTriggered
                } else {
                    WingedWalkerEffect::Walked { state }
                }
            }
        }
        WingedWalkerState::WalkRight => {
            // ROM $5B54C: walk wobble animation on even frames
            animate_four_step_walk(actor, context.frame);

            if context.frame.every(2) {
                // Even frame: step right and probe right wall ($843C)
                actor.position.move_x(1);
                if world.probe_right_wall(actor) {
                    takeoff(actor);
                    WingedWalkerEffect::TakeoffTriggered
                } else {
                    WingedWalkerEffect::Walked { state }
                }
            } else {
                // Odd frame: probe floor ($8358)
                if !world.probe_ground(actor) {
                    takeoff(actor);
                    WingedWalkerEffect::TakeoffTriggered
                } else {
                    WingedWalkerEffect::Walked { state }
                }
            }
        }
        WingedWalkerState::FlyUp => {
            // ROM $5B521: 2-frame flap toggle
            actor.animate_two_frame_toggle(context.frame, 2);

            // ROM $58FB3: moves UP every frame
            actor.position.move_y(-1);
            actor.motion_leg = actor.motion_leg.wrapping_add(1);

            if actor.motion_leg >= FLY_UP_ALTITUDE_PX {
                actor.motion_leg = 0;
                let (walk_state, flip, _) = face_mario(context.mario_ref_x, actor.position.x);
                let swoop_state = match walk_state {
                    WingedWalkerState::WalkLeft => WingedWalkerState::SwoopLeft,
                    WingedWalkerState::WalkRight => WingedWalkerState::SwoopRight,
                    _ => WingedWalkerState::SwoopLeft,
                };
                actor.state = swoop_state.to_raw();
                actor.sprite_flip.set_horizontal(flip);
                actor.timer = HOVER_FRAMES;
                WingedWalkerEffect::Hovering { timer: HOVER_FRAMES }
            } else {
                WingedWalkerEffect::FlyingUp {
                    altitude: actor.motion_leg,
                }
            }
        }
        WingedWalkerState::SwoopLeft => {
            // ROM $5B521: 2-frame flap toggle
            actor.animate_two_frame_toggle(context.frame, 2);

            if actor.timer != 0 {
                actor.timer = actor.timer.wrapping_sub(1);
                return WingedWalkerEffect::Hovering { timer: actor.timer };
            }

            if context.frame.every(2) {
                // Even frame: probe left wall ($8554); if clear, move left by 2
                if world.probe_swoop_left_wall(actor) {
                    land(actor, context.mario_ref_x)
                } else {
                    actor.position.move_x(-2);
                    WingedWalkerEffect::Swooping { state }
                }
            } else {
                // Odd frame: probe floor ($8358); if clear, move down by 1
                if world.probe_ground(actor) {
                    land(actor, context.mario_ref_x)
                } else {
                    actor.position.move_y(1);
                    WingedWalkerEffect::Swooping { state }
                }
            }
        }
        WingedWalkerState::SwoopRight => {
            // ROM $5B521: 2-frame flap toggle
            actor.animate_two_frame_toggle(context.frame, 2);

            if actor.timer != 0 {
                actor.timer = actor.timer.wrapping_sub(1);
                return WingedWalkerEffect::Hovering { timer: actor.timer };
            }

            if context.frame.every(2) {
                // Even frame: probe right wall ($84A6); if clear, move right by 2
                if world.probe_swoop_right_wall(actor) {
                    land(actor, context.mario_ref_x)
                } else {
                    actor.position.move_x(2);
                    WingedWalkerEffect::Swooping { state }
                }
            } else {
                // Odd frame: probe floor ($8358); if clear, move down by 1
                if world.probe_ground(actor) {
                    land(actor, context.mario_ref_x)
                } else {
                    actor.position.move_y(1);
                    WingedWalkerEffect::Swooping { state }
                }
            }
        }
    }
}

fn face_mario(mario_ref_x: u16, actor_x: u16) -> (WingedWalkerState, bool, u8) {
    match mario_ref_x.cmp(&actor_x) {
        std::cmp::Ordering::Greater => (WingedWalkerState::WalkRight, true, 1),
        std::cmp::Ordering::Less => (WingedWalkerState::WalkLeft, false, 0xFF),
        std::cmp::Ordering::Equal => (WingedWalkerState::WalkLeft, false, 0),
    }
}

fn takeoff(actor: &mut Actor) {
    actor.state = WingedWalkerState::FlyUp.to_raw();
    actor.sprite = SPRITE_FLY;
    actor.animation_frame = 0;
    actor.motion_leg = 0;
}

fn land(actor: &mut Actor, mario_ref_x: u16) -> WingedWalkerEffect {
    let (next_state, flip, facing) = face_mario(mario_ref_x, actor.position.x);
    actor.state = next_state.to_raw();
    actor.sprite_flip.set_horizontal(flip);
    actor.sprite = SPRITE_WALK;
    actor.animation_frame = 0;
    WingedWalkerEffect::Landed {
        state: next_state,
        facing,
    }
}

fn animate_four_step_walk(actor: &mut Actor, frame: FrameTick) {
    if !frame.every(2) {
        return;
    }
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    let mut flip_clear = false;
    if actor.animation_frame == 4 {
        actor.animation_frame = 0;
        flip_clear = true;
    }
    const DELTAS: [i8; 4] = [-1, 1, -1, 1];
    let idx = usize::from(actor.animation_frame);
    if idx < DELTAS.len() {
        actor.sprite = actor.sprite.wrapping_add_signed(DELTAS[idx]);
    }
    if flip_clear {
        actor.sprite_flip.set_horizontal(false);
    }
    if actor.animation_frame == 3 {
        actor.sprite_flip.set_horizontal(true);
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
        swoop_left_wall: bool,
        swoop_right_wall: bool,
    }

    impl MockWorld {
        fn new_solid_ground() -> Self {
            Self {
                left_wall: false,
                right_wall: false,
                ground: true,
                swoop_left_wall: false,
                swoop_right_wall: false,
            }
        }
    }

    impl WingedWalkerWorld for MockWorld {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            self.left_wall
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            self.right_wall
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            self.ground
        }
        fn probe_swoop_left_wall(&mut self, _actor: &Actor) -> bool {
            self.swoop_left_wall
        }
        fn probe_swoop_right_wall(&mut self, _actor: &Actor) -> bool {
            self.swoop_right_wall
        }
    }

    fn make_test_actor(x: u16, y: u16) -> Actor {
        let mut actor = Actor::default();
        actor.position = Position { x, y };
        actor.screen = ScreenPosition { x: 80, y: 72 };
        actor.sprite = SPRITE_WALK;
        actor
    }

    #[test]
    fn test_init_facing_left_and_right() {
        let mut world = MockWorld::new_solid_ground();

        // Mario left
        let mut actor = make_test_actor(100, 80);
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 60,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::Initialized { facing: 0xFF });
        assert_eq!(actor.state, WingedWalkerState::WalkLeft.to_raw());
        assert!(!actor.sprite_flip.horizontal());
        assert_eq!(actor.contact_kind.raw(), CONTACT_KIND);
        assert_eq!(actor.flash.bits(), FLASH_FLAGS);

        // Mario right
        let mut actor = make_test_actor(100, 80);
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 140,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::Initialized { facing: 1 });
        assert_eq!(actor.state, WingedWalkerState::WalkRight.to_raw());
        assert!(actor.sprite_flip.horizontal());
    }

    #[test]
    fn test_walk_movement_and_wall_takeoff() {
        let mut world = MockWorld::new_solid_ground();
        let mut actor = make_test_actor(100, 80);
        actor.state = WingedWalkerState::WalkLeft.to_raw();

        // Frame 0 (even): steps left 1 px to 99
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 60,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::Walked { state: WingedWalkerState::WalkLeft });
        assert_eq!(actor.position.x, 99);

        // Frame 1 (odd): ground probe passes, no position change
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 60,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::Walked { state: WingedWalkerState::WalkLeft });
        assert_eq!(actor.position.x, 99);

        // Frame 2 (even): encounters wall -> triggers takeoff
        world.left_wall = true;
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(2),
                mario_ref_x: 60,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::TakeoffTriggered);
        assert_eq!(actor.state, WingedWalkerState::FlyUp.to_raw());
        assert_eq!(actor.sprite, SPRITE_FLY);
        assert_eq!(actor.animation_frame, 0);
        assert_eq!(actor.motion_leg, 0);
    }

    #[test]
    fn test_walk_ledge_takeoff() {
        let mut world = MockWorld::new_solid_ground();
        let mut actor = make_test_actor(100, 80);
        actor.state = WingedWalkerState::WalkRight.to_raw();

        // Frame 0: moves right 1 px
        update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(0),
                mario_ref_x: 120,
            },
            &mut world,
        );

        // Frame 1: ground disappears (walked off ledge) -> takeoff
        world.ground = false;
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 120,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::TakeoffTriggered);
        assert_eq!(actor.state, WingedWalkerState::FlyUp.to_raw());
        assert_eq!(actor.sprite, SPRITE_FLY);
    }

    #[test]
    fn test_fly_up_hover_swoop_and_landing() {
        let mut world = MockWorld::new_solid_ground();
        let mut actor = make_test_actor(100, 100);
        actor.state = WingedWalkerState::FlyUp.to_raw();
        actor.sprite = SPRITE_FLY;

        // Fly up 39 frames
        for f in 1..=39 {
            let eff = update(
                &mut actor,
                WingedWalkerContext {
                    frame: FrameTick(f),
                    mario_ref_x: 150, // Mario to right
                },
                &mut world,
            );
            assert_eq!(eff, WingedWalkerEffect::FlyingUp { altitude: f as u8 });
            assert_eq!(actor.position.y, 100 - f as u16);
        }
        assert_eq!(actor.position.y, 61);

        // 40th frame: reaches peak, transitions to SwoopRight (Mario is at 150 > 100)
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(40),
                mario_ref_x: 150,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::Hovering { timer: 32 });
        assert_eq!(actor.state, WingedWalkerState::SwoopRight.to_raw());
        assert_eq!(actor.position.y, 60);
        assert_eq!(actor.timer, 32);

        // Hover countdown for 32 frames
        for f in 41..=72 {
            let eff = update(
                &mut actor,
                WingedWalkerContext {
                    frame: FrameTick(f),
                    mario_ref_x: 150,
                },
                &mut world,
            );
            assert!(matches!(eff, WingedWalkerEffect::Hovering { .. }));
        }
        assert_eq!(actor.timer, 0);

        // Now swooping starts!
        // Ground is false so it doesn't immediately land
        world.ground = false;

        // Frame 74 (even): moves right by 2 (100 -> 102)
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(74),
                mario_ref_x: 150,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::Swooping { state: WingedWalkerState::SwoopRight });
        assert_eq!(actor.position.x, 102);

        // Frame 75 (odd): moves down by 1 (60 -> 61)
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(75),
                mario_ref_x: 150,
            },
            &mut world,
        );
        assert_eq!(eff, WingedWalkerEffect::Swooping { state: WingedWalkerState::SwoopRight });
        assert_eq!(actor.position.y, 61);

        // Next odd frame, hits ground -> lands!
        world.ground = true;
        let eff = update(
            &mut actor,
            WingedWalkerContext {
                frame: FrameTick(77),
                mario_ref_x: 150,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            WingedWalkerEffect::Landed {
                state: WingedWalkerState::WalkRight,
                facing: 1,
            }
        );
        assert_eq!(actor.state, WingedWalkerState::WalkRight.to_raw());
        assert_eq!(actor.sprite, SPRITE_WALK);
    }
}
