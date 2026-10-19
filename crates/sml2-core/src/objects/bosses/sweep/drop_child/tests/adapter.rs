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
fn handler_0xa2be_dispatch_reward_ignores_hits_state_and_slot_pointer() {
    let tables = tables();
    let layout = layout();
    assert_eq!(tables.boss_anim_handler(18), Some(0xA2BE));
    assert_eq!(
        &tables.boss_picker_profile[..22],
        &[0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3]
    );
    for ptr in [0, 0x20, 0x40, 0x60] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.spawn_boss(4);
        sim.remove_mode = 2;
        sim.boss_slot_ptr_lo = ptr;
        sim.sfx0 = 0x67;
        sim.sfx1 = 0x89;
        sim.sfx2 = 0xAB;
        let mut st = Staging {
            b: [0xA5; 32],
            px: 0x1234,
            py: 0x5678,
        };
        st.b[B_FLAGS] = 3;
        let mut expected = st.b;
        expected[B_DEF] = 4;
        expected[B_SCRY] = 128;
        expected[B_SCRX] = 88;
        expected[B_SPRITE] = 9;
        expected[B_FSTATE] = 0;
        expected[B_PHASE] = 0;
        expected[B_TYPE] = 0;
        sim.dispatch_handler(0xA2BE, &mut st, &layout, &tables);
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert_eq!((sim.sfx0, sim.sfx1, sim.sfx2), (0x67, 0x89, 0xAB));
        assert_eq!(
            (sim.boss_hits_remaining, sim.done, sim.boss_hit_latch),
            (6, 0, 0)
        );
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0xa2be_parent_launch_drop_approach_rise_dissolve_and_reuse() {
    let tables = tables();
    let layout = layout();
    for (state, flip) in [(3, 0xC7), (6, 0xE7)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.spawn_boss(4);
        sim.door_phase = 2;
        sim.slots[0].b[B_FSTATE] = state;
        sim.slots[0].b[B_T28] = 1;
        sim.slots[0].b[B_FLIP] = flip;
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(
            (
                sim.slots[2].b[B_FSTATE],
                sim.slots[2].b[B_SPRITE],
                sim.slots[2].b[B_TYPE]
            ),
            (3, 60, 64)
        );
        assert_eq!(
            (
                sim.slots[2].b[B_SCRX],
                sim.slots[2].b[B_SCRY],
                sim.slots[2].b[B_FLIP]
            ),
            (144, 67, flip)
        );
        assert_eq!(sim.slots[1].b[B_SPRITE], 59);
        // Freeze further volleys while both children complete their behavior.
        sim.slots[0].b[B_DEF] = 0;
        let mut dropped = false;
        for _ in 0..80 {
            sim.boss_engine(0, 0, &layout, &tables);
            if sim.slots[2].b[B_FSTATE] != 3 {
                dropped = true;
                break;
            }
        }
        assert!(dropped);
        assert_eq!(
            (
                sim.slots[2].b[B_FSTATE],
                sim.slots[2].b[B_SCRY],
                sim.slots[2].b[B_PHASE]
            ),
            (if flip & 32 == 0 { 1 } else { 2 }, 128, 21)
        );
        // Consume Mario's live projected X at a position just after the next move.
        let target = if flip & 32 == 0 { 141 } else { 147 };
        sim.mario.x = target - 96;
        sim.fff0 = 1;
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(
            (
                sim.player_screen_x,
                sim.slots[2].b[B_SCRX],
                sim.slots[2].b[B_FSTATE]
            ),
            (target as u8, target as u8, 4)
        );
        sim.fff0 = 3;
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(
            (sim.slots[2].b[B_SPRITE], sim.slots[2].b[B_FSTATE]),
            (61, 4)
        );
        for _ in 0..4 {
            sim.boss_engine(0, 0, &layout, &tables);
        }
        assert_eq!(
            (
                sim.slots[2].b[B_SPRITE],
                sim.slots[2].b[B_FSTATE],
                sim.slots[2].b[B_T24]
            ),
            (62, 5, 48)
        );
        for _ in 0..24 {
            sim.boss_engine(0, 0, &layout, &tables);
        }
        assert_eq!(
            (
                sim.slots[2].b[B_FSTATE],
                sim.slots[2].b[B_SCRY],
                sim.slots[2].b[B_T28],
                sim.sfx1
            ),
            (6, 80, 6, 13)
        );
        let mut idle = false;
        for _ in 0..40 {
            sim.boss_engine(0, 0, &layout, &tables);
            if sim.slots[2].b[B_FSTATE] == 0 {
                idle = true;
                break;
            }
        }
        assert!(idle);
        assert_eq!((sim.slots[2].b[B_SPRITE], sim.slots[2].b[B_TYPE]), (14, 0));
        // The command preserves the child's saturated phase for the next drop.
        sim.slots[0].b[B_DEF] = 16;
        sim.slots[0].b[B_T28] = 1;
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(
            (
                sim.slots[2].b[B_FSTATE],
                sim.slots[2].b[B_SPRITE],
                sim.slots[2].b[B_PHASE]
            ),
            (3, 60, 21)
        );
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0xa2be_removal_enters_shared_anim4_then_anim5_chain() {
    let tables = tables();
    let layout = layout();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(4);
    // Isolate the reward producer while retaining its actual slot position.
    sim.slots[0].b[B_DEF] = 0;
    sim.slots[1].b[B_DEF] = 0;
    sim.slots[2].b[B_FLIP] = 0x60;
    sim.remove_mode = 2;
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(
        (
            sim.slots[2].b[B_DEF],
            sim.slots[2].b[B_SCRX],
            sim.slots[2].b[B_SCRY]
        ),
        (4, 88, 128)
    );
    sim.boss_engine(0, 0, &layout, &tables);
    assert_eq!(
        (
            sim.slots[2].b[B_FSTATE],
            sim.slots[2].b[B_PHASE],
            sim.slots[2].b[B_SCRY]
        ),
        (1, 0, 128)
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
fn handler_0xa2be_missing_profile_reports_without_damage_or_sound() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.spawn_boss(4);
    sim.sfx1 = 0xAA;
    let mut tables = tables();
    tables.boss_picker_profile.clear();
    let layout = layout();
    let mut st = Staging::default();
    st.b[B_FSTATE] = 3;
    st.b[B_FLAGS] = 3;
    let expected = st.b;
    sim.dispatch_handler(0xA2BE, &mut st, &layout, &tables);
    assert_eq!(st.b, expected);
    assert_eq!((sim.boss_hits_remaining, sim.done, sim.sfx1), (6, 0, 0xAA));
    assert_eq!(sim.unported, vec![0xA2BE]);
}
