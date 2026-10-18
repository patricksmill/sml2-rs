use crate::objects::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn layout() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap()
}

#[test]
fn handler_0xa029_real_parent_command_drop_flight_and_idle_slot_reuse() {
    let tables = tables();
    let layout = layout();
    assert_eq!(tables.boss_anim_handler(15), Some(0xA029));
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(3);
    sim.div = 1;
    sim.fff0 = 1;
    sim.slots[0].b[B_FSTATE] = 4;
    sim.slots[0].b[B_T24] = 1;
    sim.slots[0].b[B_T25] = 32;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(
        (
            sim.slots[1].b[B_FSTATE],
            sim.slots[1].b[B_SPRITE],
            sim.slots[1].b[B_TYPE]
        ),
        (3, 49, 69)
    );
    assert_eq!((sim.slots[1].b[B_SCRY], sim.slots[1].b[B_SCRX]), (143, 88));
    assert_eq!(
        (sim.slots[2].b[B_FSTATE], sim.slots[2].b[B_SPRITE]),
        (0, 14)
    );
    for _ in 0..15 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(
        (
            sim.slots[1].b[B_FSTATE],
            sim.slots[1].b[B_SPRITE],
            sim.slots[1].b[B_PHASE]
        ),
        (2, 50, 0)
    );
    assert_eq!(
        (
            sim.slots[1].b[B_SCRY],
            sim.slots[1].b[B_SCRX],
            sim.slots[1].b[B_T25]
        ),
        (167, 88, 32)
    );
    assert_eq!(sim.facing, 1);
    let mut returned = false;
    for _ in 0..512 {
        sim.boss_engine(0, 0, &layout, &tables);
        assert!(sim.unported.is_empty());
        if sim.slots[1].b[B_FSTATE] == 0 {
            returned = true;
            break;
        }
    }
    assert!(returned, "child failed to return to idle");
    assert_eq!(
        (sim.slots[1].b[B_SPRITE], sim.slots[1].b[B_SCRX]),
        (14, 184)
    );
    assert_eq!(
        sim.boss_def_count, 2,
        "source keeps both reusable child slots"
    );
    assert!(
        !crate::scene::boss_def_portable(3),
        "full arena acceptance is still separate"
    );
}

#[test]
fn handler_0xa029_removal_skips_damage_and_only_slot_2_morphs_whole_record() {
    let tables = tables();
    let layout = layout();
    for slot in [0x20, 0x40, 0x60] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.spawn_boss(3);
        sim.remove_mode = 2;
        sim.boss_slot_ptr_lo = slot;
        sim.facing = 0xA5;
        sim.sfx0 = 0x55;
        sim.sfx1 = 0xAA;
        let mut st = Staging {
            b: [0xA5; 32],
            px: 0x1234,
            py: 0x5678,
        };
        st.b[B_FLAGS] = 3;
        let mut expected = st.b;
        sim.dispatch_handler(0xA029, &mut st, &layout, &tables);
        if slot == 0x40 {
            expected[B_SCRY] = 112;
            expected[B_SCRX] = 88;
            expected[B_DEF] = 4;
            expected[B_SPRITE] = 9;
            expected[B_FSTATE] = 0;
            expected[B_PHASE] = 0;
            expected[B_TYPE] = 0;
        }
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert_eq!(
            (
                sim.boss_hits_remaining,
                sim.boss_def_count,
                sim.done,
                sim.boss_hit_latch
            ),
            (6, 2, 0, 0)
        );
        assert_eq!((sim.facing, sim.sfx0, sim.sfx1), (0xA5, 0x55, 0xAA));
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0xa029_reward_morph_runs_existing_anim4_and_spawns_anim5() {
    let tables = tables();
    let layout = layout();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(3);
    sim.remove_mode = 2;
    sim.slots[0].b[B_DEF] = 0;
    sim.slots[2].b[B_FLIP] = 0x60;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(
        (
            sim.slots[2].b[B_DEF],
            sim.slots[2].b[B_SCRY],
            sim.slots[2].b[B_SCRX]
        ),
        (4, 112, 88)
    );
    assert_eq!(sim.slots[1].b[B_DEF], 15);
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(
        (
            sim.slots[2].b[B_FSTATE],
            sim.slots[2].b[B_PHASE],
            sim.slots[2].b[B_SCRY]
        ),
        (1, 0, 112)
    );
    assert_eq!(
        (
            sim.slots[3].b[B_DEF],
            sim.slots[3].b[B_SPRITE],
            sim.slots[3].b[B_FLIP]
        ),
        (5, 12, 0x60)
    );
    assert_eq!(sim.boss_def_count, 3);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0xa029_child_hits_freeze_staging_without_damaging_primary_or_sounds() {
    for flags in [1, 2, 3] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.spawn_boss(3);
        sim.boss_slot_ptr_lo = 0x20;
        sim.fff0 = 0;
        sim.facing = 0xA5;
        sim.sfx0 = 0x55;
        sim.sfx1 = 0xAA;
        let mut st = Staging::default();
        st.b[B_FSTATE] = 1;
        st.b[B_SPRITE] = 50;
        st.b[B_FLAGS] = flags;
        let expected = st.b;
        sim.handler_patrol_child(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!(
            (sim.done, sim.boss_hits_remaining, sim.remove_mode),
            (1, 6, 0)
        );
        assert_eq!(sim.boss_hit_latch, u8::from(flags & 1 != 0));
        assert_eq!((sim.facing, sim.sfx0, sim.sfx1), (0xA5, 0x55, 0xAA));
    }
}

#[test]
fn handler_0xa029_vertical_tracking_consumes_live_projected_player_y() {
    let tables = tables();
    let layout = layout();
    for (player_y, cam_y, projected, next_y, facing) in [
        (82, 80, 100, 101, 1),
        (82, 88, 92, 99, 255),
        (82, 72, 108, 101, 1),
        (16, 128, 242, 101, 1),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.spawn_boss(3);
        sim.slots[0].b[B_DEF] = 0;
        sim.boss_def_count = 1;
        sim.slots[1].b[B_FSTATE] = 2;
        sim.slots[1].b[B_SCRX] = 88;
        sim.slots[1].b[B_SCRY] = 100;
        sim.slots[1].b[B_SPRITE] = 50;
        sim.fff0 = 3;
        sim.mario.x = 136;
        sim.mario.y = player_y;
        sim.boss_engine(120, cam_y, &layout, &tables);
        assert_eq!(sim.player_screen_y, projected);
        assert_eq!(
            (
                sim.slots[1].b[B_SCRX],
                sim.slots[1].b[B_SCRY],
                sim.slots[1].b[B_SPRITE]
            ),
            (89, next_y, 51)
        );
        assert_eq!(sim.facing, facing);
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0xa029_invalid_drop_phase_reports_diagnostic_preserving_staging() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.boss_slot_ptr_lo = 0x20;
    let mut st = Staging::default();
    st.b[B_FSTATE] = 3;
    st.b[B_PHASE] = 16;
    let expected = st.b;
    sim.handler_patrol_child(&mut st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.unported, vec![0xA029]);
}
