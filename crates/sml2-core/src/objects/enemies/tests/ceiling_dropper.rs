use super::*;
use crate::actor::{Actor, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    sim.move_flags = 0x97;
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

#[test]
fn handler_0x593da_trigger_arms_and_uses_the_eight_bit_distance() {
    let layout = solid();
    for (object_x, mario_ref_x, next_state) in [
        (0x0100u16, 0x0120u16, 1u8), // 0x20 < 0x30
        (0x0100, 0x0130, 0),         // 0x30 is not < 0x30
        (0x0110, 0x00F0, 1),         // 0xE0 wraps to 0x20
        (0x0100, 0x0100, 1),         // zero distance
    ] {
        let (mut sim, mut st) = fixture(0);
        position(&mut st.b, object_x, 100);
        sim.mario_ref_x = mario_ref_x;
        let mut expected = st.b;
        expected[14] = 16;
        expected[15] = next_state;
        expected[18] = 72;
        sim.handler_0x593da(&mut st, &layout);
        assert_eq!(st.b, expected, "x={object_x:#06x} mario={mario_ref_x:#06x}");
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x593da_fall_state_moves_and_lands_into_the_rise() {
    let air = air();
    for (phase, dy, next_phase) in [(0u8, 1u8, 1u8), (5, 3, 6), (21, 4, 21)] {
        let (mut sim, mut st) = fixture(1);
        sim.fff0 = 7; // every-frame $5B518
        position(&mut st.b, 100, 100);
        st.b[16] = phase;
        let mut expected = st.b;
        position(&mut expected, 100, 100 + u16::from(dy));
        expected[11] ^= 1;
        expected[16] = next_phase;
        sim.handler_0x593da(&mut st, &air);
        assert_eq!(st.b, expected);
        assert!(sim.unported.is_empty());
    }

    let solid = solid();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 7;
    position(&mut st.b, 100, 96);
    st.b[16] = 5;
    let mut expected = st.b;
    expected[11] ^= 1;
    expected[15] = 2;
    expected[16] = 0;
    sim.handler_0x593da(&mut st, &solid);
    assert_eq!(st.b, expected);
    assert_eq!(sim.sfx1, 10);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x593da_rise_stops_on_the_ceiling_probe() {
    let solid = solid();
    for (frame, toggled) in [(0u8, true), (1, false), (3, false), (4, true)] {
        let (mut sim, mut st) = fixture(2);
        sim.fff0 = frame;
        position(&mut st.b, 100, 100);
        let mut expected = st.b;
        if toggled {
            expected[11] ^= 1;
        }
        expected[15] = 0;
        sim.handler_0x593da(&mut st, &solid);
        assert_eq!(st.b, expected);
        assert!(sim.unported.is_empty());
    }

    let air = air();
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    let mut expected = st.b;
    position(&mut expected, 100, 99);
    sim.handler_0x593da(&mut st, &air);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x593da_invalid_states_report_the_entry() {
    let layout = solid();
    for state in 3..=255 {
        let (mut sim, mut st) = fixture(state);
        let original = st.b;
        sim.handler_0x593da(&mut st, &layout);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x593DA]);
    }
}

#[test]
fn handler_0x593da_real_anim64_step_triggers_from_mario() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = solid();
    assert_eq!(tables.profile_for(0, 64).unwrap().handler, 0x593DA);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 64;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 100,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[18], 72);
    assert!(sim.unported.is_empty());
}
