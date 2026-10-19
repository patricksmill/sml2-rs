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
fn handler_0xa372_real_driver_completes_all29_motion_states_with_source_durations() {
    let tables = tables();
    let layout = layout();
    assert_eq!(tables.boss_anim_handler(19), Some(0xA372));
    for div in [0, 1, 63, 64, 255] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.spawn_boss(5);
        sim.div = div;
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(
            (
                sim.slots[0].b[B_FSTATE],
                sim.slots[0].b[B_TYPE],
                sim.slots[0].b[B_T25]
            ),
            (3, 69, 36)
        );
        let mut duration = [0; 32];
        let mut cycled = false;
        for _ in 0..2000 {
            let state = sim.slots[0].b[B_FSTATE];
            duration[state as usize] += 1;
            sim.boss_engine(0, 0, &layout, &tables);
            assert!(sim.unported.is_empty(), "DIV={div}, state {state}");
            if state == 31 && sim.slots[0].b[B_FSTATE] == 3 {
                cycled = true;
                break;
            }
        }
        assert!(cycled);
        // Includes state-7's immediate state-8 move, countdown return ticks,
        // and the extra sentinel-only tick at the end of every path.
        assert_eq!(
            &duration[3..],
            &[
                18, 26, 38, 13, 28, 17, 82, 17, 40, 13, 68, 13, 14, 32, 14, 12, 18, 82, 12, 42, 13,
                68, 13, 14, 32, 28, 13, 60, 18
            ]
        );
        assert_eq!(
            (
                sim.slots[0].b[B_T25],
                sim.slots[0].b[B_T28],
                sim.slots[0].b[B_FLIP],
                sim.slots[0].b[B_TYPE]
            ),
            (36, 64, 0, 69)
        );
        assert_eq!(
            (sim.slots[0].b[B_DEFEAT], sim.slots[0].b[B_FLASH]),
            (128, 16)
        );
    }
}

#[test]
fn handler_0xa372_driver_stomp_defeat_stun_and_death_morph_order() {
    let tables = tables();
    let layout = layout();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(5);
    sim.slots[0].b[B_FSTATE] = 15;
    sim.slots[0].b[B_SPRITE] = 69;
    sim.slots[0].b[B_TYPE] = 69;
    sim.slots[0].b[B_FLAGS] = 1;
    sim.boss_hits_remaining = 2;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(
        (
            sim.boss_hits_remaining,
            sim.remove_mode,
            sim.boss_clear_flag
        ),
        (0, 1, 1)
    );
    assert_eq!(sim.fade_timer_set, Some(640));
    assert_eq!(
        (sim.saved_frame_state, sim.spawn_sprite, sim.saved_type),
        (15, 69, 69)
    );
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_SPRITE]
        ),
        (1, 48, 74)
    );
    for _ in 0..48 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_SPRITE]
        ),
        (2, 32, 72)
    );
    for _ in 0..32 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(
        (
            sim.slots[0].b[B_DEF],
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_FLASH]
        ),
        (3, 0, 16)
    );
    assert_eq!(sim.music_request, Some(255));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0xa372_projectile_preserves_timer_and_sprite_then_restores_saved_state() {
    let tables = tables();
    let layout = layout();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(5);
    sim.slots[0].b[B_FSTATE] = 9;
    sim.slots[0].b[B_SPRITE] = 66;
    sim.slots[0].b[B_TYPE] = 69;
    sim.slots[0].b[B_FLAGS] = 2;
    sim.slots[0].b[B_T28] = 2;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!((sim.boss_hits_remaining, sim.sfx1), (5, 9));
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_SPRITE]
        ),
        (1, 2, 66)
    );
    for _ in 0..2 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_SPRITE],
            sim.slots[0].b[B_TYPE]
        ),
        (9, 66, 69)
    );
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0xa372_dispatch_sound_mapping_and_invalid_animation_diagnostic() {
    let tables = tables();
    let layout = layout();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 2;
    sim.div = 64;
    sim.sfx0 = 0xAB;
    let mut st = Staging {
        b: [0; 32],
        px: 0x1234,
        py: 0x5678,
    };
    st.b[B_FSTATE] = 17;
    st.b[B_T25] = 8;
    st.b[B_SPRITE] = 66;
    sim.dispatch_handler(0xA372, &mut st, &layout, &tables);
    assert_eq!((sim.sfx0, sim.sfx1, sim.sfx2), (0xAB, 3, 7));
    assert_eq!((st.px, st.py), (0x1234, 0x5678));
    assert_eq!((st.b[B_SCRX], st.b[B_T25], st.b[B_FLASH]), (2, 6, 16));
    sim.fff0 = 0;
    st.b[B_ANIMFRAME] = 4;
    let mut expected = st.b;
    expected[B_ANIMFRAME] = 5;
    sim.dispatch_handler(0xA372, &mut st, &layout, &tables);
    assert_eq!(st.b, expected);
    assert_eq!(sim.unported, vec![0xA372]);
}
