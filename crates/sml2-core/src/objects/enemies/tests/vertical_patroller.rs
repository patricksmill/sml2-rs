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
fn handler_0x5a992_init_arms_contact_and_a_64_step_leg() {
    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 1;
    sim.handler_0x5a992(&mut st);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], 72);
    assert_eq!(st.b[24], 64);
    assert_eq!(st.b[25], 64);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a992_even_leg_ends_and_reverses() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    position(&mut st.b, 100, 100);
    st.b[11] = 10;
    st.b[24] = 1;
    st.b[25] = 64;
    sim.handler_0x5a992(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 101);
    assert_eq!(st.b[11], 11);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[24], 64);
}

#[test]
fn handler_0x5a992_odd_frame_leaves_the_pacer_alone() {
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[27] = 8;
    let expected = st.b;
    sim.handler_0x5a992(&mut st);
    assert_eq!(st.b, expected);
}

#[test]
fn handler_0x5a992_real_mode_b_anim80_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 80).unwrap().handler, 0x5A992);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 80;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 72);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(sim.slots[1].b[24], 64);
    assert_eq!(sim.slots[1].b[27], 0);
    assert!(sim.unported.is_empty());

    // The leg lives in `$FFE8`. A non-zero `$FFEB` makes the engine play
    // knockback and skip this handler, which is the bounce the saw was taking.
    let mut y = 100u16;
    for _ in 0..8 {
        sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
        assert_eq!(u16::from_be_bytes([sim.slots[1].b[0], sim.slots[1].b[1]]), 100);
        assert_eq!(sim.slots[1].b[27], 0);
        let next = u16::from_be_bytes([sim.slots[1].b[3], sim.slots[1].b[4]]);
        assert!(next == y || next == y + 1, "saw y jumped from {y} to {next}");
        y = next;
    }
    assert_eq!(y, 102);
}
