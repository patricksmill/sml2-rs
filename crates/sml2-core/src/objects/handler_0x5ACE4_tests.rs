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
fn handler_0x5ace4_init_arms_the_flash_timer() {
    let tables = tables();
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5ace4(&mut st, &tables);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[28], 32);
    assert_eq!(sim.done, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ace4_timer_flickers_then_launches() {
    let tables = tables();
    let (mut sim, mut st) = fixture(2);
    st.b[28] = 32;
    sim.handler_0x5ace4(&mut st, &tables);
    assert_eq!(st.b[28], 31);
    assert_eq!(st.b[15], 2);
    assert_eq!(sim.done, 0);

    st.b[28] = 7;
    sim.handler_0x5ace4(&mut st, &tables);
    assert_eq!(st.b[28], 6);
    assert_eq!(sim.done, 1);

    st.b[28] = 1;
    sim.done = 0;
    sim.handler_0x5ace4(&mut st, &tables);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], 80);
    assert_eq!(st.b[28], 0);
    assert_eq!(sim.sfx2, 6);
    assert_eq!(sim.done, 0);
}

#[test]
fn handler_0x5ace4_slides_left_and_respawns_at_the_screen_edge() {
    let tables = tables();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 80);
    st.b[10] = 0xAF;
    sim.handler_0x5ace4(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 98);
    assert_eq!(st.b[15], 1);

    sim.fff0 = 0;
    st.b[11] = 10;
    st.b[26] = 0;
    sim.handler_0x5ace4(&mut st, &tables);
    assert_eq!(st.b[11], 11);
    assert_eq!(st.b[26], 1);

    let record = SpawnRecord {
        x: 0x0120,
        y: 0x0080,
        anim_index: 104,
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
    st.b[10] = 0xB0;
    st.b[15] = 1;
    sim.fff0 = 1;
    sim.handler_0x5ace4(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0120);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x0080);
    assert_eq!(st.b[8], 1);
    assert_eq!(sim.standby, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ace4_real_mode_b_anims_104_and_105_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 104).unwrap().handler, 0x5ACE4);
    assert_eq!(tables.profile_for(0x0A, 105).unwrap().handler, 0x5ACE4);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 104;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(sim.slots[1].b[28], 32);
    assert!(sim.unported.is_empty());
}
