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
    sim.handler_0x58c52(st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, facing);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58c52_init_faces_by_x_and_sets_span_36() {
    for (target_x, state, flip, facing) in
        [(200u16, 2u8, 0x20u8, 1u8), (50, 1, 0, 255), (100, 1, 0, 0)]
    {
        let (mut sim, mut st) = fixture(0);
        sim.fff0 = 3; // no $5B51A toggle
        position(&mut st.b, 100, 100);
        st.b[13] = 0;
        st.b[14] = 0;
        sim.mario_ref_x = target_x;
        let mut expected = st.b;
        expected[13] = flip;
        expected[14] = 16;
        expected[15] = state;
        expected[18] = 32;
        expected[24] = 36;
        check(&mut sim, &mut st, expected, facing);
        assert_eq!(sim.mario_x_adjust, None);
    }
}

#[test]
fn handler_0x58c52_sprite_toggles_every_fourth_frame() {
    for (frame, toggled) in [
        (0u8, true),
        (1, false),
        (2, false),
        (3, false),
        (4, true),
        (6, false),
        (255, false),
    ] {
        let (mut sim, mut st) = fixture(0);
        sim.fff0 = frame;
        position(&mut st.b, 100, 100);
        st.b[11] = 0x20;
        st.b[13] = 0;
        st.b[14] = 0;
        sim.mario_ref_x = 200;
        let mut expected = st.b;
        expected[13] = 0x20;
        expected[14] = 16;
        expected[15] = 2;
        expected[18] = 32;
        expected[24] = 36;
        if toggled {
            expected[11] ^= 1;
        }
        check(&mut sim, &mut st, expected, 1);
    }
}

#[test]
fn handler_0x58c52_short_legs_move_and_flip_to_the_long_legs() {
    for (state, dx, cap, next_state) in [(1u8, -1i16, 36u8, 4u8), (2, 1, 36, 3)] {
        for (phase, transitions) in [(0u8, false), (cap - 1, true), (cap, false)] {
            let (mut sim, mut st) = fixture(state);
            sim.fff0 = 3;
            position(&mut st.b, 100, 100);
            st.b[16] = phase;
            st.b[23] = 0;
            st.b[24] = 36;
            let mut expected = st.b;
            position(&mut expected, (100i16 + dx) as u16, 100);
            expected[16] = if transitions {
                0
            } else {
                phase.wrapping_add(1)
            };
            if transitions {
                expected[15] = next_state;
            }
            check(&mut sim, &mut st, expected, 99);
            assert_eq!(sim.mario_x_adjust, None);
        }
    }
}

#[test]
fn handler_0x58c52_long_legs_use_double_span_and_toggle() {
    for (state, dx, next_state) in [(3u8, -1i16, 4u8), (4, 1, 3)] {
        for (phase, transitions) in [(0u8, false), (71, true), (72, false)] {
            let (mut sim, mut st) = fixture(state);
            sim.fff0 = 3;
            position(&mut st.b, 100, 100);
            st.b[16] = phase;
            st.b[23] = 0;
            st.b[24] = 36;
            let mut expected = st.b;
            position(&mut expected, (100i16 + dx) as u16, 100);
            expected[16] = if transitions {
                0
            } else {
                phase.wrapping_add(1)
            };
            if transitions {
                expected[15] = next_state;
            }
            check(&mut sim, &mut st, expected, 99);
        }
    }
}

#[test]
fn handler_0x58c52_carries_a_stood_on_rider_only() {
    for (state, dx) in [(1u8, -1i16), (2, 1)] {
        for (flags, carry) in [(0u8, None), (0x08, Some(dx))] {
            let (mut sim, mut st) = fixture(state);
            sim.fff0 = 3;
            position(&mut st.b, 100, 100);
            st.b[16] = 0;
            st.b[23] = flags;
            st.b[24] = 36;
            sim.mario.x = 100;
            sim.handler_0x58c52(&mut st);
            assert_eq!(sim.mario_x_adjust, carry.map(|d| (100i16 + d) as u16));
            assert!(sim.unported.is_empty());
        }
    }
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[16] = 0;
    st.b[23] = 0x08;
    st.b[24] = 36;
    sim.mario_x_adjust = Some(5);
    sim.handler_0x58c52(&mut st);
    assert_eq!(sim.mario_x_adjust, Some(4));
}

#[test]
fn handler_0x58c52_invalid_states_report_the_entry() {
    for state in 5..=255 {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 3;
        let original = st.b;
        sim.handler_0x58c52(&mut st);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x58C52]);
    }
}

#[test]
fn handler_0x58c52_real_anim56_step_dispatches() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 56).unwrap().handler, 0x58C52);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 56;
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
    assert_eq!(sim.slots[1].b[24], 36);
    assert!(sim.unported.is_empty());
}
