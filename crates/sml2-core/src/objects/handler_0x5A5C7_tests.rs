use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn staging(state: u8, pointer: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 69;
    st.b[15] = state;
    st.b[6..8].copy_from_slice(&pointer.to_be_bytes());
    st
}

fn parent(sim: &mut ObjectSim, x: u16, y: u16, marker: u8) {
    let mut parent = Actor::default();
    parent.position = Position { x, y };
    parent.marker = marker;
    parent.activity = 2;
    sim.slots[0].b = parent.to_raw_slot();
}

#[test]
fn handler_0x5a5c7_init_arms_contact_and_lifts_24() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0, 0);
    st.b[3..5].copy_from_slice(&0x0080u16.to_be_bytes());
    sim.handler_0x5a5c7(&mut st);
    assert_eq!((st.b[18], st.b[15]), (77, 1));
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x0080 - 24);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a5c7_follows_the_parent_and_cycles_the_sprite() {
    let mut sim = ObjectSim::new(vec![], 0);
    parent(&mut sim, 0x1234, 0x5678, 2);
    sim.fff0 = 4;
    let mut st = staging(1, 0xAD00);
    st.b[11] = 10;
    st.b[26] = 0;
    sim.handler_0x5a5c7(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x1234);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x5678 - 24);
    assert_eq!(st.b[2], 2);
    assert_eq!((st.b[11], st.b[26]), (11, 1));
    assert!(sim.unported.is_empty());

    // The fourth entry only runs on the wrap from animation frame 3.
    let mut st = staging(1, 0xAD00);
    st.b[11] = 0x20;
    st.b[26] = 3;
    sim.handler_0x5a5c7(&mut st);
    assert_eq!((st.b[11], st.b[26]), (0x1E, 0));
}

#[test]
fn handler_0x5a5c7_unresolved_source_reports_without_writes() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(1, 0xAB08);
    let before = st.b;
    sim.handler_0x5a5c7(&mut st);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A5C7]);
}

#[test]
fn handler_0x5a5c7_unknown_state_reports_without_writes() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(2, 0xAD00);
    let before = st.b;
    sim.handler_0x5a5c7(&mut st);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A5C7]);
}

#[test]
fn handler_0x5a5c7_real_mode_b_anim69_step() {
    let _ = tables();
    let tables = tables();
    let lay = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 69).unwrap().handler, 0x5A5C7);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position {
        x: 0x0100,
        y: 0x0080,
    };
    actor.activity = 2;
    actor.definition = 69;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        0x0080,
        0x0080,
        &lay,
        &tables,
        &MarioContact::default(),
        false,
    );
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(
        u16::from_be_bytes([sim.slots[1].b[3], sim.slots[1].b[4]]),
        0x0080 - 24
    );
}
