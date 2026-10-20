use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    sim.mario.x = 40;
    let mut st = Staging::default();
    st.b[15] = state;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5ad4d_init_arms_the_rightward_leg() {
    let (mut sim, mut st) = fixture(0);
    sim.handler_0x5ad4d(&mut st);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[16], 1);
    assert_eq!(st.b[18], 32);
    assert_eq!(st.b[24], 80);
    assert_eq!(st.b[25], 40);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5ad4d_right_leg_carries_and_turns_at_the_end() {
    let (mut sim, mut st) = fixture(2);
    position(&mut st.b, 100, 80);
    st.b[16] = 1;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[25] = 40;
    sim.handler_0x5ad4d(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 101);
    assert_eq!(st.b[16], 0);
    assert_eq!(st.b[25], 39);
    assert_eq!(sim.mario_x_adjust, Some(41));

    st.b[25] = 1;
    st.b[16] = 1;
    sim.mario_x_adjust = None;
    sim.handler_0x5ad4d(&mut st);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[25], 0);
    assert_eq!(st.b[28], 48);
}

#[test]
fn handler_0x5ad4d_easy_odd_frames_only_bob() {
    let (mut sim, mut st) = fixture(2);
    sim.easy_mode = true;
    sim.fff0 = 0;
    position(&mut st.b, 100, 80);
    st.b[11] = 10;
    st.b[16] = 1;
    st.b[25] = 40;
    st.b[26] = 0;
    sim.handler_0x5ad4d(&mut st);
    assert_eq!(st.b[11], 11);
    assert_eq!(st.b[26], 1);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 101);

    sim.fff0 = 1;
    let x = u16::from_be_bytes([st.b[0], st.b[1]]);
    let phase = st.b[16];
    let span = st.b[25];
    sim.handler_0x5ad4d(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), x);
    assert_eq!(st.b[16], phase);
    assert_eq!(st.b[25], span);
}

#[test]
fn handler_0x5ad4d_pause_reloads_and_the_left_leg_turns() {
    let (mut sim, mut st) = fixture(3);
    st.b[24] = 80;
    st.b[28] = 1;
    sim.handler_0x5ad4d(&mut st);
    assert_eq!(st.b[15], 4);
    assert_eq!(st.b[16], 0);
    assert_eq!(st.b[25], 80);

    let (mut sim, mut st) = fixture(4);
    position(&mut st.b, 100, 80);
    st.b[16] = 1;
    st.b[25] = 1;
    sim.handler_0x5ad4d(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[28], 48);

    st.b[24] = 80;
    st.b[28] = 5;
    sim.handler_0x5ad4d(&mut st);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[28], 4);
}

#[test]
fn handler_0x5ad4d_real_mode_b_anim107_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 107).unwrap().handler, 0x5AD4D);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 107;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[24], 80);
    assert_eq!(sim.slots[1].b[25], 40);
    assert_eq!(sim.slots[1].b[27], 0);
    assert!(sim.unported.is_empty());
}
