use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

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

#[test]
fn handler_0x5ac7d_init_sets_contact_and_easy_sprite() {
    let tables = tables();
    let (mut sim, mut st) = fixture(0);
    st.b[11] = 10;
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 10);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], 32);

    let (mut sim, mut st) = fixture(0);
    sim.easy_mode = true;
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 224);
    assert_eq!(st.b[15], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ac7d_stood_on_selects_the_growth_state() {
    let tables = tables();
    let (mut sim, mut st) = fixture(1);
    st.b[11] = 4;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    sim.fff0 = 0;
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 5);
    assert_eq!(st.b[15], 2);

    let (mut sim, mut st) = fixture(1);
    st.b[11] = 4;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    sim.easy_mode = true;
    sim.fff0 = 1;
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 4);
    assert_eq!(st.b[15], 3);
}

#[test]
fn handler_0x5ac7d_normal_growth_chimes_then_despawns() {
    let tables = tables();
    let (mut sim, mut st) = fixture(2);
    st.b[8] = 2;
    st.b[11] = 0xCD;
    sim.active = 1;
    sim.fff0 = 1;
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 0xCD);
    assert_eq!(sim.sfx2, 0);

    sim.fff0 = 0;
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 0xCE);
    assert_eq!(sim.sfx2, 1);
    assert_eq!(st.b[15], 2);

    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 0xCF);
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[..19], [0; 19]);
    assert_eq!(st.b[23..], [0; 9]);
    assert_eq!(sim.done, 2);
    assert_eq!(sim.active, 0);
}

#[test]
fn handler_0x5ac7d_easy_growth_uses_the_higher_thresholds() {
    let tables = tables();
    let (mut sim, mut st) = fixture(3);
    st.b[11] = 0xE1;
    sim.fff0 = 8;
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 0xE2);
    assert_eq!(sim.sfx2, 1);

    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(st.b[11], 0xE3);
    sim.handler_0x5ac7d(&mut st, &tables);
    assert_eq!(sim.done, 2);
}

#[test]
fn handler_0x5ac7d_real_mode_b_anim103_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 103).unwrap().handler, 0x5AC7D);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 103;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(sim.slots[1].b[18], 32);
    assert!(sim.unported.is_empty());
}
