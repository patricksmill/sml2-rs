use crate::objects::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn handler_0x9927_counted_remove_clears_entire_slot_and_wraps_count() {
    for count in [0u8, 1, 4, 255] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.boss_def_count = count;
        let mut st = Staging {
            b: [0xA5; 32],
            px: 0x1234,
            py: 0x5678,
        };
        st.b[B_FSTATE] = 3;
        st.b[B_T28] = 1;
        sim.handler_door_child(&mut st);
        assert_eq!(st.b, [0; 32]);
        assert_eq!(sim.boss_def_count, count.wrapping_sub(1));
        assert_eq!(sim.done, 1);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x9927_real_parent_spawns_and_dispatches_child() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.boss_anim_handler(9), Some(0x9927));
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(1);
    sim.div = 2;
    sim.fff0 = 7;
    sim.slots[0].b[B_FSTATE] = 3;
    sim.slots[0].b[B_ANIMFRAME] = 1;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(sim.boss_def_count, 4);
    assert_eq!(sim.door_phase, 2);
    assert_eq!(sim.slots[4].b[B_DEF], 9);
    // The driver latches its iteration count before spawning; init next frame.
    assert_eq!(sim.slots[4].b[B_FSTATE], 0);
    sim.boss_engine(0, 0, &layout, &tables);
    let child = sim.slots[4].b;
    assert_eq!((child[B_SCRY], child[B_SCRX]), (84, 136));
    assert_eq!((child[B_FSTATE], child[B_TYPE]), (1, 64));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x9927_signal_drives_real_central_platform_and_removes_child() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.boss_def_count = 2;
    sim.door_phase = 2;
    sim.slots[1].b[B_DEF] = 8;
    sim.slots[1].b[B_SCRY] = 116;
    sim.slots[2].b[B_DEF] = 9;
    sim.slots[2].b[B_SCRY] = 112;
    sim.slots[2].b[B_SCRX] = 152;
    sim.slots[2].b[B_SPRITE] = 26;
    // Child init + 60 trajectory steps + sentinel + 32 pause ticks.
    for _ in 0..94 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(sim.door_sequence_active, 1);
    assert_eq!(sim.slots[2].b[B_FSTATE], 4);
    assert_eq!((sim.slots[2].b[B_SCRY], sim.slots[2].b[B_SCRX]), (144, 88));
    assert_eq!(sim.slots[1].b[B_FSTATE], 0);
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(sim.slots[1].b[B_FSTATE], 1);
    // Platform flashes, rises 64px, then pauses before clearing the signal.
    for _ in 0..128 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(sim.slots[1].b[B_SCRY], 52);
    assert_eq!(sim.slots[1].b[B_FSTATE], 4);
    assert_eq!(sim.door_sequence_active, 0);
    assert_eq!(sim.slots[2].b, [0; 32]);
    assert_eq!(sim.boss_def_count, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x9927_invalid_phase_reports_diagnostic_and_keeps_animation() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.door_phase = 2;
    let mut st = Staging::default();
    st.b[B_FSTATE] = 1;
    st.b[B_PHASE] = 61;
    sim.handler_door_child(&mut st);
    assert_eq!(st.b[B_SPRITE], 1);
    assert_eq!(st.b[B_PHASE], 61);
    assert_eq!(sim.unported, vec![0x9927]);
}
