use crate::objects::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn handler_0x9ef4_dispatch_writes_only_selected_child_screen_and_sprite() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.boss_anim_handler(14), Some(0x9EF4));
    for parity in [0u8, 1, 2, 255] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.spawn_boss(3);
        sim.div = 1;
        sim.scene_slot_parity = parity;
        for slot in 1..=2 {
            sim.slots[slot].b =
                std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(slot as u8));
            sim.slots[slot].b[B_FSTATE] = 0;
        }
        let mut expected = [sim.slots[1].b, sim.slots[2].b];
        let mut st = Staging {
            b: sim.slots[0].b,
            px: 0x1234,
            py: 0x5678,
        };
        st.b[B_FSTATE] = 4;
        st.b[B_T24] = 1;
        sim.dispatch_handler(0x9EF4, &mut st, &layout, &tables);
        let selected = usize::from(parity != 0);
        expected[selected][B_SCRY] = 143;
        expected[selected][B_SCRX] = 88;
        expected[selected][B_SPRITE] = 49;
        assert_eq!([sim.slots[1].b, sim.slots[2].b], expected);
        assert_eq!(sim.scene_slot_parity, parity ^ 1);
        assert_eq!(sim.boss_def_count, 2, "commands reuse existing children");
        assert_eq!((st.b[B_FSTATE], st.b[B_T28], st.b[B_SPRITE]), (5, 48, 47));
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x9ef4_real_driver_stomp_stun_defeat_and_death_morph() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(3);
    // Isolate main-slot damage and morph from child behavior.
    sim.boss_def_count = 0;
    sim.slots[0].b[B_FSTATE] = 5;
    sim.slots[0].b[B_SPRITE] = 45;
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
        (5, 45, 69)
    );
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_SPRITE]
        ),
        (1, 104, 48)
    );
    for _ in 0..104 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_SPRITE]
        ),
        (2, 32, 48)
    );
    for _ in 0..32 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!((sim.slots[0].b[B_DEF], sim.slots[0].b[B_FSTATE]), (3, 0));
    assert_eq!(sim.music_request, Some(255));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x9ef4_projectile_preserves_timer_then_restores_saved_fields() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(3);
    sim.boss_def_count = 0;
    sim.slots[0].b[B_FSTATE] = 4;
    sim.slots[0].b[B_SPRITE] = 46;
    sim.slots[0].b[B_TYPE] = 69;
    sim.slots[0].b[B_T28] = 2;
    sim.slots[0].b[B_FLAGS] = 2;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!((sim.boss_hits_remaining, sim.sfx1), (5, 9));
    assert_eq!((sim.slots[0].b[B_SPRITE], sim.slots[0].b[B_T28]), (46, 2));
    for _ in 0..2 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_SPRITE],
            sim.slots[0].b[B_TYPE]
        ),
        (4, 46, 69)
    );
    assert_eq!(sim.remove_mode, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x9ef4_busy_child_does_not_switch_slot_or_overwrite_other_child() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(3);
    sim.div = 1;
    sim.slots[1].b[B_FSTATE] = 1;
    let expected = [sim.slots[1].b, sim.slots[2].b];
    let mut st = Staging::default();
    st.b[B_FSTATE] = 4;
    st.b[B_T24] = 1;
    sim.dispatch_handler(0x9EF4, &mut st, &layout, &tables);
    assert_eq!([sim.slots[1].b, sim.slots[2].b], expected);
    assert_eq!((sim.scene_slot_parity, st.b[B_SPRITE]), (0, 44));
}

#[test]
fn handler_0x9ef4_child_selection_resets_on_entry_and_level_reload() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.scene_slot_parity = 255;
    sim.spawn_boss(3);
    assert_eq!(sim.scene_slot_parity, 0);
    sim.scene_slot_parity = 255;
    sim.reset_level_start();
    assert_eq!(sim.scene_slot_parity, 0);
}
