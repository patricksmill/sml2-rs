use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn dirty(state: u8, flip: u8) -> Staging {
    let mut b = std::array::from_fn(|i| (i as u8).wrapping_mul(29).wrapping_add(0x81));
    b[15] = state;
    b[13] = flip;
    Staging {
        b,
        ..Default::default()
    }
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5a816_init_sets_type_and_selects_the_course_from_flip_bits() {
    for (flip, state, expected_flip) in [
        (0x00u8, 3u8, 0x00u8),
        (0x20, 4, 0x20),
        (0x01, 1, 0x00),
        (0x21, 2, 0x20),
        (0x40, 3, 0x40),
        (0x60, 4, 0x60),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = dirty(0, flip);
        let before = st.b;
        sim.handler_0x5a816(&mut st);
        let mut expected = before;
        expected[13] = expected_flip;
        expected[15] = state;
        expected[18] = 73;
        assert_eq!(st.b, expected, "flip={flip:#04x}");
    }
}

#[test]
fn handler_0x5a816_diagonal_states_move_up_one_and_sideways() {
    for (state, x, want_x) in [
        (1u8, 0x0064u16, 0x0063u16),
        (2, 0x0064, 0x0065),
        (1, 0x0000, 0xFFFF),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = dirty(state, 0x20);
        position(&mut st.b, x, 0x0064);
        let before = st.b;
        sim.handler_0x5a816(&mut st);
        let mut expected = before;
        expected[0..2].copy_from_slice(&want_x.to_be_bytes());
        expected[3..5].copy_from_slice(&0x0063u16.to_be_bytes());
        assert_eq!(st.b, expected, "state={state}, x={x:#06x}");
    }
}

#[test]
fn handler_0x5a816_straight_states_move_two_without_y() {
    for (state, x, expected) in [(3u8, 0x0001u16, 0xFFFFu16), (4, 0xFFFE, 0x0000)] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = dirty(state, 0x00);
        position(&mut st.b, x, 0x0050);
        let before = st.b;
        sim.handler_0x5a816(&mut st);
        let mut expected_slot = before;
        expected_slot[0..2].copy_from_slice(&expected.to_be_bytes());
        assert_eq!(st.b, expected_slot, "state={state}, x={x:#06x}");
    }
}

#[test]
fn handler_0x5a816_unknown_state_reports_without_writes() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = dirty(5, 0x20);
    let before = st.b;
    sim.handler_0x5a816(&mut st);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A816]);
}

#[test]
fn handler_0x5a816_real_mode_b_anim78_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 78).unwrap().handler, 0x5A816);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position {
        x: 0x0090,
        y: 0x0090,
    };
    actor.activity = 2;
    actor.definition = 78;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        0x90,
        0x0090,
        &layout,
        &tables,
        &MarioContact::default(),
        false,
    );
    assert_eq!(sim.slots[1].b[15], 3);
    assert_eq!(sim.slots[1].b[18], 73);
    assert_eq!(
        u16::from_be_bytes([sim.slots[1].b[0], sim.slots[1].b[1]]),
        0x0090
    );
    assert!(sim.unported.is_empty());
}
