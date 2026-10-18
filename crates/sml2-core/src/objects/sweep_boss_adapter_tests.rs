use crate::objects::*;
fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn handler_0xa0b9_dispatch_independent_busy_children_full_flip_and_preserved_bytes() {
    let tables = tables();
    assert_eq!(tables.boss_anim_handler(16), Some(0xA0B9));
    // Pin extracted profiles to independently transcribed source samples.
    assert_eq!(
        &tables.boss_picker_profile[..13],
        &[0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2]
    );
    assert_eq!(
        &tables.boss_sweep_profile[..27],
        &[0, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0]
    );
    assert_eq!(&tables.boss_sweep_profile[27..], &[0; 85]);
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    for state in [3, 6] {
        for busy in [[0u8, 0], [1, 0], [0, 255], [1, 255]] {
            let mut sim = ObjectSim::new(vec![], 0);
            sim.spawn_boss(4);
            sim.door_phase = 4;
            for (index, busy) in busy.into_iter().enumerate() {
                sim.slots[index + 1].b = std::array::from_fn(|i| (i as u8).wrapping_mul(19));
                sim.slots[index + 1].b[B_FSTATE] = busy;
            }
            let mut expected = [sim.slots[1].b, sim.slots[2].b];
            let mut st = Staging::default();
            st.b[B_FSTATE] = state;
            st.b[B_T28] = 1;
            st.b[B_SCRX] = 2;
            st.b[B_SCRY] = 254;
            st.b[B_FLIP] = 0xDF;
            st.px = 0x1234;
            st.py = 0x5678;
            sim.dispatch_handler(0xA0B9, &mut st, &layout, &tables);
            for index in 0..2 {
                if busy[index] == 0 {
                    expected[index][B_SCRY] = if index == 0 { 1 } else { 17 };
                    expected[index][B_SCRX] = if index == 0 {
                        if state == 3 {
                            250
                        } else {
                            10
                        }
                    } else {
                        2
                    };
                    expected[index][B_SPRITE] = if index == 0 { 59 } else { 60 };
                    expected[index][B_FLIP] = 0xDF;
                }
            }
            assert_eq!([sim.slots[1].b, sim.slots[2].b], expected);
            assert_eq!((sim.door_phase, st.b[B_T28]), (3, 96));
            assert_eq!((st.px, st.py), (0x1234, 0x5678));
            assert!(sim.unported.is_empty());
        }
    }
}

#[test]
fn handler_0xa0b9_real_driver_stomp_delay_defeat_and_morph() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(4);
    sim.boss_def_count = 0;
    sim.slots[0].b[B_FSTATE] = 6;
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
        (6, 52, 69)
    );
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_SPRITE]
        ),
        (9, 1, 53)
    );
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(
        (sim.slots[0].b[B_FSTATE], sim.slots[0].b[B_T28], sim.sfx1),
        (1, 80, 5)
    );
    for _ in 0..80 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_SPRITE]
        ),
        (2, 32, 54)
    );
    for _ in 0..32 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!((sim.slots[0].b[B_DEF], sim.slots[0].b[B_FSTATE]), (3, 0));
    assert_eq!(sim.music_request, Some(255));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0xa0b9_projectile_preserves_timer_and_sprite_then_recovers() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(4);
    sim.boss_def_count = 0;
    sim.slots[0].b[B_FSTATE] = 8;
    sim.slots[0].b[B_TYPE] = 69;
    sim.slots[0].b[B_T28] = 2;
    sim.slots[0].b[B_FLAGS] = 2;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!((sim.boss_hits_remaining, sim.sfx1), (5, 9));
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_SPRITE]
        ),
        (1, 2, 52)
    );
    for _ in 0..2 {
        sim.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_TYPE],
            sim.slots[0].b[B_SPRITE]
        ),
        (8, 69, 52)
    );
    assert_eq!(sim.remove_mode, 0);
    assert!(sim.unported.is_empty());
}
