use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[15] = state;
    (sim, st)
}

#[test]
fn handler_0x5ab4b_launch_moves_left_48_instead_of_64() {
    let (mut sim, mut st) = fixture(4);
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    st.b[3..5].copy_from_slice(&100u16.to_be_bytes());
    st.b[28] = 1;
    sim.handler_0x5ab4b(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 52);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 36);
    assert_eq!(st.b[11], 193);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], 72);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ab4b_rider_still_starts_the_shared_fuse() {
    let (mut sim, mut st) = fixture(3);
    st.b[3..5].copy_from_slice(&80u16.to_be_bytes());
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    sim.handler_0x5ab4b(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 88);
    assert_eq!(st.b[11], 221);
    assert_eq!(st.b[15], 4);
    assert_eq!(st.b[28], 4);
}

#[test]
fn handler_0x5ab4b_real_mode_b_anim99_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 99).unwrap().handler, 0x5AB4B);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 99;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 3);
    assert_eq!(sim.slots[1].b[24], 48);
    assert_eq!(sim.slots[1].b[27], 0);
    assert!(sim.unported.is_empty());
}
