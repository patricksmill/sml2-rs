use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}
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
fn handler_0x59e5b_init_faces_and_arms_span_48() {
    let layout = air();
    for (target_x, state, flip, facing) in
        [(200u16, 2u8, 0x20u8, 1u8), (50, 1, 0, 255), (100, 1, 0, 0)]
    {
        let (mut sim, mut st) = fixture(0);
        position(&mut st.b, 100, 100);
        st.b[13] = 0;
        st.b[14] = 0;
        sim.mario_ref_x = target_x;
        let mut expected = st.b;
        expected[13] = flip;
        expected[14] = 16;
        expected[15] = state;
        expected[18] = 76;
        expected[25] = 48;
        sim.handler_0x59e5b(&mut st, &layout);
        assert_eq!(st.b, expected);
        assert_eq!(sim.facing, facing);
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x59e5b_walks_until_the_span_expires() {
    let layout = air();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 2;
    position(&mut st.b, 100, 100);
    st.b[11] = 0x20;
    st.b[25] = 2;
    let mut expected = st.b;
    position(&mut expected, 99, 100);
    expected[25] = 1;
    sim.handler_0x59e5b(&mut st, &layout);
    assert_eq!(st.b, expected);

    let mut expected = st.b;
    position(&mut expected, 98, 100);
    expected[11] = 0x35;
    expected[15] = 3;
    expected[25] = 48;
    sim.handler_0x59e5b(&mut st, &layout);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59e5b_left_wall_flips_without_moving() {
    let layout = solid();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 3;
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    st.b[25] = 10;
    let mut expected = st.b;
    expected[13] = 0x20;
    expected[15] = 2;
    sim.handler_0x59e5b(&mut st, &layout);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, 99);
}

#[test]
fn handler_0x59e5b_rise_climbs_the_sprite_then_counts_to_32() {
    let layout = air();
    let (mut sim, mut st) = fixture(3);
    sim.fff0 = 1;
    st.b[11] = 0x35;
    let expected = st.b;
    sim.handler_0x59e5b(&mut st, &layout);
    assert_eq!(st.b, expected);

    sim.fff0 = 2;
    let mut expected = st.b;
    expected[11] = 0x36;
    sim.handler_0x59e5b(&mut st, &layout);
    assert_eq!(st.b, expected);

    st.b[11] = 0x37;
    position(&mut st.b, 100, 100);
    st.b[27] = 0x1F;
    let mut expected = st.b;
    position(&mut expected, 100, 99);
    expected[15] = 4;
    expected[27] = 0x20;
    sim.handler_0x59e5b(&mut st, &layout);
    assert_eq!(st.b, expected);
}

#[test]
fn handler_0x59e5b_ceiling_skips_the_rise() {
    let layout = solid();
    let (mut sim, mut st) = fixture(3);
    sim.fff0 = 0;
    position(&mut st.b, 100, 100);
    st.b[11] = 0x37;
    st.b[27] = 4;
    let mut expected = st.b;
    expected[15] = 4;
    sim.handler_0x59e5b(&mut st, &layout);
    assert_eq!(st.b, expected);
}

#[test]
fn handler_0x59e5b_descent_faces_mario_when_the_rise_is_undone() {
    let layout = air();
    let (mut sim, mut st) = fixture(4);
    sim.fff0 = 0;
    sim.mario_ref_x = 40;
    position(&mut st.b, 100, 100);
    st.b[11] = 0x35;
    st.b[13] = 0x20;
    st.b[27] = 1;
    let mut expected = st.b;
    position(&mut expected, 100, 101);
    expected[13] = 0;
    expected[15] = 1;
    expected[27] = 0;
    sim.handler_0x59e5b(&mut st, &layout);
    assert_eq!(st.b, expected);
    assert_eq!(sim.facing, 255);
}

#[test]
fn handler_0x59e5b_real_mode_b_anim11_step() {
    let tables = tables();
    let layout = air();
    assert_eq!(tables.profile_for(0x0A, 11).unwrap().handler, 0x59E5B);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.mario_ref_x = 200;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 11;
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
    assert_eq!(sim.slots[1].b[18], 76);
    assert_eq!(sim.slots[1].b[25], 48);
    assert_eq!(sim.slots[1].b[15], 2);
    assert!(sim.unported.is_empty());
}
