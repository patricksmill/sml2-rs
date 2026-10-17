use super::*;
use crate::actor::{Actor, Position};

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
fn handler_0x59eff_arms_and_waits_for_mario_within_48() {
    for (object_x, mario_x, next) in [
        (0x0100u16, 0x0120u16, 1u8),
        (0x0100, 0x0130, 0),
        (0x0110, 0x00F0, 1),
    ] {
        let (mut sim, mut st) = fixture(0);
        position(&mut st.b, object_x, 100);
        st.b[14] = 0;
        sim.mario_ref_x = mario_x;
        let mut expected = st.b;
        expected[14] = 16;
        expected[15] = next;
        expected[18] = 72;
        sim.handler_0x59eff(&mut st);
        assert_eq!(st.b, expected, "mario={mario_x:#x}");
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x59eff_bobs_down_until_sprite_41_then_pauses() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[11] = 0x28;
    let expected = st.b;
    sim.handler_0x59eff(&mut st);
    assert_eq!(st.b, expected);

    sim.fff0 = 4;
    st.b[28] = 2;
    let mut expected = st.b;
    expected[28] = 1;
    sim.handler_0x59eff(&mut st);
    assert_eq!(st.b, expected);

    st.b[28] = 0;
    let mut expected = st.b;
    position(&mut expected, 100, 108);
    expected[11] = 0x29;
    expected[15] = 2;
    expected[28] = 8;
    sim.handler_0x59eff(&mut st);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59eff_bobs_up_until_sprite_37() {
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 0;
    position(&mut st.b, 100, 100);
    st.b[11] = 0x26;
    st.b[28] = 0;
    sim.handler_0x59eff(&mut st);
    assert_eq!(st.b[11], 0x25);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[28], 8);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 92);
}

#[test]
fn handler_0x59eff_invalid_state_is_unported() {
    let (mut sim, mut st) = fixture(3);
    sim.fff0 = 0;
    let expected = st.b;
    sim.handler_0x59eff(&mut st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.unported, vec![0x59EFF]);
}

#[test]
fn handler_0x59eff_real_mode_b_anim33_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 33).unwrap().handler, 0x59EFF);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.mario_ref_x = 0x0070;
    let mut actor = Actor::default();
    actor.position = Position { x: 0x0060, y: 100 };
    actor.activity = 2;
    actor.definition = 33;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        100,
        100,
        &layout,
        &tables,
        &MarioContact {
            x: 0x0070,
            ..Default::default()
        },
        false,
    );
    assert_eq!(sim.slots[1].b[18], 72);
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 1);
    assert!(sim.unported.is_empty());
}
