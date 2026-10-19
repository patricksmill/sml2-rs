use super::*;
use crate::actor::{Actor, Position};

fn air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
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
fn handler_0x5a0aa_init_faces_and_loads_span_48() {
    let layout = air();
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    sim.mario_ref_x = 200;
    sim.handler_0x5a0aa(&mut st, &layout);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[18], 77);
    assert_eq!(st.b[25], 48);
    assert_eq!(sim.facing, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a0aa_span_expiry_spawns_anim_38() {
    let layout = air();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    position(&mut st.b, 100, 80);
    st.b[13] = 0x20;
    st.b[25] = 1;
    sim.handler_0x5a0aa(&mut st, &layout);
    assert_eq!(st.b[11], 66);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[25], 0);
    assert_eq!(st.b[26], 0);
    assert_eq!(st.b[28], 16);
    assert_eq!(sim.sfx1, 17);
    assert_eq!(sim.standby, 1);
    let child = sim.slots.iter().find(|slot| slot.b[5] == 38).unwrap();
    assert_eq!(child.b[11], 69);
    assert_eq!(child.b[13], 0x20);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a0aa_pause_hops_three_pixels_and_reloads_span_88() {
    let layout = air();
    let (mut sim, mut st) = fixture(3);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    st.b[25] = 5;
    st.b[28] = 1;
    sim.handler_0x5a0aa(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 103);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[25], 88);
    assert_eq!(st.b[28], 0);
}

#[test]
fn handler_0x5a0aa_even_patrol_walks_left_while_the_span_counts() {
    let layout = air();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    position(&mut st.b, 100, 100);
    st.b[11] = 10;
    st.b[25] = 10;
    st.b[26] = 0;
    sim.handler_0x5a0aa(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(st.b[25], 9);
    assert_eq!(st.b[11], 11);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a0aa_real_mode_b_anim37_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = air();
    assert_eq!(tables.profile_for(0x0A, 37).unwrap().handler, 0x5A0AA);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 37;
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
    assert_eq!(sim.slots[1].b[18], 77);
    assert_eq!(sim.slots[1].b[25], 48);
    assert!(sim.unported.is_empty());
}
