use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

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
fn handler_0x5a355_init_rises_36_and_picks_a_curve() {
    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    sim.mario_ref_x = 200;
    sim.handler_0x5a355(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 64);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 0x10);
    assert_eq!(st.b[18], 72);
    assert_eq!(st.b[25], 20);
    assert_eq!(st.b[24], 1);
    assert_eq!(sim.facing, 1);

    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[13] = 0x20;
    sim.mario_ref_x = 40;
    sim.handler_0x5a355(&mut st);
    assert_eq!(st.b[13], 0);
    assert_eq!(st.b[15], 0x12);
    assert_eq!(st.b[25], 1);
    assert_eq!(sim.facing, 255);
}

#[test]
fn handler_0x5a355_even_frame_steps_both_axes_and_carries() {
    let (mut sim, mut st) = fixture(0x10);
    sim.fff0 = 0;
    sim.mario.x = 10;
    sim.mario.y = 20;
    position(&mut st.b, 100, 100);
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[25] = 20;
    st.b[24] = 1;
    sim.handler_0x5a355(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 103);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 101);
    assert_eq!(st.b[25], 19);
    assert_eq!(st.b[24], 2);
    assert_eq!(sim.mario_x_adjust, Some(13));
    assert_eq!(sim.mario_y_adjust, Some(21));
}

#[test]
fn handler_0x5a355_vertical_index_advances_the_high_nibble() {
    let (mut sim, mut st) = fixture(0x10);
    sim.fff0 = 2;
    position(&mut st.b, 100, 100);
    st.b[25] = 2;
    st.b[24] = 0x14;
    sim.handler_0x5a355(&mut st);
    assert_eq!(st.b[15], 0x20);
    assert_eq!(st.b[24], 0x15);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a355_real_mode_b_anim67_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 67).unwrap().handler, 0x5A355);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 67;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        100,
        100,
        &layout,
        &tables,
        &MarioContact {
            x: 1000,
            ..Default::default()
        },
        false,
    );
    assert_eq!(sim.slots[1].b[18], 72);
    assert_eq!(sim.slots[1].b[15], 0x10);
    assert!(sim.unported.is_empty());
}
