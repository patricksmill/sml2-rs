use super::*;
use crate::actor::{Actor, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    sim.move_flags = 0x97;
    sim.fff0 = 3; // no $5B511 toggle on the checked frame
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}
fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}
fn solid() -> LevelLayout {
    LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap()
}
fn air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
}
/// Tile 64: below the ground threshold 72, above the wall/ceiling threshold 56.
fn floor() -> LevelLayout {
    LevelLayout::from_bytes(&[0x40; 0x3000]).unwrap()
}
fn check(
    sim: &mut ObjectSim,
    st: &mut Staging,
    layout: &LevelLayout,
    expected: [u8; 32],
    facing: u8,
) {
    sim.handler_0x59431(st, layout);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, facing);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59431_fall_body_lands_and_faces_mario() {
    let layout = solid();
    for (target_x, state, flip, facing) in
        [(200u16, 2u8, 0x20u8, 1u8), (50, 1, 0, 255), (100, 1, 0, 0)]
    {
        let (mut sim, mut st) = fixture(0);
        position(&mut st.b, 100, 96);
        st.b[16] = 7; // phase is reset on landing
        sim.mario_ref_x = target_x;
        let mut expected = st.b;
        expected[15] = state;
        expected[13] = flip;
        expected[16] = 0;
        expected[18] = 77;
        check(&mut sim, &mut st, &layout, expected, facing);
    }
}

#[test]
fn handler_0x5945a_fall_body_sets_contact_72_and_flash_16() {
    let layout = solid();
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 100, 96);
    sim.mario_ref_x = 200;
    let mut expected = st.b;
    expected[13] = 0x20;
    expected[14] = 16;
    expected[15] = 2;
    expected[16] = 0;
    expected[18] = 72;
    sim.handler_0x5945a(&mut st, &layout);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59431_fall_body_moves_on_the_shared_curve() {
    let layout = air();
    for (phase, dy, next_phase) in [(0u8, 0u8, 1u8), (5, 2, 6), (21, 3, 21)] {
        let (mut sim, mut st) = fixture(0);
        position(&mut st.b, 100, 100);
        st.b[16] = phase;
        st.b[11] = 0x20;
        sim.mario_ref_x = 200;
        let mut expected = st.b;
        position(&mut expected, 100, 100 + u16::from(dy));
        expected[16] = next_phase;
        check(&mut sim, &mut st, &layout, expected, 99);
    }
}

#[test]
fn handler_0x59431_fall_body_advanced_profile_is_diagnostic() {
    let layout = air();
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 100, 100);
    st.b[16] = 22;
    let original = st.b;
    sim.handler_0x59431(&mut st, &layout);
    assert_eq!(st.b, original);
    assert_eq!(sim.unported, vec![0x59431]);
}

#[test]
fn handler_0x59431_patrol_even_frames_move_or_turn() {
    for (state, wall, x, next_state, flip) in [
        (1u8, false, 99u16, 1u8, 0u8),
        (1, true, 99, 2, 0x20),
        (2, false, 101, 2, 0),
        (2, true, 101, 1, 0x20),
    ] {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 2; // even frame, no $5B511 toggle
        position(&mut st.b, 100, 100);
        st.b[11] = 0x20;
        st.b[13] = 0;
        let layout = if wall { solid() } else { floor() };
        let mut expected = st.b;
        position(&mut expected, x, 100);
        expected[15] = next_state;
        expected[13] = flip;
        check(&mut sim, &mut st, &layout, expected, 99);
    }
}

#[test]
fn handler_0x59431_patrol_odd_frames_reverse_at_a_ledge() {
    for (state, ground, next_state, flip) in [
        (1u8, false, 2u8, 0x20u8),
        (1, true, 1, 0),
        (2, false, 1, 0x20),
        (2, true, 2, 0),
    ] {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 1; // odd frame: ground check instead of movement
        position(&mut st.b, 100, 96);
        st.b[13] = 0;
        let layout = if ground { floor() } else { air() };
        let mut expected = st.b;
        expected[15] = next_state;
        expected[13] = flip;
        check(&mut sim, &mut st, &layout, expected, 99);
    }
}

#[test]
fn handler_0x59431_sprite_toggle_runs_every_eighth_frame() {
    for (frame, toggled) in [
        (0u8, true),
        (1, false),
        (4, false),
        (7, false),
        (8, true),
        (12, false),
        (255, false),
    ] {
        let (mut sim, mut st) = fixture(1);
        sim.fff0 = frame;
        position(&mut st.b, 100, 96);
        st.b[11] = 0x20;
        st.b[13] = 0;
        let layout = floor();
        let mut expected = st.b;
        position(&mut expected, if frame % 2 == 0 { 99 } else { 100 }, 96);
        if toggled {
            expected[11] ^= 1;
        }
        check(&mut sim, &mut st, &layout, expected, 99);
    }
}

#[test]
fn handler_0x59431_and_0x5945a_real_anim_steps_stay_distinct() {
    let t = tables();
    let layout = solid();
    assert_eq!(t.profile_for(0, 65).unwrap().handler, 0x59431);
    assert_eq!(t.profile_for(0, 66).unwrap().handler, 0x5945A);
    for (anim, contact, flash) in [(65u8, 77u8, 0u8), (66, 72, 16)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 3;
        sim.phase = false;
        let mut actor = Actor::default();
        actor.position = Position { x: 100, y: 96 };
        actor.activity = 2;
        actor.definition = anim;
        actor.state = 0;
        sim.slots[1].b = actor.to_raw_slot();
        let mario = MarioContact {
            x: 1000,
            ..Default::default()
        };
        sim.step(100, 100, &layout, &t, &mario, false);
        let result = &sim.slots[1].b;
        assert_eq!(result[15], 2, "anim {anim}");
        assert_eq!(result[18], contact, "anim {anim}");
        assert_eq!(result[14], flash, "anim {anim}");
        assert!(sim.unported.is_empty());
    }
}

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}
