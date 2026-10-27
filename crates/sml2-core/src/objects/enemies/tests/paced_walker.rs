use super::*;

struct MockWorld {
    left_wall: bool,
    right_wall: bool,
}

impl PacedWalkerWorld for MockWorld {
    fn probe_left_wall(&mut self, _position: Position) -> bool {
        self.left_wall
    }
    fn probe_right_wall(&mut self, _position: Position) -> bool {
        self.right_wall
    }
}

#[test]
fn paced_walker_init_faces_mario_right_and_sets_flash_and_type() {
    let mut actor = PacedWalker::default();
    actor.position.x = 100;
    let mut world = MockWorld {
        left_wall: false,
        right_wall: false,
    };
    let context = PacedWalkerContext {
        frame: FrameTick(1), // not multiple of 4, so sprite doesn't toggle
        mario_ref_x: 150,
    };

    let effect = update(&mut actor, context, &mut world);

    assert_eq!(actor.state, PacedWalkerState::WalkRight);
    assert!(actor.sprite_flip.horizontal());
    assert_eq!(actor.flash.bits(), 0x10);
    assert_eq!(actor.contact_kind, ContactKind::from_raw(CONTACT_KIND));
    assert_eq!(actor.pause_ticks, 1);
    assert_eq!(
        effect,
        PacedWalkerEffect::Initialized {
            facing: Ordering::Greater
        }
    );
}

#[test]
fn paced_walker_init_faces_mario_left_and_equal() {
    let mut actor = PacedWalker::default();
    actor.position.x = 100;
    let mut world = MockWorld {
        left_wall: false,
        right_wall: false,
    };

    // Mario to the left
    let context_left = PacedWalkerContext {
        frame: FrameTick(1),
        mario_ref_x: 50,
    };
    let effect_left = update(&mut actor, context_left, &mut world);
    assert_eq!(actor.state, PacedWalkerState::WalkLeft);
    assert!(!actor.sprite_flip.horizontal());
    assert_eq!(
        effect_left,
        PacedWalkerEffect::Initialized {
            facing: Ordering::Less
        }
    );

    // Mario equal
    actor.state = PacedWalkerState::Init;
    let context_eq = PacedWalkerContext {
        frame: FrameTick(1),
        mario_ref_x: 100,
    };
    let effect_eq = update(&mut actor, context_eq, &mut world);
    assert_eq!(actor.state, PacedWalkerState::WalkLeft);
    assert!(!actor.sprite_flip.horizontal());
    assert_eq!(
        effect_eq,
        PacedWalkerEffect::Initialized {
            facing: Ordering::Equal
        }
    );
}

#[test]
fn paced_walker_two_frame_toggle_cadence() {
    let mut actor = PacedWalker::default();
    actor.sprite = 10;
    let mut world = MockWorld {
        left_wall: false,
        right_wall: false,
    };

    // Frame 0 (multiple of 4) -> toggles
    update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(0),
            mario_ref_x: 0,
        },
        &mut world,
    );
    assert_eq!(actor.sprite, 11);

    // Frame 1 -> no toggle
    update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 0,
        },
        &mut world,
    );
    assert_eq!(actor.sprite, 11);

    // Frame 2 -> no toggle
    update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(2),
            mario_ref_x: 0,
        },
        &mut world,
    );
    assert_eq!(actor.sprite, 11);

    // Frame 3 -> no toggle
    update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(3),
            mario_ref_x: 0,
        },
        &mut world,
    );
    assert_eq!(actor.sprite, 11);

    // Frame 4 -> toggles back
    update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(4),
            mario_ref_x: 0,
        },
        &mut world,
    );
    assert_eq!(actor.sprite, 10);
}

#[test]
fn paced_walker_cadence_counts_down_then_steps_48_times() {
    let mut actor = PacedWalker::default();
    actor.position.x = 200;
    let mut world = MockWorld {
        left_wall: false,
        right_wall: false,
    };

    // Init: Mario to left -> WalkLeft, timer = 1
    update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 50,
        },
        &mut world,
    );
    assert_eq!(actor.pause_ticks, 1);
    assert_eq!(actor.position.x, 200);

    // Frame 1: timer 1 -> 0, arms budget to 48 (0x30), no movement
    let eff = update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 50,
        },
        &mut world,
    );
    assert_eq!(eff, PacedWalkerEffect::None);
    assert_eq!(actor.pause_ticks, 0);
    assert_eq!(actor.remaining_pixels, 48);
    assert_eq!(actor.position.x, 200);

    // Next 47 steps: each decrements budget and moves X by -1
    for i in 1..=47 {
        let eff = update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(
            eff,
            PacedWalkerEffect::Moved {
                state: PacedWalkerState::WalkLeft
            }
        );
        assert_eq!(actor.remaining_pixels, 48 - i);
        assert_eq!(actor.position.x, 200 - i as u16);
    }

    // 48th step: reaches budget 0, pauses with timer = 16 (0x10)
    let eff = update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 50,
        },
        &mut world,
    );
    assert_eq!(eff, PacedWalkerEffect::Paused);
    assert_eq!(actor.remaining_pixels, 0);
    assert_eq!(actor.position.x, 200 - 48);
    assert_eq!(actor.pause_ticks, 16);

    // Next 15 pause frames: timer decrements from 16 to 1, no movement
    for pause_step_idx in 1..=15 {
        let eff = update(
            &mut actor,
            PacedWalkerContext {
                frame: FrameTick(1),
                mario_ref_x: 50,
            },
            &mut world,
        );
        assert_eq!(eff, PacedWalkerEffect::None);
        assert_eq!(actor.pause_ticks, 16 - pause_step_idx);
        assert_eq!(actor.position.x, 200 - 48);
    }

    // 16th pause frame: timer 1 -> 0, reloads budget to 48, no movement
    let eff = update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 50,
        },
        &mut world,
    );
    assert_eq!(eff, PacedWalkerEffect::None);
    assert_eq!(actor.pause_ticks, 0);
    assert_eq!(actor.remaining_pixels, 48);
    assert_eq!(actor.position.x, 200 - 48);

    // Next frame: begins walking again
    let eff = update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 50,
        },
        &mut world,
    );
    assert_eq!(
        eff,
        PacedWalkerEffect::Moved {
            state: PacedWalkerState::WalkLeft
        }
    );
    assert_eq!(actor.position.x, 200 - 49);
}

#[test]
fn paced_walker_wall_flips_direction() {
    let mut actor = PacedWalker::default();
    actor.position.x = 100;
    actor.state = PacedWalkerState::WalkLeft;
    actor.sprite_flip.set_horizontal(false);

    let mut world = MockWorld {
        left_wall: true,
        right_wall: false,
    };

    // Left wall hit -> turn right
    let eff = update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 0,
        },
        &mut world,
    );
    assert_eq!(
        eff,
        PacedWalkerEffect::Turned {
            state: PacedWalkerState::WalkRight
        }
    );
    assert_eq!(actor.state, PacedWalkerState::WalkRight);
    assert!(actor.sprite_flip.horizontal());
    assert_eq!(actor.position.x, 100, "wall bounce does not step X");

    // Now facing right, right wall hit -> turn left
    world.left_wall = false;
    world.right_wall = true;
    let eff = update(
        &mut actor,
        PacedWalkerContext {
            frame: FrameTick(1),
            mario_ref_x: 0,
        },
        &mut world,
    );
    assert_eq!(
        eff,
        PacedWalkerEffect::Turned {
            state: PacedWalkerState::WalkLeft
        }
    );
    assert_eq!(actor.state, PacedWalkerState::WalkLeft);
    assert!(!actor.sprite_flip.horizontal());
    assert_eq!(actor.position.x, 100, "wall bounce does not step X");
}
