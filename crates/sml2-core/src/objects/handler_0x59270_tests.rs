use super::*;
use crate::actor::{Actor, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.b[26] = 0; // valid $5B5B6 animation frame
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}
fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}
fn sprite_cycle(slot: &mut [u8; 32], frame: u8) {
    if frame % 2 != 0 {
        return;
    }
    let next = (slot[26] + 1) % 3;
    slot[26] = next;
    slot[11] = slot[11].wrapping_add([0xFE, 1, 1][usize::from(next)]);
}
fn check(sim: &mut ObjectSim, st: &mut Staging, expected: [u8; 32], facing: u8) {
    sim.handler_0x59270(st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, facing);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59270_init_faces_by_x_and_sets_leg_20() {
    for (target_x, state, flip, facing) in
        [(200u16, 2u8, 0x20u8, 1u8), (50, 1, 0, 255), (100, 1, 0, 0)]
    {
        let (mut sim, mut st) = fixture(0);
        sim.fff0 = 3;
        position(&mut st.b, 100, 100);
        st.b[13] = 0;
        st.b[14] = 0;
        st.b[24] = 0;
        sim.mario_ref_x = target_x;
        let mut expected = st.b;
        expected[13] = flip;
        expected[14] = 16;
        expected[15] = state;
        expected[18] = 32;
        expected[24] = 20;
        check(&mut sim, &mut st, expected, facing);
    }
}

#[test]
fn handler_0x59270_left_and_right_legs_gate_the_vertical_step_to_even_frames() {
    for (state, dx, next_state) in [(1u8, -1i16, 3u8), (2, 1, 5)] {
        // Odd frame: only X and its carry.
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 3;
        position(&mut st.b, 100, 100);
        st.b[16] = 0;
        st.b[24] = 20;
        st.b[23] = 0;
        let mut expected = st.b;
        position(&mut expected, (100i16 + dx) as u16, 100);
        expected[16] = 0;
        sprite_cycle(&mut expected, 3);
        check(&mut sim, &mut st, expected, 99);

        // Even frame: X, Y, and the counter; at 19 the counter reaches 20.
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 2;
        position(&mut st.b, 100, 100);
        st.b[16] = 19;
        st.b[24] = 20;
        st.b[23] = 0;
        let mut expected = st.b;
        position(&mut expected, (100i16 + dx) as u16, 101);
        expected[15] = next_state;
        expected[16] = 0;
        sprite_cycle(&mut expected, 2);
        check(&mut sim, &mut st, expected, 99);
    }
}

#[test]
fn handler_0x59270_rise_uses_double_leg_and_toggles_on_even_states() {
    for (state, next_state, flip) in [(3u8, 4u8, 0x20u8), (5, 6, 0x20)] {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 3;
        position(&mut st.b, 100, 100);
        st.b[13] = 0;
        st.b[16] = 39;
        st.b[24] = 20;
        st.b[23] = 0;
        let mut expected = st.b;
        position(&mut expected, 100, 99);
        expected[13] = flip;
        expected[15] = next_state;
        expected[16] = 0;
        sprite_cycle(&mut expected, 3);
        check(&mut sim, &mut st, expected, 99);
    }
}

#[test]
fn handler_0x59270_down_legs_advance_to_odd_states_without_flipping() {
    for (state, dx, next_state) in [(4u8, 1i16, 5u8), (6, -1, 3)] {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 2;
        position(&mut st.b, 100, 100);
        st.b[13] = 0x20;
        st.b[16] = 39;
        st.b[24] = 20;
        st.b[23] = 0;
        let mut expected = st.b;
        position(&mut expected, (100i16 + dx) as u16, 101);
        expected[13] = 0x20; // the new state is odd, so no flip toggle
        expected[15] = next_state;
        expected[16] = 0;
        sprite_cycle(&mut expected, 2);
        check(&mut sim, &mut st, expected, 99);
    }
}

#[test]
fn handler_0x59270_carries_a_stood_on_rider_on_both_axes() {
    for (state, dx, dy) in [
        (1u8, -1i16, 1i16),
        (2, 1, 1),
        (3, 0, -1),
        (4, 1, 1),
        (5, 0, -1),
        (6, -1, 1),
    ] {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 2; // even: both axes move on the down legs
        position(&mut st.b, 100, 100);
        st.b[16] = 0;
        st.b[23] = 0x08;
        st.b[24] = 20;
        sim.mario.x = 100;
        sim.mario.y = 100;
        sim.handler_0x59270(&mut st);
        assert_eq!(
            sim.mario_x_adjust,
            (dx != 0).then_some((100i16 + dx) as u16)
        );
        assert_eq!(
            sim.mario_y_adjust,
            (dy != 0).then_some((100i16 + dy) as u16)
        );
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x59270_invalid_states_report_the_entry() {
    for state in 7..=255 {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 3;
        let original = st.b;
        sim.handler_0x59270(&mut st);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x59270]);
    }
}

#[test]
fn handler_0x59270_real_anim61_step_dispatches() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 61).unwrap().handler, 0x59270);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 61;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 200,
        y: 100,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[24], 20);
    assert!(sim.unported.is_empty());
}
