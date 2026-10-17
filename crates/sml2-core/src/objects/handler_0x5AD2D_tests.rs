use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
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
fn handler_0x5ad2d_init_faces_right_and_arms_the_timer() {
    let tables = tables();
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5ad2d(&mut st, &tables);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[28], 32);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ad2d_slides_right_and_respawns_at_screen_b8() {
    let tables = tables();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 80);
    st.b[10] = 0xB7;
    sim.handler_0x5ad2d(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 102);
    assert_eq!(st.b[15], 1);

    let record = SpawnRecord {
        x: 0x0120,
        y: 0x0080,
        anim_index: 106,
        mode_flag: 0,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [0; 6],
    };
    let mut sim = ObjectSim::new(vec![record], 0x0A);
    sim.active = 1;
    let mut st = Staging::default();
    position(&mut st.b, 50, 50);
    st.b[6] = 0xAB;
    st.b[7] = 0x06;
    st.b[8] = 2;
    st.b[10] = 0xB8;
    st.b[15] = 1;
    sim.fff0 = 1;
    sim.handler_0x5ad2d(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0120);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x0080);
    assert_eq!(st.b[8], 1);
    assert_eq!(sim.standby, 1);
}

#[test]
fn handler_0x5ad2d_real_mode_b_anim106_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 106).unwrap().handler, 0x5AD2D);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 106;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[13], 0x20);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(sim.slots[1].b[28], 32);
    assert!(sim.unported.is_empty());
}
