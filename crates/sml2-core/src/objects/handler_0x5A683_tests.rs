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
fn handler_0x5a683_init_waits_to_be_stood_on() {
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5a683(&mut st);
    assert_eq!(st.b[15], 4);
    assert_eq!(st.b[18], 32);

    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    sim.handler_0x5a683(&mut st);
    assert_eq!(st.b[15], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a683_sprite_extends_then_the_slide_carries() {
    let (mut sim, mut st) = fixture(1);
    st.b[11] = 0xBD;
    sim.handler_0x5a683(&mut st);
    assert_eq!(st.b[11], 0xBE);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[25], 112);

    sim.fff0 = 0;
    sim.mario.x = 40;
    position(&mut st.b, 100, 80);
    st.b[23] = 0x08;
    st.b[25] = 1;
    sim.handler_0x5a683(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 101);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[25], 0);
    assert_eq!(st.b[28], 32);
    assert_eq!(sim.mario_x_adjust, Some(41));
}

#[test]
fn handler_0x5a683_pause_then_lowers_the_sprite_and_clears_contact() {
    let (mut sim, mut st) = fixture(3);
    st.b[11] = 0xBE;
    st.b[18] = 32;
    st.b[28] = 2;
    sim.handler_0x5a683(&mut st);
    assert_eq!(st.b[28], 1);
    assert_eq!(st.b[11], 0xBE);

    st.b[28] = 0;
    sim.handler_0x5a683(&mut st);
    assert_eq!(st.b[11], 0xBD);
    assert_eq!(st.b[18], 0);

    st.b[11] = 0xBC;
    sim.handler_0x5a683(&mut st);
    assert_eq!(st.b[11], 0xBC);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a683_real_mode_b_anim73_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 73).unwrap().handler, 0x5A683);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 73;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[15], 4);
    assert!(sim.unported.is_empty());
}
