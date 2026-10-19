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

#[test]
fn handler_0x58bcb_anim40_uses_span_20() {
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 100, 100);
    st.b[18] = 0;
    sim.mario_ref_y = 200;
    sim.handler_0x58bcb(&mut st);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[18], 32);
    assert_eq!(st.b[24], 20);
    assert_eq!(sim.facing, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58bcb_long_legs_cap_at_double_span() {
    for (state, dy, next_state) in [(3u8, -1i16, 4u8), (4, 1, 3)] {
        let (mut sim, mut st) = fixture(state);
        position(&mut st.b, 100, 100);
        st.b[16] = 39; // span 20 * 2 = 40
        st.b[23] = 0;
        st.b[24] = 20;
        let mut expected = st.b;
        position(&mut expected, 100, (100i16 + dy) as u16);
        expected[15] = next_state;
        expected[16] = 0;
        sim.handler_0x58bcb(&mut st);
        assert_eq!(st.b, expected);
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x58bcb_real_anim40_step_dispatches_to_its_entry() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 40).unwrap().handler, 0x58BCB);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 50 };
    actor.activity = 2;
    actor.definition = 40;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 100,
        y: 100,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[24], 20);
    assert!(sim.unported.is_empty());
}
