use super::*;
use crate::actor::{Actor, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    let mut st = Staging::default();
    st.b[15] = state;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5b177_init_faces_and_blinks_on_quarter_frames() {
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 100, 80);
    st.b[11] = 4;
    sim.mario_ref_x = 200;
    sim.handler_0x5b177(&mut st);
    assert_eq!(st.b[11], 4);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[14], 16);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[18], 77);
    assert_eq!(sim.facing, 1);

    sim.fff0 = 0;
    st.b[15] = 0;
    sim.handler_0x5b177(&mut st);
    assert_eq!(st.b[11], 5);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5b177_even_frames_rise_then_fall() {
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 100, 80);
    sim.handler_0x5b177(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 100);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 80);

    sim.fff0 = 0;
    sim.handler_0x5b177(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 98);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 76);
    assert_eq!(st.b[16], 1);
    assert_eq!(st.b[24], 0);

    st.b[16] = 20;
    sim.handler_0x5b177(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 75);
    assert_eq!(st.b[16], 0);
    assert_eq!(st.b[24], 1);

    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 0;
    position(&mut st.b, 100, 80);
    st.b[16] = 21;
    st.b[24] = 1;
    sim.handler_0x5b177(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 102);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 84);
    assert_eq!(st.b[16], 0);
    assert_eq!(st.b[24], 0);
}

#[test]
fn handler_0x5b177_real_mode_b_anim91_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 91).unwrap().handler, 0x5B177);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 91;
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
    assert_eq!(sim.slots[1].b[14], 16);
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(sim.slots[1].b[18], 77);
    assert!(sim.unported.is_empty());
}
