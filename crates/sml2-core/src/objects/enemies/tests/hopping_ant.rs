use super::*;
use crate::actor::{Actor, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = Staging::default();
    st.b[15] = state;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5afd7_init_faces_mario() {
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 100, 80);
    sim.mario_ref_x = 200;
    sim.handler_0x5afd7(&mut st);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[18], 77);
    assert_eq!(st.b[25], 2);
    assert_eq!(st.b[24], 1);
    assert_eq!(sim.facing, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5afd7_even_apex_turns_or_opens() {
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 100, 80);
    st.b[16] = 14;
    st.b[25] = 2;
    st.b[24] = 1;
    sim.handler_0x5afd7(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 77);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[16], 0);
    assert_eq!(st.b[25], 2);
    assert_eq!(st.b[24], 0);

    sim.fff0 = 1;
    let y = u16::from_be_bytes([st.b[3], st.b[4]]);
    sim.handler_0x5afd7(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 100);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), y);
    assert_eq!(st.b[16], 0);

    let (mut sim, mut st) = fixture(2);
    position(&mut st.b, 100, 80);
    st.b[16] = 14;
    st.b[25] = 1;
    st.b[24] = 0;
    sim.handler_0x5afd7(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 101);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 80);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[16], 0);
    assert_eq!(st.b[25], 0);
    assert_eq!(st.b[24], 1);
    assert_eq!(st.b[31], 2);
}

#[test]
fn handler_0x5afd7_mouth_spawns_then_the_pause_restores_the_walk() {
    let (mut sim, mut st) = fixture(3);
    position(&mut st.b, 100, 80);
    st.b[11] = 0x6F;
    st.b[31] = 1;
    sim.handler_0x5afd7(&mut st);
    assert_eq!(st.b[11], 108);
    assert_eq!(st.b[15], 4);
    assert_eq!(st.b[28], 8);
    assert_eq!(sim.slots[0].b[5], 89);
    assert_eq!(sim.slots[0].b[8], 1);
    assert_eq!(sim.slots[0].b[11], 112);
    assert_eq!(
        u16::from_be_bytes([sim.slots[0].b[0], sim.slots[0].b[1]]),
        100
    );
    assert_eq!(sim.standby, 1);

    sim.fff0 = 1;
    st.b[15] = 3;
    st.b[11] = 0x6C;
    sim.handler_0x5afd7(&mut st);
    assert_eq!(st.b[11], 0x6C);
    assert_eq!(st.b[15], 3);

    let (mut sim, mut st) = fixture(4);
    st.b[28] = 1;
    st.b[31] = 2;
    sim.handler_0x5afd7(&mut st);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[25], 2);
    assert_eq!(st.b[28], 0);

    st.b[15] = 4;
    st.b[28] = 0;
    sim.handler_0x5afd7(&mut st);
    assert_eq!(st.b[15], 4);
    assert_eq!(st.b[28], 255);
}

#[test]
fn handler_0x5afd7_real_mode_b_anim88_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 88).unwrap().handler, 0x5AFD7);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 88;
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
    assert_eq!(sim.slots[1].b[18], 77);
    assert_eq!(sim.slots[1].b[25], 2);
    assert_eq!(sim.slots[1].b[24], 1);
    assert!(sim.unported.is_empty());
}
