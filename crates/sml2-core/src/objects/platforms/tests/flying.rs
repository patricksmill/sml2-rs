use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

fn air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
}
fn solid() -> LevelLayout {
    LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap()
}

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    sim.slot_ptr_hi = 0xAD;
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
fn handler_0x5a545_init_faces_and_spawns_anim_69() {
    let layout = air();
    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 1;
    position(&mut st.b, 100, 80);
    st.b[13] = 0;
    sim.mario_ref_x = 200;
    sim.handler_0x5a545(&mut st, &layout);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[18], 32);
    assert_eq!(sim.facing, 1);
    assert_eq!(sim.standby, 1);
    let child = sim.slots.iter().find(|slot| slot.b[5] == 69).unwrap();
    assert_eq!(child.b[11], 168);
    assert_eq!(child.b[13], 0x20);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a545_even_air_walks_and_rises() {
    let layout = air();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 2;
    position(&mut st.b, 100, 100);
    st.b[16] = 0;
    st.b[24] = 0;
    sim.handler_0x5a545(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 98);
    assert_eq!(st.b[16], 1);
    assert_eq!(sim.mario_x_adjust, None);
}

#[test]
fn handler_0x5a545_left_wall_flips_without_moving() {
    let layout = solid();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 2;
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    st.b[16] = 3;
    sim.handler_0x5a545(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 100);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[16], 3);
}

#[test]
fn handler_0x5a545_phase_eight_toggles_the_vertical_leg() {
    let layout = air();
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 2;
    sim.mario.x = 40;
    sim.mario.y = 40;
    position(&mut st.b, 100, 100);
    st.b[16] = 7;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[24] = 0;
    sim.handler_0x5a545(&mut st, &layout);
    assert_eq!(st.b[16], 0);
    assert_eq!(st.b[24], 1);
    assert_eq!(sim.mario_x_adjust, Some(41));
    assert_eq!(sim.mario_y_adjust, Some(40));
}

#[test]
fn handler_0x5a545_real_mode_b_anim68_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = air();
    assert_eq!(tables.profile_for(0x0A, 68).unwrap().handler, 0x5A545);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    sim.slot_ptr_hi = 0xAD;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 68;
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
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[14], 16);
    assert!(sim.unported.is_empty());
}
