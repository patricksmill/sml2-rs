use super::*;
use crate::actor::{Actor, Position};

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

#[test]
fn handler_0x59fdf_init_faces_and_arms_a_24_frame_timer() {
    let (mut sim, mut st) = fixture(0);
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    st.b[13] = 0;
    sim.mario_ref_x = 200;
    sim.handler_0x59fdf(&mut st);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[18], 77);
    assert_eq!(st.b[28], 24);
    assert_eq!(sim.facing, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59fdf_counts_down_then_spawns_anim_36() {
    let (mut sim, mut st) = fixture(1);
    st.b[13] = 0x20;
    st.b[28] = 2;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    st.b[3..5].copy_from_slice(&80u16.to_be_bytes());
    sim.handler_0x59fdf(&mut st);
    assert_eq!(st.b[28], 1);
    assert_eq!(st.b[15], 1);
    assert_eq!(sim.standby, 0);

    sim.handler_0x59fdf(&mut st);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[28], 16);
    assert_eq!(sim.standby, 1);
    let child = sim.slots.iter().find(|slot| slot.b[5] == 36).unwrap();
    assert_eq!(child.b[11], 62);
    assert_eq!(child.b[13], 0x20);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59fdf_post_spawn_wait_refaces_with_sprite_59() {
    let (mut sim, mut st) = fixture(3);
    sim.fff0 = 1;
    sim.mario_ref_x = 40;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    st.b[11] = 10;
    st.b[28] = 1;
    sim.handler_0x59fdf(&mut st);
    assert_eq!(st.b[11], 59);
    assert_eq!(st.b[15], 1);
    assert_eq!(st.b[13], 0);
    assert_eq!(st.b[28], 112);
    assert_eq!(sim.facing, 255);
}

#[test]
fn handler_0x59fdf_real_mode_b_anim35_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 35).unwrap().handler, 0x59FDF);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.mario_ref_x = 200;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 80 };
    actor.activity = 2;
    actor.definition = 35;
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
    assert_eq!(sim.slots[1].b[28], 24);
    assert!(sim.unported.is_empty());
}
