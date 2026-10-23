use super::*;
use crate::actor::InteractionFlags;
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

fn ride(slot: &mut [u8; 32]) {
    slot[23] = InteractionFlags::from_bits(0x08).bits();
}

#[test]
fn handler_0x5a258_waits_for_a_rider_then_arms_the_drop() {
    let tables = tables();
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5a258(&mut st, &tables);
    assert_eq!(st.b[15], 0);
    assert_eq!(st.b[18], 32);

    ride(&mut st.b);
    sim.handler_0x5a258(&mut st, &tables);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[24], 24);
    assert_eq!(st.b[25], 24);
}

#[test]
fn handler_0x5a258_sinks_while_ridden_and_rises_after_the_pause() {
    let tables = tables();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    sim.mario.y = 40;
    position(&mut st.b, 100, 100);
    ride(&mut st.b);
    st.b[24] = 2;
    sim.handler_0x5a258(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 101);
    assert_eq!(st.b[24], 1);
    assert_eq!(sim.mario_y_adjust, Some(41));

    st.b[23] &= !0x08;
    sim.handler_0x5a258(&mut st, &tables);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[28], 16);

    st.b[28] = 0;
    st.b[25] = 24;
    sim.handler_0x5a258(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 100);
    assert_eq!(st.b[24], 2);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a258_bottom_restores_the_spawn_record() {
    let tables = tables();
    let record = SpawnRecord {
        x: 0x0120,
        y: 0x0080,
        anim_index: 63,
        mode_flag: 0,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [0; 6],
    };
    let mut sim = ObjectSim::new(vec![record], 0x0A);
    sim.active = 1;
    let mut st = Staging::default();
    st.b[6] = 0xAB;
    st.b[7] = 0x08; // record start $AB06 + 2, as $8CA8 stores it
    st.b[8] = 2;
    st.b[15] = 1;
    st.b[23] = 0x08;
    st.b[24] = 0;
    position(&mut st.b, 50, 50);
    sim.fff0 = 0;
    sim.handler_0x5a258(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0120);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x0080);
    assert_eq!(st.b[8], 1);
    assert_eq!(st.b[15], 0);
    // $5B2BF/$863C never reclassify the slot; promotion/demotion own the counts.
    assert_eq!((sim.active, sim.standby), (1, 0));
}

#[test]
fn handler_0x5a258_real_mode_b_anim63_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 63).unwrap().handler, 0x5A258);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = crate::actor::Actor::default();
    actor.position = crate::actor::Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 63;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[15], 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a258_ridden_platform_sinks_instead_of_knockback() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = crate::actor::Actor::default();
    actor.position = crate::actor::Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 63;
    sim.slots[1].b = actor.to_raw_slot();
    sim.slots[1].b[23] = 0x08;
    let mario = MarioContact::default();
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(sim.slots[1].b[24], 24);
    assert_eq!(sim.slots[1].b[27], 0);

    let mut y = 100u16;
    for _ in 0..6 {
        sim.slots[1].b[23] = 0x08;
        sim.step(100, 100, &layout, &tables, &mario, false);
        assert_eq!(
            u16::from_be_bytes([sim.slots[1].b[0], sim.slots[1].b[1]]),
            100
        );
        assert_eq!(sim.slots[1].b[27], 0);
        let next = u16::from_be_bytes([sim.slots[1].b[3], sim.slots[1].b[4]]);
        assert!(
            next == y || next == y + 1,
            "platform y jumped from {y} to {next}"
        );
        y = next;
    }
    assert!(y > 100, "ridden platform never sank");
}
