use super::*;
use crate::actor::{Actor, Position};

fn air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
}

#[test]
fn handler_0x59fb9_init_faces_and_arms_flash_and_contact() {
    let layout = air();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    sim.mario_ref_x = 50;
    let mut st = Staging::default();
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    sim.handler_0x59fb9(&mut st, &layout);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], handler_0x59fb9::CONTACT);
    assert_eq!(sim.facing, 255);
}

#[test]
fn handler_0x59fb9_even_frame_walks_off_the_ledge() {
    let layout = air();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = Staging::default();
    st.b[15] = 1;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    st.b[3..5].copy_from_slice(&100u16.to_be_bytes());
    sim.handler_0x59fb9(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 99);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[11], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59fb9_odd_frame_does_not_walk() {
    let layout = air();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    let mut st = Staging::default();
    st.b[15] = 1;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    sim.handler_0x59fb9(&mut st, &layout);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 100);
    assert_eq!(st.b[15], 1);
}

#[test]
fn handler_0x59fb9_real_mode_b_anim32_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = air();
    assert_eq!(tables.profile_for(0x0A, 32).unwrap().handler, 0x59FB9);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    sim.mario_ref_x = 50;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 32;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        100,
        100,
        &layout,
        &tables,
        &MarioContact {
            x: 40,
            ..Default::default()
        },
        false,
    );
    assert_eq!(sim.slots[1].b[18], 77);
    assert_eq!(sim.slots[1].b[14], 16);
    assert!(sim.unported.is_empty());
}
