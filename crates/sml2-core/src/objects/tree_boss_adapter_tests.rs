use crate::level::{LevelLayout, LAYOUT_SIZE};
use crate::objects::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn handler_0x93a7_dispatch_damage_saves_hit_and_preserves_staging_coordinates() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&vec![0; LAYOUT_SIZE]).unwrap();
    for (flags, cost, stomp) in [(1, 2u8, true), (2, 1, false), (3, 2, true)] {
        for primary in [false, true] {
            for hits in [0u8, 1, 2, 3, 6] {
                let mut sim = ObjectSim::new(vec![], 0);
                sim.boss_slot_ptr_lo = if primary { 0 } else { 0x20 };
                sim.boss_hits_remaining = hits;
                sim.boss_timer_reload = 80;
                sim.boss_alt_phase = 0x77;
                let mut st = Staging {
                    b: [0xA5; 32],
                    px: 0x1234,
                    py: 0x5678,
                };
                st.b[B_FLAGS] = flags;
                st.b[B_FSTATE] = 6;
                let mut expected = st.b;
                expected[B_TYPE] = 0;
                expected[B_FSTATE] = 1;
                if stomp {
                    expected[B_SPRITE] = 0;
                    expected[B_T28] = 80;
                }
                sim.dispatch_handler(0x93A7, &mut st, &layout, &tables);
                assert_eq!(st.b, expected);
                assert_eq!((st.px, st.py), (0x1234, 0x5678));
                assert_eq!(
                    (sim.spawn_sprite, sim.saved_frame_state, sim.saved_type),
                    (0xA5, 6, 0xA5)
                );
                assert_eq!(
                    sim.boss_hits_remaining,
                    if primary {
                        hits.wrapping_sub(cost)
                    } else {
                        hits
                    }
                );
                let defeated = primary && (hits == cost || (stomp && hits < cost));
                assert_eq!(sim.fade_timer_set, defeated.then_some(0x0280));
                assert_eq!(sim.boss_clear_flag, u8::from(defeated));
                assert_eq!(sim.remove_mode, u8::from(defeated));
                assert_eq!(sim.boss_alt_phase, 0x77);
                assert_eq!(sim.sfx0, if primary && stomp { 19 } else { 0 });
                assert_eq!(sim.sfx1, if primary && !stomp { 9 } else { 0 });
                assert!(sim.unported.is_empty());
            }
        }
    }
}

#[test]
fn handler_0x93a7_adapter_sweep_effects_and_persistent_alternate_phase() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.boss_hits_remaining = 2;
    sim.sfx1 = 0xA1;
    sim.sfx2 = 0xA2;
    let mut st = Staging::default();
    st.b[B_FSTATE] = 4;
    st.b[B_PHASE] = 104;
    sim.handler_tree_boss(&mut st, &tables);
    assert_eq!((st.b[B_FSTATE], st.b[B_PHASE], st.b[B_T28]), (5, 92, 80));
    assert_eq!(sim.boss_alt_phase, 1, "$AFC6");
    assert_eq!((sim.sfx1, sim.sfx2), (0xA1, 0xA2));
    // Delay expiration applies sound 8, then the normal return blink/motion.
    sim.fff0 = 0;
    st.b[B_T28] = 1;
    st.b[B_PHASE] = 1;
    st.b[B_SCRY] = 255;
    sim.handler_tree_boss(&mut st, &tables);
    assert_eq!(
        st.b[B_SCRY], 0,
        "latched alternate leg moves below phase 12"
    );
    assert_eq!((st.b[B_FSTATE], st.b[B_PHASE], st.b[B_SPRITE]), (6, 1, 2));
    assert_eq!((sim.sfx1, sim.sfx2, sim.boss_alt_phase), (8, 2, 1));
    sim.boss_hits_remaining = 6;
    st.b[B_PHASE] = 104;
    sim.handler_tree_boss(&mut st, &tables);
    assert_eq!(
        (st.b[B_FSTATE], st.b[B_PHASE], sim.boss_alt_phase),
        (7, 104, 1)
    );
}

#[test]
fn handler_0x93a7_adapter_morph_mutes_only_primary_and_reports_invalid_state() {
    let tables = tables();
    for primary in [false, true] {
        for timer in [0u8, 1, 2] {
            let mut sim = ObjectSim::new(vec![], 0);
            sim.boss_slot_ptr_lo = if primary { 0 } else { 0x20 };
            sim.music_request = Some(0x77);
            let mut st = Staging {
                b: [0xA5; 32],
                ..Default::default()
            };
            st.b[B_FLAGS] = 0;
            st.b[B_FSTATE] = 2;
            st.b[B_T28] = timer;
            let mut expected = st.b;
            expected[B_FLASH] ^= 0x10;
            expected[B_T28] = timer.wrapping_sub(1);
            if timer == 1 {
                expected[B_DEF] = 3;
                expected[B_FSTATE] = 0;
                expected[B_ANIMFRAME] = 0;
                expected[B_FLASH] = 0;
            }
            sim.handler_tree_boss(&mut st, &tables);
            assert_eq!(st.b, expected);
            assert_eq!(
                sim.music_request,
                Some(if primary && timer == 1 { 255 } else { 0x77 })
            );
        }
    }
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[B_FSTATE] = 8;
    sim.handler_tree_boss(&mut st, &tables);
    assert!(sim.unported.contains(&0x93A7));
}
