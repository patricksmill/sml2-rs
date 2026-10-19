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
        anim_index: 53,
        mode_flag: 2,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [0; 6],
    }
}

fn staging(state: u8, x: u16, y: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 84;
    st.b[15] = state;
    st.b[0..2].copy_from_slice(&x.to_be_bytes());
    st.b[3..5].copy_from_slice(&y.to_be_bytes());
    st
}

fn y_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[3], slot[4]])
}

#[test]
fn handler_0x5aa68_init_hops_without_the_money_bag() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0, 0x0090, 0x0100);
    st.b[13] = 0x00;
    sim.handler_0x5aa68(&mut st, &tables);
    assert_eq!(y_of(&st.b), 0x0100 - 20);
    assert_eq!(
        (st.b[13], st.b[14], st.b[15], st.b[16], st.b[18]),
        (0x40, 0, 1, 0, 0)
    );
    assert_eq!(sim.money_bag, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5aa68_rise_and_respawn_share_the_anim54_tail() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![record()], 0);
    let mut st = staging(1, 0x0090, 0x0100);
    st.b[16] = 0;
    sim.handler_0x5aa68(&mut st, &tables);
    assert_eq!(y_of(&st.b), 0x0100 - 4);
    assert_eq!(st.b[16], 1);

    let mut st = staging(2, 0x0090, 0x0100);
    st.b[6] = 0xAB;
    st.b[7] = 0x08;
    st.b[9] = 0xA0;
    st.b[16] = 21;
    sim.handler_0x5aa68(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0120);
    assert_eq!(y_of(&st.b), 0x0080);
    assert_eq!(st.b[5], 83);
    assert_eq!(st.b[11], tables.anim_to_sprite_a[83]);
    assert_eq!(sim.done, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5aa68_unknown_state_reports_without_writes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(3, 0x0090, 0x0100);
    let before = st.b;
    sim.handler_0x5aa68(&mut st, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5AA68]);
}

#[test]
fn handler_0x5aa68_real_mode_b_anim84_step() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 84).unwrap().handler, 0x5AA68);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = crate::actor::Actor::default();
    actor.position = crate::actor::Position {
        x: 0x0090,
        y: 0x0100,
    };
    actor.activity = 2;
    actor.definition = 84;
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
    assert_eq!(sim.slots[1].b[13], 0x40);
    assert!(sim.unported.is_empty());
}
