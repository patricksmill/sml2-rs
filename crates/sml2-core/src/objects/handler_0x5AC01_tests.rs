use super::*;
use crate::level::SpawnRecord;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn record() -> SpawnRecord {
    SpawnRecord {
        x: 0x0120,
        y: 0x0080,
        anim_index: 101,
        mode_flag: 2,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [0; 6],
    }
}

fn staging(state: u8, timer: u8) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 101;
    st.b[15] = state;
    st.b[28] = timer;
    st.b[6] = 0xAB;
    st.b[7] = 0x08;
    st
}

#[test]
fn handler_0x5ac01_init_starts_the_fade() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0, 0);
    sim.handler_0x5ac01(&mut st, &tables);
    assert_eq!((st.b[15], st.b[28]), (1, 56));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ac01_hard_mode_rebirths_with_the_decremented_definition() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![record()], 0);
    sim.configure(0x0A);
    let mut st = staging(1, 1);
    st.b[31] = 5;
    sim.handler_0x5ac01(&mut st, &tables);
    assert_eq!(st.b[5], 100);
    assert_eq!(st.b[18], 73);
    assert_eq!(st.b[15], 5, "state comes from the visibility byte");
    assert_eq!(st.b[11], tables.anim_to_sprite_b[100]);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0120);
    assert_eq!(sim.done, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ac01_easy_mode_removes_and_counts_the_boss() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.easy_mode = true;
    sim.boss_defeated = true;
    sim.active = 1;
    let mut st = staging(1, 1);
    sim.handler_0x5ac01(&mut st, &tables);
    let hb = tables.hitbox_for(0, 0).unwrap();
    assert_eq!(&st.b[..19], &[0; 19]);
    assert_eq!(&st.b[19..23], &hb.map(|b| b as u8));
    assert_eq!(&st.b[23..], &[0; 9]);
    assert_eq!(sim.done, 2);
    assert_eq!((sim.active, sim.standby), (0, 0));
    assert!(!sim.boss_defeated);
}

#[test]
fn handler_0x5ac01_unknown_state_reports_without_writes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(2, 0);
    let before = st.b;
    sim.handler_0x5ac01(&mut st, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5AC01]);
}

#[test]
fn handler_0x5ac01_real_mode_b_anim101_step() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 101).unwrap().handler, 0x5AC01);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = crate::actor::Actor::default();
    actor.position = crate::actor::Position {
        x: 0x0100,
        y: 0x0080,
    };
    actor.activity = 2;
    actor.definition = 101;
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
    assert_eq!((sim.slots[1].b[15], sim.slots[1].b[28]), (1, 56));
}
