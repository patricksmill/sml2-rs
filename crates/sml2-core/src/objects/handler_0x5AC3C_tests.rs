use super::*;
use crate::actor::{Actor, Position};

fn air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
}
fn solid() -> LevelLayout {
    LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap()
}

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[15] = state;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5ac3c_init_waits_for_mario_within_48() {
    let layout = air();
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5ac3c(&mut st, &layout);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[18], 72);

    position(&mut st.b, 0x0100, 80);
    sim.mario_ref_x = 0x0130;
    sim.handler_0x5ac3c(&mut st, &layout);
    assert_eq!(st.b[15], 3);

    sim.mario_ref_x = 0x0110;
    sim.handler_0x5ac3c(&mut st, &layout);
    assert_eq!(st.b[15], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ac3c_falls_until_the_ground_and_rises_until_the_ceiling() {
    let air = air();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[16] = 0;
    sim.handler_0x5ac3c(&mut st, &air);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 101);
    assert_eq!(st.b[16], 1);

    let solid = solid();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 0x60);
    st.b[16] = 4;
    sim.handler_0x5ac3c(&mut st, &solid);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x60);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[16], 0);
    assert_eq!(sim.sfx1, 10);

    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    sim.handler_0x5ac3c(&mut st, &air);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 99);

    sim.handler_0x5ac3c(&mut st, &solid);
    assert_eq!(st.b[15], 3);
}

#[test]
fn handler_0x5ac3c_real_mode_b_anim102_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = air();
    assert_eq!(tables.profile_for(0x0A, 102).unwrap().handler, 0x5AC3C);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 102;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 3);
    assert_eq!(sim.slots[1].b[18], 72);
    assert!(sim.unported.is_empty());
}
