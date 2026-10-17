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

#[test]
fn handler_0x5a236_init_arms_contact_32() {
    let (sim, mut st) = fixture(0);
    let mut expected = st.b;
    expected[15] = 1;
    expected[18] = 32;
    sim.handler_0x5a236(&mut st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.mario_x_adjust, None);
}

#[test]
fn handler_0x5a236_sprite_cycle_and_even_frame_carry() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    sim.mario.x = 50;
    st.b[11] = 10;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[26] = 0;
    sim.handler_0x5a236(&mut st);
    assert_eq!(st.b[26], 1);
    assert_eq!(st.b[11], 11);
    assert_eq!(sim.mario_x_adjust, Some(51));

    sim.fff0 = 4;
    sim.handler_0x5a236(&mut st);
    assert_eq!(st.b[26], 2);
    assert_eq!(st.b[11], 12);

    sim.fff0 = 8;
    sim.handler_0x5a236(&mut st);
    assert_eq!(st.b[26], 0);
    assert_eq!(st.b[11], 10);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a236_odd_frame_does_not_carry() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    sim.mario.x = 50;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    sim.handler_0x5a236(&mut st);
    assert_eq!(sim.mario_x_adjust, None);
}

#[test]
fn handler_0x5a236_real_mode_b_anim62_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 62).unwrap().handler, 0x5A236);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 62;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[15], 1);
    assert!(sim.unported.is_empty());
}
