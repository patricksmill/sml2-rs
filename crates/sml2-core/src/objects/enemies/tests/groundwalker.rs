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
    fn probe_left_wall(&mut self, _position: Position) -> bool {
        self.left_wall
    }
    fn probe_right_wall(&mut self, _position: Position) -> bool {
        self.right_wall
    }
}

#[test]
fn groundwalker_init_faces_mario_and_sets_flash_and_type() {
    let mut world = MockWorld::default();

    // Mario east -> state 2, flip horizontal, facing 1
    let mut actor = GroundWalker::default();
    actor.position.x = 100;
    let eff = update(
        &mut actor,
        GroundWalkerContext {
            frame: FrameTick(1), // odd: no bob
            mario_ref_x: 108,
        },
        &mut world,
    );
    assert_eq!(
        eff,
        GroundWalkerEffect::Initialized {
            facing: Ordering::Greater
        }
    );
    assert_eq!(actor.flash.bits(), 0x10);
    assert_eq!(actor.contact_kind.raw(), 0x4E);
    assert_eq!(actor.state, GroundWalkerState::WalkRight);
    assert!(actor.sprite_flip.horizontal());
    assert_eq!(actor.pause_ticks, 1);

    // Mario west -> state 1, no flip, facing 0xFF
    let mut actor = GroundWalker::default();
    actor.position.x = 100;
    let eff = update(
        &mut actor,
        GroundWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 92,
        },
        &mut world,
    );
    assert_eq!(
        eff,
        GroundWalkerEffect::Initialized {
            facing: Ordering::Less
        }
    );
    assert_eq!(actor.state, GroundWalkerState::WalkLeft);
    assert!(!actor.sprite_flip.horizontal());
    assert_eq!(actor.pause_ticks, 1);
}

#[test]
fn groundwalker_cadence_counts_down_then_steps_48_times() {
    let mut world = MockWorld::default();
    let mut actor = GroundWalker::default();
    actor.position.x = 100;
    actor.state = GroundWalkerState::WalkLeft;
    actor.pause_ticks = 1;

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
    assert_eq!(actor.pause_ticks, 0);
    assert_eq!(actor.remaining_pixels, 0x30);

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
    assert_eq!(actor.remaining_pixels, 1);

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
    assert_eq!(actor.pause_ticks, 0x10);
    assert_eq!(actor.remaining_pixels, 0);
}

#[test]
fn groundwalker_wall_flips_direction() {
    // Left wall hit in state 1 -> turns to state 2, flips horizontal
    let mut world = MockWorld {
        left_wall: true,
        ..MockWorld::default()
    };
    let mut actor = GroundWalker::default();
    actor.state = GroundWalkerState::WalkLeft;
    actor.position.x = 100;
    actor.pause_ticks = 5;
    actor.remaining_pixels = 0x30;

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
    assert_eq!(actor.state, GroundWalkerState::WalkRight);
    assert!(actor.sprite_flip.horizontal());
    assert_eq!(actor.position.x, 100);
    assert_eq!(actor.pause_ticks, 5);
    assert_eq!(actor.remaining_pixels, 0x30);

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
    assert_eq!(actor.state, GroundWalkerState::WalkLeft);
    assert!(!actor.sprite_flip.horizontal());
    assert_eq!(actor.position.x, 100);
}
