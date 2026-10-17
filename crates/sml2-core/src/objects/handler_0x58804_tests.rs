use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

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
    st.b[23] &= 0x0F; // keep fixture noise but clear interaction flags
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x58804_state_zero_advances_to_one() {
    let (mut sim, mut st) = fixture(0);
    let mut expected = st.b;
    expected[15] = 1;
    sim.handler_0x58804(&mut st, &tables());
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58804_proximity_arms_contact_136() {
    let tables = tables();
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 0x0100, 0x0200);
    sim.mario_ref_x = 0x0110; // distance 0x10 < 0x30
    let mut expected = st.b;
    expected[15] = 2;
    expected[18] = 136;
    sim.handler_0x58804(&mut st, &tables);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());

    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 0x0100, 0x0200);
    sim.mario_ref_x = 0x0130; // distance 0x30 is not < 0x30
    let expected = st.b;
    sim.handler_0x58804(&mut st, &tables);
    assert_eq!(st.b, expected);
}

#[test]
fn handler_0x58804_sprite_ramp_reaches_88_on_quarter_frames() {
    let tables = tables();
    let (mut sim, mut st) = fixture(2);
    st.b[11] = 0x86;
    sim.fff0 = 4; // fff0 & 3 == 0
    let mut expected = st.b;
    expected[11] = 0x87;
    sim.handler_0x58804(&mut st, &tables);
    assert_eq!(st.b, expected);

    sim.fff0 = 8;
    expected[11] = 0x88;
    expected[15] = 3;
    sim.handler_0x58804(&mut st, &tables);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58804_state_three_rises_and_despawns_on_touch() {
    let tables = tables();
    let (mut sim, mut st) = fixture(3);
    position(&mut st.b, 0x0100, 0x0200);
    st.b[8] = 2; // active slot for remove_object accounting
    sim.fff0 = 3; // no $5B511 toggle
    let mut expected = st.b;
    position(&mut expected, 0x0100, 0x01FF);
    sim.handler_0x58804(&mut st, &tables);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());

    let (mut sim, mut st) = fixture(3);
    position(&mut st.b, 0x0100, 0x0200);
    st.b[8] = 2;
    st.b[23] = InteractionFlags::from_bits(0x10).bits();
    sim.handler_0x58804(&mut st, &tables);
    assert_eq!(st.b[..19], [0; 19]);
    assert_eq!(st.b[23..], [0; 9]);
    assert_eq!(sim.done, 2);
}

#[test]
fn handler_0x58804_real_mode_b_anim24_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 24).unwrap().handler, 0x58804);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 0;
    sim.phase = false;
    sim.mario_ref_x = 0x0064;
    let mut actor = Actor::default();
    actor.position = Position { x: 0x0060, y: 0x0100 };
    actor.activity = 2;
    actor.definition = 24;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 1000,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[15], 1);
    assert!(sim.unported.is_empty());
}
