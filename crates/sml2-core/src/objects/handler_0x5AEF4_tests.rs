use super::*;
use crate::actor::{Actor, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    let mut st = Staging::default();
    st.b[15] = state;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5aef4_init_faces_mario() {
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 100, 80);
    sim.mario_ref_x = 40;
    sim.handler_0x5aef4(&mut st);
    assert_eq!(st.b[13], 0);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], 72);
    assert_eq!(sim.facing, 255);
}

#[test]
fn handler_0x5aef4_shorter_curves_turn_at_index_32() {
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 100, 80);
    sim.handler_0x5aef4(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 96);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 80);
    assert_eq!(st.b[25], 1);

    st.b[25] = 32;
    sim.handler_0x5aef4(&mut st);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[25], 0);

    let (mut sim, mut st) = fixture(3);
    position(&mut st.b, 100, 80);
    st.b[25] = 12;
    sim.handler_0x5aef4(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 101);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 82);
    assert_eq!(st.b[25], 13);

    let (mut sim, mut st) = fixture(2);
    st.b[25] = 32;
    sim.handler_0x5aef4(&mut st);
    assert_eq!(st.b[15], 4);
    assert_eq!(st.b[25], 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5aef4_real_mode_b_anim111_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 111).unwrap().handler, 0x5AEF4);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 111;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        100,
        100,
        &layout,
        &tables,
        &MarioContact {
            x: 1000,
            ..Default::default()
        },
        false,
    );
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(sim.slots[1].b[18], 72);
    assert!(sim.unported.is_empty());
}
