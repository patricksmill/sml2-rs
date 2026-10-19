//! Unit tests for native Wario drop child (anim 28, ROM `$0B0C2`).

use super::drop_child::{update_drop_child, DropChildContext, DropChildEffects, DropChildError};
use crate::actor::{Actor, FrameTick, SpriteFlipFlags};
use crate::objects::{ObjectSim, Staging};

const DUMMY_PICKER_PROFILE: [u8; 38] = [
    0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x03, 0x02, 0x03, 0x03, 0x03,
    0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02,
    0x03, 0x03, 0x02, 0x03, 0x03, 0x00,
];

#[test]
fn drop_child_state0_selects_horizontal_direction_from_flip() {
    let mut actor = Actor::default();
    actor.state = 0;
    actor.sprite_flip = SpriteFlipFlags::from_bits(0);

    let ctx = DropChildContext {
        frame: FrameTick(0),
        player_screen_x: 100,
        picker_profile: &DUMMY_PICKER_PROFILE,
    };

    let res = update_drop_child(&mut actor, ctx).expect("update");
    assert_eq!(res, DropChildEffects::default());
    assert_eq!(actor.state, 1, "facing left -> state 1");

    actor.state = 0;
    actor.sprite_flip = SpriteFlipFlags::from_bits(32);
    let res = update_drop_child(&mut actor, ctx).expect("update");
    assert_eq!(res, DropChildEffects::default());
    assert_eq!(actor.state, 2, "facing right -> state 2");
}

#[test]
fn drop_child_state1_moves_left_and_state2_moves_right() {
    let mut actor1 = Actor::default();
    actor1.state = 1;
    actor1.screen.x = 80;
    actor1.screen.y = 50;

    let mut actor2 = Actor::default();
    actor2.state = 2;
    actor2.screen.x = 80;
    actor2.screen.y = 50;

    let ctx = DropChildContext {
        frame: FrameTick(1), // frame & 3 != 0 -> no vertical / proximity branch
        player_screen_x: 0,
        picker_profile: &DUMMY_PICKER_PROFILE,
    };

    update_drop_child(&mut actor1, ctx).expect("state 1 update");
    update_drop_child(&mut actor2, ctx).expect("state 2 update");

    assert_eq!(actor1.screen.x, 78, "state 1 moves left by 2");
    assert_eq!(actor2.screen.x, 82, "state 2 moves right by 2");
}

#[test]
fn drop_child_proximity_triggers_state3_and_sfx0() {
    let mut actor = Actor::default();
    actor.state = 1;
    actor.screen.x = 55;
    actor.screen.y = 50;
    actor.motion_phase = 7;
    actor.motion_leg = 1;

    let ctx = DropChildContext {
        frame: FrameTick(4), // frame & 3 == 0
        player_screen_x: 50, // abs_diff = 53 - 50 = 3 < 8 (after x -= 2: x = 53)
        picker_profile: &DUMMY_PICKER_PROFILE,
    };

    let effects = update_drop_child(&mut actor, ctx).expect("update");
    assert_eq!(effects.sound0, Some(6), "sfxRequest0 = 6");
    assert_eq!(actor.state, 3, "transitions to state 3");
    assert_eq!(actor.motion_phase, 0, "resets motion_phase");
    assert_eq!(actor.motion_leg, 0, "resets motion_leg");
}

#[test]
fn drop_child_state1_vertical_bob_cycles_legs() {
    let mut actor = Actor::default();
    actor.state = 1;
    actor.screen.x = 100;
    actor.screen.y = 50;
    actor.motion_phase = 0;
    actor.motion_leg = 0;

    // Run 15 steps with player far away so dist_x >= 8
    for i in 0..15 {
        let ctx = DropChildContext {
            frame: FrameTick((i as u64) * 4), // frame & 3 == 0
            player_screen_x: 0,
            picker_profile: &DUMMY_PICKER_PROFILE,
        };
        update_drop_child(&mut actor, ctx).expect("update");
    }

    // After 15 steps of leg 0, phase wraps and switches to leg 1
    assert_eq!(actor.motion_leg, 1, "switched to leg 1");
    assert_eq!(actor.motion_phase, 0, "motion_phase reset to 0");
}

#[test]
fn drop_child_state3_rise_then_fall_to_ground_state4() {
    let mut actor = Actor::default();
    actor.state = 3;
    actor.screen.x = 50;
    actor.screen.y = 50;
    actor.motion_phase = 0;
    actor.motion_leg = 0;

    // Leg 0: 12 steps of RiseProfile::Short
    for i in 0..12 {
        let ctx = DropChildContext {
            frame: FrameTick(i),
            player_screen_x: 50,
            picker_profile: &DUMMY_PICKER_PROFILE,
        };
        update_drop_child(&mut actor, ctx).expect("rise step");
    }

    assert_eq!(actor.motion_leg, 1, "leg 0 completed, switched to leg 1");
    assert_eq!(actor.motion_phase, 0, "motion_phase reset to 0");

    // Leg 1: falls until screen.y >= 0x90 (144)
    actor.screen.y = 143;
    let ctx = DropChildContext {
        frame: FrameTick(12),
        player_screen_x: 50,
        picker_profile: &DUMMY_PICKER_PROFILE,
    };
    update_drop_child(&mut actor, ctx).expect("fall step");
    assert_eq!(
        actor.screen.y, 143,
        "first step added picker[0]=0 -> y=143 < 144"
    );

    actor.screen.y = 144;
    let ctx2 = DropChildContext {
        frame: FrameTick(13),
        player_screen_x: 50,
        picker_profile: &DUMMY_PICKER_PROFILE,
    };
    update_drop_child(&mut actor, ctx2).expect("ground landing step");
    assert_eq!(actor.state, 4, "landed on ground -> state 4");
    assert_eq!(actor.sprite, 108, "sprite set to 108");

    // State 4 is inert
    let ctx3 = DropChildContext {
        frame: FrameTick(14),
        player_screen_x: 50,
        picker_profile: &DUMMY_PICKER_PROFILE,
    };
    let res = update_drop_child(&mut actor, ctx3).expect("state 4");
    assert_eq!(res, DropChildEffects::default());
    assert_eq!(actor.state, 4);
    assert_eq!(actor.sprite, 108);
}

#[test]
fn drop_child_unsupported_state_errors() {
    let mut actor = Actor::default();
    actor.state = 5;
    let ctx = DropChildContext {
        frame: FrameTick(0),
        player_screen_x: 0,
        picker_profile: &DUMMY_PICKER_PROFILE,
    };
    assert_eq!(
        update_drop_child(&mut actor, ctx),
        Err(DropChildError::UnsupportedState(5))
    );
}

fn tables() -> crate::objects::ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn drop_child_adapter_dispatches_and_produces_sfx0() {
    let mut sim = ObjectSim::new(Vec::new(), 0);
    sim.player_screen_x = 50;
    sim.fff0 = 4; // frame tick 4 (frame & 3 == 0)

    let tables = tables();

    let mut st = Staging::default();
    st.b[15] = 1; // state = 1
    st.b[10] = 55; // screen.x = 55 (abs_diff to player_screen_x 50 is 5 < 8 after move_x(-2))

    sim.handler_wario28(&mut st, &tables);

    assert_eq!(sim.sfx0, 6, "emitted sfx0 = 6 on proximity latch");
    assert_eq!(st.b[15], 3, "actor transitioned to state 3");
}
