use super::*;
use crate::actor::{Actor, InteractionFlags, Position};
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
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.b[23] &= 0x0F;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5a5ec_waits_until_stood_on() {
    let tables = tables();
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5a5ec(&mut st, &tables);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 0);
    assert_eq!(st.b[18], 32);

    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    sim.handler_0x5a5ec(&mut st, &tables);
    assert_eq!(st.b[15], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a5ec_climb_ends_after_48_even_steps() {
    let tables = tables();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    sim.mario.x = 10;
    sim.mario.y = 20;
    position(&mut st.b, 100, 100);
    st.b[23] = 0x08;
    st.b[25] = 0x2F;
    sim.handler_0x5a5ec(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 101);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 99);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[25], 0);
    assert_eq!(sim.mario_x_adjust, Some(11));
    assert_eq!(sim.mario_y_adjust, Some(19));
}

#[test]
fn handler_0x5a5ec_drop_arms_the_pause() {
    let tables = tables();
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 0;
    position(&mut st.b, 100, 100);
    st.b[25] = 0x1F;
    sim.handler_0x5a5ec(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 101);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[25], 0);
    assert_eq!(st.b[28], 64);
}

#[test]
fn handler_0x5a5ec_pause_restores_the_spawn_record() {
    let tables = tables();
    let record = SpawnRecord {
        x: 0x0120,
        y: 0x0080,
        anim_index: 71,
        mode_flag: 0,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [0; 6],
    };
    let mut sim = ObjectSim::new(vec![record], 0x0A);
    sim.active = 1;
    sim.fff0 = 0;
    let mut st = Staging::default();
    st.b[6] = 0xAB;
    st.b[7] = 0x08; // record start $AB06 + 2, as $8CA8 stores it
    st.b[8] = 2;
    st.b[14] = 16;
    st.b[15] = 4;
    st.b[28] = 1;
    position(&mut st.b, 50, 50);
    sim.handler_0x5a5ec(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0120);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x0080);
    assert_eq!(st.b[8], 1);
    // $5B2BF/$863C never reclassify the slot; promotion/demotion own the counts.
    assert_eq!((sim.active, sim.standby), (1, 0));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a5ec_real_mode_b_anim71_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 71).unwrap().handler, 0x5A5EC);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 71;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 0);
    assert!(sim.unported.is_empty());
}
