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
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.b[23] &= 0x0F;
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

fn check(sim: &mut ObjectSim, st: &mut Staging, expected: [u8; 32], facing: u8) {
    sim.handler_0x58ce0(st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, facing);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58ce0_init_faces_and_arms_leg_counters() {
    for (target_x, state, flip, facing) in
        [(200u16, 2u8, 0x20u8, 1u8), (50, 1, 0, 255), (100, 1, 0, 0)]
    {
        let (mut sim, mut st) = fixture(0);
        sim.fff0 = 0;
        position(&mut st.b, 100, 100);
        st.b[13] = 0;
        sim.mario_ref_x = target_x;
        let mut expected = st.b;
        expected[13] = flip;
        expected[15] = state;
        expected[18] = 32;
        expected[27] = 24;
        expected[28] = 48;
        check(&mut sim, &mut st, expected, facing);
    }
}

#[test]
fn handler_0x58ce0_skips_motion_on_odd_fff0_frames() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[27] = 10;
    let expected = st.b;
    sim.handler_0x58ce0(&mut st);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x58ce0_state_one_steps_diagonally_and_counts_down() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 2;
    position(&mut st.b, 100, 100);
    st.b[27] = 2;
    st.b[28] = 48;
    let mut expected = st.b;
    position(&mut expected, 99, 101);
    expected[27] = 1;
    check(&mut sim, &mut st, expected, 99);

    st.b[27] = 1;
    let mut expected = st.b;
    position(&mut expected, 98, 102);
    expected[27] = 48;
    expected[15] = 2;
    check(&mut sim, &mut st, expected, 99);
}

#[test]
fn handler_0x58ce0_carries_a_stood_on_rider_on_both_axes() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    position(&mut st.b, 100, 100);
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[27] = 24;
    st.b[28] = 48;
    sim.mario.x = 200;
    sim.mario.y = 200;
    sim.handler_0x58ce0(&mut st);
    assert_eq!(sim.mario_x_adjust, Some(199));
    assert_eq!(sim.mario_y_adjust, Some(201));
}

#[test]
fn handler_0x58ce0_real_anim44_mode_a_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(5, 44).unwrap().handler, 0x58CE0);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(5);
    sim.fff0 = 0;
    sim.mario_ref_x = 200;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 44;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 1000,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[27], 24);
    assert_eq!(sim.slots[1].b[28], 48);
    assert!(sim.unported.is_empty());
}
