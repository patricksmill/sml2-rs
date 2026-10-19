use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

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
fn handler_0x5aab9_init_arms_the_wait() {
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5aab9(&mut st);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[18], 32);
    assert_eq!(st.b[25], 48);
    assert_eq!(st.b[27], 48);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5aab9_rider_drops_then_the_fuse_launches() {
    let (mut sim, mut st) = fixture(3);
    position(&mut st.b, 100, 100);
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    sim.handler_0x5aab9(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 108);
    assert_eq!(st.b[11], 221);
    assert_eq!(st.b[15], 4);
    assert_eq!(st.b[28], 4);

    st.b[28] = 1;
    sim.handler_0x5aab9(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 36);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 44);
    assert_eq!(st.b[11], 193);
    assert_eq!(st.b[12], 128);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], 72);
}

#[test]
fn handler_0x5aab9_easy_mode_halves_the_step_and_the_fall_plays_sfx() {
    let (mut sim, mut st) = fixture(1);
    sim.easy_mode = true;
    position(&mut st.b, 100, 100);
    st.b[25] = 48;
    st.b[27] = 2;
    sim.handler_0x5aab9(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 102);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[27], 48);
    assert_eq!(sim.sfx1, 10);

    sim.sfx1 = 0;
    st.b[27] = 2;
    sim.handler_0x5aab9(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 100);
    assert_eq!(st.b[15], 1);
    assert_eq!(sim.sfx1, 0);
}

#[test]
fn handler_0x5aab9_real_mode_b_anim98_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 98).unwrap().handler, 0x5AAB9);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 98;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 3);
    assert_eq!(sim.slots[1].b[18], 32);
    assert!(sim.unported.is_empty());
}
