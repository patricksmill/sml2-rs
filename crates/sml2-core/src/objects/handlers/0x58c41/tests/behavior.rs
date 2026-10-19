use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.b[23] &= 0x0F;
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

fn check(sim: &mut ObjectSim, st: &mut Staging, expected: [u8; 32], facing: u8) {
    sim.handler_0x58c41(st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, facing);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58c41_init_faces_by_x_and_sets_span_60() {
    for (target_x, state, flip, facing) in
        [(200u16, 2u8, 0x20u8, 1u8), (50, 1, 0, 255), (100, 1, 0, 0)]
    {
        let (mut sim, mut st) = fixture(0);
        position(&mut st.b, 100, 100);
        st.b[13] = 0;
        st.b[14] = 0;
        sim.mario_ref_x = target_x;
        let mut expected = st.b;
        expected[13] = flip;
        expected[15] = state;
        expected[18] = 32;
        expected[27] = 60;
        check(&mut sim, &mut st, expected, facing);
        assert_eq!(sim.mario_x_adjust, None);
    }
}

#[test]
fn handler_0x58c41_short_legs_move_and_flip_to_the_long_legs() {
    for (state, dx, cap, next_state) in [(1u8, -1i16, 60u8, 4u8), (2, 1, 60, 3)] {
        for (phase, transitions) in [(0u8, false), (cap - 1, true), (cap, false)] {
            let (mut sim, mut st) = fixture(state);
            position(&mut st.b, 100, 100);
            st.b[16] = phase;
            st.b[27] = 60;
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
fn handler_0x58c41_long_legs_use_double_span_and_toggle() {
    for (state, dx, next) in [(3u8, -1i16, 4u8), (4, 1, 3)] {
        let (mut sim, mut st) = fixture(state);
        position(&mut st.b, 100, 100);
        st.b[16] = 119;
        st.b[27] = 60;
        let mut expected = st.b;
        position(&mut expected, (100i16 + dx) as u16, 100);
        expected[16] = 0;
        expected[15] = next;
        check(&mut sim, &mut st, expected, 99);
    }
}

#[test]
fn handler_0x58c41_carries_a_stood_on_rider_only() {
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 100, 100);
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[27] = 60;
    sim.mario.x = 200;
    sim.handler_0x58c41(&mut st);
    assert_eq!(sim.mario_x_adjust, Some(199));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58c41_real_anim43_mode_a_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(5, 43).unwrap().handler, 0x58C41);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(5);
    sim.mario_ref_x = 200;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 43;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 1000,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[27], 60);
    assert_eq!(sim.slots[1].b[15], 2);
    assert!(sim.unported.is_empty());
}

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}
