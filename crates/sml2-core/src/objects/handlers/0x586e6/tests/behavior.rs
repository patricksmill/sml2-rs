use super::*;
use crate::actor::{Actor, Position};

#[test]
fn handler_0x586e6_sets_contact_137_and_touches_nothing_else() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    let mut expected = st.b;
    expected[18] = 137;
    sim.handler_0x586e6(&mut st);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x586e6_real_mode_b_anim31_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 31).unwrap().handler, 0x586E6);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A); // mode B table
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 31;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 1000,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[18], 137);
    assert!(sim.unported.is_empty());
}
