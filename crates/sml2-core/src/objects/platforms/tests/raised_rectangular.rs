use super::*;
use crate::actor::{Actor, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
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
fn check(sim: &mut ObjectSim, st: &mut Staging, expected: [u8; 32], facing: u8) {
    sim.handler_0x58e7e(st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, facing);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58e7e_init_rises_32_faces_and_arms_the_legs() {
    for (target_x, state, flip, facing) in
        [(200u16, 2u8, 0x20u8, 1u8), (50, 1, 0, 255), (100, 1, 0, 0)]
    {
        let (mut sim, mut st) = fixture(0);
        position(&mut st.b, 100, 100);
        st.b[13] = 0;
        st.b[14] = 0;
        st.b[16] = 0;
        st.b[24] = 0;
        st.b[25] = 0;
        sim.mario_ref_x = target_x;
        let mut expected = st.b;
        position(&mut expected, 100, 68);
        expected[13] = flip;
        expected[15] = state;
        expected[16] = 80;
        expected[18] = 32;
        expected[24] = 80;
        expected[25] = 40;
        check(&mut sim, &mut st, expected, facing);
        assert_eq!(sim.mario_x_adjust, None);
        assert_eq!(sim.mario_y_adjust, None);
    }
}

#[test]
fn handler_0x58e7e_first_horizontal_leg_counts_to_40_and_reloads_80() {
    for (state, dx) in [(1u8, -1i16), (2, 1)] {
        for (span, transitions) in [(0u8, false), (1, true), (2, false)] {
            let (mut sim, mut st) = fixture(state);
            position(&mut st.b, 100, 100);
            st.b[16] = 80;
            st.b[24] = 80;
            st.b[25] = span;
            st.b[23] = 0;
            let mut expected = st.b;
            position(&mut expected, (100i16 + dx) as u16, 100);
            expected[25] = if transitions {
                80
            } else {
                span.wrapping_sub(1)
            };
            if transitions {
                expected[15] = 3;
            }
            check(&mut sim, &mut st, expected, 99);
        }
    }
}

#[test]
fn handler_0x58e7e_vertical_leg_counts_to_80_and_selects_by_flip() {
    for (flip, next_state) in [(0u8, 4u8), (0x20, 6)] {
        for (leg, transitions) in [(0u8, false), (1, true), (2, false)] {
            let (mut sim, mut st) = fixture(3);
            position(&mut st.b, 100, 100);
            st.b[13] = flip;
            st.b[16] = 80;
            st.b[24] = leg;
            st.b[25] = 40;
            st.b[23] = 0;
            let mut expected = st.b;
            position(&mut expected, 100, 101);
            expected[24] = if transitions { 80 } else { leg.wrapping_sub(1) };
            if transitions {
                expected[15] = next_state;
            }
            check(&mut sim, &mut st, expected, 99);
        }
    }
}

#[test]
fn handler_0x58e7e_horizontal_corners_select_by_flip() {
    for (state, dx, flip, next_state) in [
        (4u8, 1i16, 0u8, 5u8),
        (4, 1, 0x20, 3),
        (6, -1, 0, 3),
        (6, -1, 0x20, 5),
    ] {
        let (mut sim, mut st) = fixture(state);
        position(&mut st.b, 100, 100);
        st.b[13] = flip;
        st.b[16] = 80;
        st.b[24] = 80;
        st.b[25] = 1;
        st.b[23] = 0;
        let mut expected = st.b;
        position(&mut expected, (100i16 + dx) as u16, 100);
        expected[15] = next_state;
        expected[25] = 80;
        check(&mut sim, &mut st, expected, 99);
    }
}

#[test]
fn handler_0x58e7e_up_leg_selects_by_flip() {
    for (flip, next_state) in [(0u8, 6u8), (0x20, 4)] {
        let (mut sim, mut st) = fixture(5);
        position(&mut st.b, 100, 100);
        st.b[13] = flip;
        st.b[16] = 80;
        st.b[24] = 1;
        st.b[25] = 40;
        st.b[23] = 0;
        let mut expected = st.b;
        position(&mut expected, 100, 99);
        expected[15] = next_state;
        expected[24] = 80;
        check(&mut sim, &mut st, expected, 99);
    }
}

#[test]
fn handler_0x58e7e_carries_a_stood_on_rider_on_both_axes() {
    for (state, dx, dy) in [
        (1u8, -1i16, 0i16),
        (2, 1, 0),
        (3, 0, 1),
        (4, 1, 0),
        (5, 0, -1),
        (6, -1, 0),
    ] {
        let (mut sim, mut st) = fixture(state);
        position(&mut st.b, 100, 100);
        st.b[16] = 80;
        st.b[23] = 0x08;
        st.b[24] = 2;
        st.b[25] = 2;
        sim.mario.x = 100;
        sim.mario.y = 100;
        sim.handler_0x58e7e(&mut st);
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
fn handler_0x58e7e_horizontal_carry_publishes_direction_gated_scroll() {
    // `$3CC0`: right carry adds scroll when hvel >= $80.
    // `$3CF1`: left carry adds scroll when hvel < $81.
    for (state, hvel, x, right, left) in [
        (1, 0x80, 99u16, 0, 1),
        (1, 0x81, 99, 0, 0),
        (6, 0x80, 99, 0, 1),
        (6, 0x81, 99, 0, 0),
        (2, 0x7F, 101, 0, 0),
        (2, 0x80, 101, 1, 0),
        (4, 0x7F, 101, 0, 0),
        (4, 0x80, 101, 1, 0),
    ] {
        let (mut sim, mut st) = fixture(state);
        position(&mut st.b, 100, 100);
        st.b[23] = 8;
        st.b[25] = 2;
        sim.mario.x = 100;
        sim.mario.hvel = hvel;
        let mut expected = st.b;
        expected[0..2].copy_from_slice(&x.to_be_bytes());
        expected[25] = 1;
        sim.handler_0x58e7e(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!(sim.mario_x_adjust.unwrap().to_be_bytes(), x.to_be_bytes());
        assert_eq!(
            [sim.scroll_delta_right, sim.scroll_delta_left],
            [right, left]
        );
    }
}

#[test]
fn handler_0x58e7e_carry_reaches_the_next_camera_step() {
    use crate::{camera::CameraSim, player_sim::PlayerSim};

    let (mut objects, mut st) = fixture(2);
    position(&mut st.b, 332, 720);
    st.b[23] = 8;
    st.b[25] = 2;
    objects.mario.x = 331;
    objects.mario.y = 677;
    objects.mario.hvel = 0x80;
    objects.handler_0x58e7e(&mut st);
    let mut player = PlayerSim::new();
    player.x = 331;
    player.y = 677;
    player.apply_object_result(&mut objects, false);
    assert_eq!(player.x.to_le_bytes(), [0x4C, 1]);

    let mut camera = CameraSim::new(331, 691);
    camera.right_enabled = true;
    camera.edge_now = Some(0);
    camera.apply_object_result(&mut objects);
    assert_eq!(
        [objects.scroll_delta_right, camera.scroll_delta_right],
        [0, 1]
    );
    camera.step(&mut player);
    // `$0B90`: inherited delta 1 plus the one-pixel centering correction.
    assert_eq!(camera.x.to_le_bytes(), [0x4D, 1]);
    assert_eq!(camera.scroll_delta_right, 0);
}

#[test]
fn handler_0x58e7e_invalid_states_report_the_entry() {
    for state in 7..=255 {
        let (mut sim, mut st) = fixture(state);
        let original = st.b;
        sim.handler_0x58e7e(&mut st);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x58E7E]);
    }
}

#[test]
fn handler_0x58e7e_real_anim48_step_dispatches() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 48).unwrap().handler, 0x58E7E);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 48;
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
    assert_eq!(sim.slots[1].b[24], 80);
    assert_eq!(sim.slots[1].b[25], 40);
    assert!(sim.unported.is_empty());
}
