use super::*;
use crate::actor::{Actor, Position};
use crate::level::SpawnRecord;

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

fn y_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[3], slot[4]])
}

#[test]
fn handler_0x5917a_init_flashes_counts_the_bag_and_hops() {
    let tables = tables();
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 0x90, 0x0100);
    st.b[13] = 0x00;
    st.b[14] = 0xAA;
    st.b[16] = 0x55;
    st.b[18] = 0x66;
    sim.handler_0x5917a(&mut st, &tables);
    assert_eq!(st.b[14], 16);
    assert_eq!(y_of(&st.b), 0x0100 - 20);
    assert_eq!(st.b[13], 0x40);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[16], 0);
    assert_eq!(st.b[18], 0);
    assert_eq!(sim.money_bag, 1);
    assert_eq!(sim.done, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5917a_rise_runs_the_eight_step_profile() {
    let tables = tables();
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 0x40, 0x0100);
    st.b[5] = 54;
    st.b[16] = 0;
    sim.handler_0x5917a(&mut st, &tables);
    // Phase 0 selects 2, doubled by $581CE: 4 pixels up.
    assert_eq!(y_of(&st.b), 0x0100 - 4);
    assert_eq!(st.b[16], 1);
    assert_eq!(st.b[15], 1);

    // Phase 7 selects 0 and the postincrement reaches the cap.
    st.b[16] = 7;
    sim.handler_0x5917a(&mut st, &tables);
    assert_eq!(y_of(&st.b), 0x0100 - 4);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[16], 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5917a_fall_moves_without_touching_the_definition() {
    let tables = tables();
    let (mut sim, mut st) = fixture(2);
    position(&mut st.b, 0x40, 0x0200);
    st.b[5] = 54;
    st.b[9] = 0x9F;
    st.b[16] = 5;
    st.b[8] = 2;
    sim.handler_0x5917a(&mut st, &tables);
    assert_eq!(y_of(&st.b), 0x0200 + 2);
    assert_eq!(st.b[16], 6);
    assert_eq!(st.b[9], 0x9F);
    assert_eq!(st.b[5], 54);
    assert_eq!(st.b[8], 2);
    assert_eq!(sim.done, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5917a_falls_to_the_limit_and_restores_the_record_as_anim53() {
    let tables = tables();
    let record = SpawnRecord {
        x: 0x0120,
        y: 0x0080,
        anim_index: 53,
        mode_flag: 2,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [0; 6],
    };
    let mut sim = ObjectSim::new(vec![record], 0);
    let mut st = Staging::default();
    position(&mut st.b, 50, 50);
    st.b[5] = 54;
    st.b[6] = 0xAB;
    st.b[7] = 0x08; // record start $AB06 + 2
    st.b[8] = 2;
    st.b[9] = 0xA0;
    st.b[13] = 0x40;
    st.b[14] = 0x11;
    st.b[15] = 2;
    st.b[16] = 21;
    st.b[18] = 0x22;
    sim.handler_0x5917a(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0120);
    assert_eq!(y_of(&st.b), 0x0080);
    assert_eq!(st.b[2], 2);
    assert_eq!(st.b[5], 53);
    assert_eq!(st.b[8], 1);
    assert_eq!(st.b[11], tables.anim_to_sprite_a[53]);
    assert_eq!(st.b[13], 0x40);
    assert_eq!(&st.b[14..31], &[0; 17]);
    assert_eq!(sim.done, 1);
    assert_eq!(sim.spawn_attr, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5917a_unknown_state_reports_without_writes() {
    let tables = tables();
    let (mut sim, mut st) = fixture(3);
    st.b[16] = 9;
    let before = st.b;
    sim.handler_0x5917a(&mut st, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5917A]);
}

#[test]
fn handler_0x5917a_real_mode_a_anim54_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x00, 54).unwrap().handler, 0x5917A);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x00);
    let mut actor = Actor::default();
    actor.position = Position { x: 0x90, y: 0x0090 };
    actor.activity = 2;
    actor.definition = 54;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(0x90, 0x0090, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(sim.slots[1].b[13], 0x40);
    assert_eq!(y_of(&sim.slots[1].b), 0x0090 - 20);
    assert!(sim.unported.is_empty());
}
