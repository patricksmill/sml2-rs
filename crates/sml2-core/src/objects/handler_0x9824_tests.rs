use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    sim.door_sequence_active = 1;
    sim.door_phase = 1;
    sim.boss_slot_ptr_lo = 0x20;
    let mut st = Staging::default();
    st.b[B_DEF] = 7;
    st.b[B_FSTATE] = state;
    (sim, st)
}

#[test]
fn handler_0x9824_arms_only_the_matching_side() {
    let tables = tables();
    let (mut sim, mut st) = fixture(0);
    sim.door_sequence_active = 0;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_FSTATE], 0);

    sim.door_sequence_active = 1;
    sim.door_phase = 2;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_FSTATE], 0);

    sim.door_phase = 1;
    sim.door_parity = 0;
    sim.boss_slot_ptr_lo = 0x60;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_FSTATE], 0);

    sim.boss_slot_ptr_lo = 0x20;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.b[B_HITREACT], 80);
    assert_eq!(st.b[B_T28], 64);
    assert_eq!(st.b[B_SPRITE], 25);

    let (mut sim, mut st) = fixture(0);
    sim.door_phase = 3;
    sim.door_parity = 0;
    sim.boss_slot_ptr_lo = 0x60;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_FSTATE], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x9824_rises_with_a_rider_then_falls_to_the_sill() {
    let tables = tables();
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    st.b[B_T28] = 1;
    st.b[B_FLIP] = 0;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_FLIP], 0x20);
    assert_eq!(st.b[B_FSTATE], 2);
    assert_eq!(st.b[B_SPRITE], 19);
    assert_eq!(sim.sfx2, 5);

    st.b[B_SCRY] = 40;
    st.b[B_HITREACT] = 2;
    st.b[B_FLAGS] = 0x08;
    sim.mario.y = 40;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_SCRY], 38);
    assert_eq!(sim.mario_y_adjust, Some(38));
    assert_eq!(st.b[B_FSTATE], 3);
    assert_eq!(st.b[B_T28], 32);

    st.b[B_T28] = 1;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(sim.door_sequence_active, 0);
    assert_eq!(st.b[B_FSTATE], 4);

    st.b[B_SCRY] = 0x80;
    st.b[B_PHASE] = 2;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_SCRY], 0x81);
    assert_eq!(st.b[B_PHASE], 3);
    assert_eq!(st.b[B_FSTATE], 4);

    st.b[B_SCRY] = 0x84;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_SCRY], 0x84);
    assert_eq!(st.b[B_FSTATE], 0);
}

#[test]
fn handler_0x9824_removal_clears_contact_without_rising() {
    let tables = tables();
    let (mut sim, mut st) = fixture(2);
    st.b[B_SCRY] = 40;
    st.b[B_TYPE] = 69;
    sim.remove_mode = 1;
    sim.handler_0x9824(&mut st, &tables);
    assert_eq!(st.b[B_SCRY], 40);
    assert_eq!(st.b[B_TYPE], 0);
    assert_eq!(st.b[B_FSTATE], 2);
}

#[test]
fn handler_0x9824_real_boss_anim7_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.boss_anim_handler(7), Some(0x9824));
    let mut sim = ObjectSim::new(vec![], 0);
    sim.boss_def_count = 1;
    sim.door_sequence_active = 1;
    sim.door_phase = 1;
    sim.slots[1].b[B_DEF] = 7;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(sim.slots[1].b[B_FSTATE], 1);
    assert_eq!(sim.slots[1].b[B_SPRITE], 25);
    assert!(sim.unported.is_empty());
}
