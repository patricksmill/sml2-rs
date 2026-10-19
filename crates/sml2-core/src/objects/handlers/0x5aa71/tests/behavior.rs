use super::*;
use crate::actor::{Actor, Position};

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
fn handler_0x5aa71_init_arms_the_wait() {
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5aa71(&mut st);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[18], 72);
    assert_eq!(st.b[25], 8);
    assert_eq!(st.b[27], 8);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5aa71_proximity_sets_sprite_bit_0() {
    let (mut sim, mut st) = fixture(3);
    position(&mut st.b, 0x0100, 80);
    st.b[11] = 0x10;
    sim.mario_ref_x = 0x0130;
    sim.handler_0x5aa71(&mut st);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[11], 0x10);

    sim.mario_ref_x = 0x0110;
    sim.handler_0x5aa71(&mut st);
    assert_eq!(st.b[11], 0x11);
    assert_eq!(st.b[15], 1);
}

#[test]
fn handler_0x5aa71_leg_reverses_after_eight_steps() {
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 100, 100);
    st.b[25] = 8;
    st.b[27] = 1;
    sim.handler_0x5aa71(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 99);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[27], 8);
}

#[test]
fn handler_0x5aa71_real_mode_b_anims_86_and_87_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 86).unwrap().handler, 0x5AA71);
    assert_eq!(tables.profile_for(0x0A, 87).unwrap().handler, 0x5AA71);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 86;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 72);
    assert_eq!(sim.slots[1].b[15], 3);
    assert_eq!(sim.slots[1].b[27], 8);
    assert!(sim.unported.is_empty());
}
