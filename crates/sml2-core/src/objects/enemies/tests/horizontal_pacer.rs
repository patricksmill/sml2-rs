use super::*;
use crate::actor::{Actor, Position};

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
fn handler_0x5a30e_init_faces_and_loads_both_legs() {
    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    sim.mario_ref_x = 200;
    sim.handler_0x5a30e(&mut st);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[18], 72);
    assert_eq!(st.b[24], 112);
    assert_eq!(st.b[25], 56);
    assert_eq!(sim.facing, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a30e_even_steps_reload_the_long_leg_and_flip() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    position(&mut st.b, 100, 100);
    st.b[11] = 10;
    st.b[13] = 0;
    st.b[24] = 112;
    st.b[25] = 1;
    sim.handler_0x5a30e(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(st.b[11], 11);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[25], 112);
}

#[test]
fn handler_0x5a30e_odd_frame_leaves_the_walker_alone() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[25] = 10;
    let expected = st.b;
    sim.handler_0x5a30e(&mut st);
    assert_eq!(st.b, expected);
}

#[test]
fn handler_0x5a30e_real_mode_b_anim65_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 65).unwrap().handler, 0x5A30E);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 65;
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
    assert_eq!(sim.slots[1].b[24], 112);
    assert_eq!(sim.slots[1].b[25], 56);
    assert_eq!(sim.slots[1].b[27], 0);
    assert!(sim.unported.is_empty());

    let mario = MarioContact {
        x: 1000,
        ..Default::default()
    };
    let mut x = 100u16;
    for _ in 0..8 {
        sim.step(100, 100, &layout, &tables, &mario, false);
        assert_eq!(
            u16::from_be_bytes([sim.slots[1].b[3], sim.slots[1].b[4]]),
            100
        );
        assert_eq!(sim.slots[1].b[27], 0);
        let next = u16::from_be_bytes([sim.slots[1].b[0], sim.slots[1].b[1]]);
        assert!(
            next == x || next == x + 1,
            "pacer x jumped from {x} to {next}"
        );
        x = next;
    }
    assert_eq!(x, 102);
}
