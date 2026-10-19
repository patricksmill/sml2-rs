use super::*;
use crate::actor::{Actor, Position};

#[test]
fn handler_0x5a8d2_arms_contact_and_sets_done() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = 0;
    let mut expected = st.b;
    expected[15] = 1;
    expected[18] = 16;
    sim.handler_0x5a8d2(&mut st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.done, 1);
}

#[test]
fn handler_0x5a8d2_nonzero_state_only_sets_done() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[15] = 2;
    let expected = st.b;
    sim.handler_0x5a8d2(&mut st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.done, 1);
}

#[test]
fn handler_0x5a8d2_real_mode_b_anim53_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 53).unwrap().handler, 0x5A8D2);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 53;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact::default();
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[18], 16);
    assert_eq!(sim.slots[1].b[15], 1);
    assert!(sim.unported.is_empty());
}
