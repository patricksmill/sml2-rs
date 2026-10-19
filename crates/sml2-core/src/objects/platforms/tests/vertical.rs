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
    sim.handler_0x58bba(st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, facing);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58bba_init_faces_by_y_and_sets_span_44() {
    for (player_y, state, facing) in [(200u16, 2u8, 1u8), (50, 1, 255), (100, 1, 0)] {
        let (mut sim, mut st) = fixture(0);
        position(&mut st.b, 100, 100);
        st.b[16] = 7; // the init does not reset the phase
        st.b[18] = 0;
        sim.mario_ref_y = player_y;
        let mut expected = st.b;
        expected[15] = state;
        expected[18] = 32;
        expected[24] = 44;
        check(&mut sim, &mut st, expected, facing);
        assert_eq!(st.b[16], 7);
        assert_eq!(sim.mario_y_adjust, None);
    }
}

#[test]
fn handler_0x58bba_short_legs_move_and_flip_to_the_long_legs() {
    for (state, dy, cap, next_state) in [(1u8, -1i16, 44u8, 4u8), (2, 1, 44, 3)] {
        for (phase, transitions) in [(0u8, false), (cap - 1, true), (cap, false)] {
            let (mut sim, mut st) = fixture(state);
            position(&mut st.b, 100, 100);
            st.b[16] = phase;
            st.b[23] = 0;
            st.b[24] = 44;
            let mut expected = st.b;
            position(&mut expected, 100, (100i16 + dy) as u16);
            expected[16] = if transitions {
                0
            } else {
                phase.wrapping_add(1)
            };
            if transitions {
                expected[15] = next_state;
            }
            check(&mut sim, &mut st, expected, 99);
            assert_eq!(sim.mario_y_adjust, None);
        }
    }
}

#[test]
fn handler_0x58bba_long_legs_use_double_span_and_toggle() {
    for (state, dy, next_state) in [(3u8, -1i16, 4u8), (4, 1, 3)] {
        for (phase, transitions) in [(0u8, false), (87, true), (88, false)] {
            let (mut sim, mut st) = fixture(state);
            position(&mut st.b, 100, 100);
            st.b[16] = phase;
            st.b[23] = 0;
            st.b[24] = 44;
            let mut expected = st.b;
            position(&mut expected, 100, (100i16 + dy) as u16);
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
fn handler_0x58bba_carries_a_stood_on_rider_only() {
    for (state, dy) in [(1u8, -1i16), (2, 1)] {
        for (flags, carry) in [(0u8, None), (0x08, Some(dy))] {
            let (mut sim, mut st) = fixture(state);
            position(&mut st.b, 100, 100);
            st.b[16] = 0;
            st.b[23] = flags;
            sim.mario.y = 100;
            sim.handler_0x58bba(&mut st);
            assert_eq!(sim.mario_y_adjust, carry.map(|d| (100i16 + d) as u16));
            assert!(sim.unported.is_empty());
        }
    }

    // $3D10/$3CDF accumulate on an existing rider adjustment.
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 100, 100);
    st.b[16] = 0;
    st.b[23] = 0x08;
    sim.mario_y_adjust = Some(5);
    sim.handler_0x58bba(&mut st);
    assert_eq!(sim.mario_y_adjust, Some(4));
}

#[test]
fn handler_0x58bba_invalid_states_report_the_entry() {
    for state in 5..=255 {
        let (mut sim, mut st) = fixture(state);
        let original = st.b;
        sim.handler_0x58bba(&mut st);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x58BBA]);
    }
}

#[test]
fn handler_0x58bba_real_anim41_step_and_sibling_anim40_stay_distinct() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 41).unwrap().handler, 0x58BBA);
    assert_eq!(tables.profile_for(0, 40).unwrap().handler, 0x58BCB);
    for (anim, span) in [(41u8, 44u8), (40, 20)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 3;
        sim.phase = false;
        let mut actor = Actor::default();
        actor.position = Position { x: 100, y: 50 };
        actor.activity = 2;
        actor.definition = anim;
        actor.state = 0;
        sim.slots[1].b = actor.to_raw_slot();
        let mario = MarioContact {
            x: 100,
            y: 100,
            ..Default::default()
        };
        sim.step(100, 100, &layout, &tables, &mario, false);
        let result = &sim.slots[1].b;
        assert_eq!(result[15], 2, "anim {anim}");
        assert_eq!(result[18], 32, "anim {anim}");
        assert_eq!(result[24], span, "anim {anim}");
        assert!(sim.unported.is_empty());
    }
}
