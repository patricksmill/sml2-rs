use super::*;
use crate::actor::{Actor, Position};

fn air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
}
fn solid() -> LevelLayout {
    LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap()
}

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
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
fn handler_0x5a6e7_init_faces_and_arms_contact() {
    let layout = air();
    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    sim.mario_ref_x = 40;
    sim.handler_0x5a6e7(&mut st, &layout);
    assert_eq!(st.b[13], 0);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], 72);
    assert_eq!(sim.facing, 255);
}

#[test]
fn handler_0x5a6e7_solid_ceiling_keeps_the_direction() {
    let layout = solid();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    sim.handler_0x5a6e7(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[13], 0);
}

#[test]
fn handler_0x5a6e7_open_air_turns_after_the_step() {
    let layout = air();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    position(&mut st.b, 100, 100);
    st.b[11] = 10;
    st.b[13] = 0;
    sim.handler_0x5a6e7(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(st.b[11], 11);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a6e7_real_mode_b_anim74_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = solid();
    assert_eq!(tables.profile_for(0x0A, 74).unwrap().handler, 0x5A6E7);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 74;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        100,
        100,
        &layout,
        &tables,
        &MarioContact {
            x: 40,
            ..Default::default()
        },
        false,
    );
    assert_eq!(sim.slots[1].b[18], 72);
    assert!(sim.unported.is_empty());
}
