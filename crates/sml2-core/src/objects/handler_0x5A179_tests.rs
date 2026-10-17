use super::*;
use crate::actor::{Actor, Position};

fn solid() -> LevelLayout {
    LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap()
}

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    sim.fff0 = 1;
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
fn handler_0x5a179_init_faces_and_arms_contact_77() {
    let layout = solid();
    let (mut sim, mut st) = fixture(0);
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    sim.mario_ref_x = 40;
    sim.handler_0x5a179(&mut st, &layout);
    assert_eq!(st.b[13], 0);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[18], 77);
    assert_eq!(sim.facing, 255);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a179_timer_gates_the_proximity_strike() {
    let layout = solid();
    let (mut sim, mut st) = fixture(1);
    position(&mut st.b, 0x0100, 0x60);
    st.b[28] = 2;
    sim.mario_ref_x = 0x0110;
    sim.handler_0x5a179(&mut st, &layout);
    assert_eq!(st.b[28], 1);
    assert_eq!(st.b[15], 1);

    st.b[28] = 0;
    sim.mario_ref_x = 0x0130;
    sim.handler_0x5a179(&mut st, &layout);
    assert_eq!(st.b[15], 1, "distance 0x30 is not inside 0x20");

    sim.mario_ref_x = 0x0110;
    sim.handler_0x5a179(&mut st, &layout);
    assert_eq!(st.b[11], 0x63);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[18], 76);
    assert_eq!(st.b[26], 0);
}

#[test]
fn handler_0x5a179_crouch_and_rise_restore_the_patrol() {
    let layout = solid();
    let (mut sim, mut st) = fixture(3);
    sim.fff0 = 0;
    st.b[11] = 0x62;
    sim.handler_0x5a179(&mut st, &layout);
    assert_eq!(st.b[11], 0x61);
    assert_eq!(st.b[15], 4);
    assert_eq!(st.b[28], 32);

    sim.fff0 = 1;
    st.b[28] = 0;
    st.b[11] = 0x62;
    position(&mut st.b, 100, 100);
    sim.mario_ref_x = 200;
    let sprite = st.b[11];
    sim.handler_0x5a179(&mut st, &layout);
    assert_eq!(st.b[11], sprite);
    assert_eq!(st.b[15], 4);

    sim.fff0 = 0;
    sim.handler_0x5a179(&mut st, &layout);
    assert_eq!(st.b[11], 0x63);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[18], 77);
    assert_eq!(st.b[28], 64);
    assert_eq!(sim.facing, 1);
}

#[test]
fn handler_0x5a179_real_mode_b_anim39_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = solid();
    assert_eq!(tables.profile_for(0x0A, 39).unwrap().handler, 0x5A179);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 39;
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
    assert!(sim.unported.is_empty());
}
