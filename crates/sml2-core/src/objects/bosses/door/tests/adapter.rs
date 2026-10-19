use super::*;
use crate::actor::InteractionFlags;

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    sim.boss_hits_remaining = 6;
    sim.boss_timer_reload = 80;
    sim.boss_sprite = 9;
    let mut st = Staging::default();
    st.b[B_DEF] = 6;
    st.b[B_FSTATE] = state;
    (sim, st)
}

#[test]
fn handler_0x96fe_intro_waits_out_the_fade_then_places_the_door() {
    let (mut sim, mut st) = fixture(0);
    sim.fade_timer = 0x0280;
    st.b[B_T28] = 0x6F;
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(st.b[B_T28], 0x6F);

    sim.fade_timer = 0;
    sim.div = 1;
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_T28], 0);
    assert_eq!(st.b[B_FSTATE], 3);
    assert_eq!(st.b[B_ANIMFRAME], 8);
    assert_eq!(st.b[B_SCRY], 112);
    assert_eq!(st.b[B_SCRX], 24);
    assert_eq!(st.b[B_FLIP], 0x20);
    assert_eq!(sim.door_parity, 1);
    assert_eq!(sim.sfx1, 2);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x96fe_stomp_banks_the_slot_and_a_clear_falls() {
    let (mut sim, mut st) = fixture(3);
    st.b[B_SPRITE] = 20;
    st.b[B_TYPE] = 69;
    st.b[B_FLAGS] = InteractionFlags::from_bits(0x01).bits();
    sim.handler_0x96fe(&mut st);
    assert_eq!(sim.spawn_sprite, 20);
    assert_eq!(sim.saved_frame_state, 3);
    assert_eq!(sim.saved_type, 69);
    assert_eq!(sim.boss_hit_latch, 1);
    assert_eq!(st.b[B_SPRITE], 17);
    assert_eq!(st.b[B_T28], 80);
    assert_eq!(st.b[B_TYPE], 0);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(sim.boss_hits_remaining, 4);

    let (mut sim, mut st) = fixture(1);
    st.b[B_T28] = 1;
    sim.remove_mode = 1;
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_FSTATE], 2);
    assert_eq!(st.b[B_T28], 0x20);
    assert_eq!(st.b[B_SPRITE], 9);
}

#[test]
fn handler_0x96fe_mouth_spawns_then_returns_to_the_intro() {
    let (mut sim, mut st) = fixture(3);
    st.b[B_ANIMFRAME] = 0;
    st.b[B_FLIP] = 0x20;
    sim.div = 2;
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_FSTATE], 4);
    assert_eq!(st.b[B_SPRITE], 16);
    assert_eq!(st.b[B_TYPE], 69);
    assert_eq!(st.b[B_T28], 112);
    assert_eq!(sim.door_phase, 2);
    assert_eq!(sim.sfx2, 6);
    assert_eq!(sim.slots[1].b[B_DEF], 9);
    assert_eq!(sim.slots[1].b[B_SPRITE], 26);
    assert_eq!(sim.slots[1].b[B_FLIP], 0x20);
    assert_eq!(sim.boss_def_count, 1);

    st.b[B_T28] = 1;
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_FSTATE], 5);
    assert_eq!(st.b[B_SPRITE], 20);
    assert_eq!(st.b[B_TYPE], 0);

    sim.fff0 = 0;
    st.b[B_SPRITE] = 0x17;
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_SPRITE], 0x18);
    assert_eq!(st.b[B_FSTATE], 5);
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_SPRITE], 14);
    assert_eq!(st.b[B_FSTATE], 6);
    assert_eq!(st.b[B_T24], 48);
    assert_eq!(st.b[B_HITREACT], 0);

    st.b[B_T24] = 2;
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_FSTATE], 7);
    sim.handler_0x96fe(&mut st);
    assert_eq!(st.b[B_FSTATE], 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x96fe_real_boss_anim6_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.boss_anim_handler(6), Some(0x96FE));
    let mut sim = ObjectSim::new(vec![], 0);
    sim.slots[0].b[B_DEF] = 6;
    sim.slots[0].b[B_T28] = 0x6F;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(sim.slots[0].b[B_FSTATE], 3);
    assert_eq!(sim.slots[0].b[B_ANIMFRAME], 8);
    assert_eq!(sim.sfx1, 2);
    assert!(sim.unported.is_empty());
}
