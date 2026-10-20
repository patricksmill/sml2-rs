use super::*;
use crate::actor::{Actor, Position};

fn air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
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
fn handler_0x5a2d0_init_arms_the_rise() {
    let layout = air();
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5a2d0(&mut st, &layout);
    assert_eq!(st.b[12], 128);
    assert_eq!(st.b[15], 5);
    assert_eq!(st.b[18], 77);
    assert_eq!(st.b[24], 16);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a2d0_rise_faces_mario_when_the_counter_ends() {
    let layout = air();
    let (mut sim, mut st) = fixture(5);
    sim.fff0 = 2;
    sim.mario_ref_x = 200;
    position(&mut st.b, 100, 100);
    st.b[12] = 128;
    st.b[13] = 0;
    st.b[24] = 1;
    sim.handler_0x5a2d0(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 99);
    assert_eq!(st.b[12], 0);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[24], 0);
    assert_eq!(sim.facing, 1);
}

#[test]
fn handler_0x5a2d0_crawl_walks_left_on_even_frames() {
    let layout = air();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 2;
    position(&mut st.b, 100, 100);
    sim.handler_0x5a2d0(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(st.b[15], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a2d0_real_mode_b_anim64_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = air();
    assert_eq!(tables.profile_for(0x0A, 64).unwrap().handler, 0x5A2D0);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 64;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 77);
    assert_eq!(sim.slots[1].b[15], 5);
    assert_eq!(sim.slots[1].b[24], 16);
    assert_eq!(sim.slots[1].b[27], 0);
    assert!(sim.unported.is_empty());
}
