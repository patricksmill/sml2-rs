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
fn handler_0xa27e_dispatch_sound_no_damage_and_diagnostic() {
    let tables = tables();
    let layout = layout();
    assert_eq!(tables.boss_anim_handler(17), Some(0xA27E));
    for state in [0, 1, 2, 3, 255] {
        for sprite in [14, 59] {
            let mut sim = ObjectSim::new(vec![], 0);
            sim.spawn_boss(4);
            sim.sfx2 = 0xA7;
            sim.fff0 = 1;
            sim.remove_mode = 2;
            sim.boss_hits_remaining = 1;
            sim.boss_hit_latch = 0x67;
            sim.done = 0x89;
            let mut st = Staging::default();
            st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(29));
            st.b[B_FSTATE] = state;
            st.b[B_SPRITE] = sprite;
            st.b[B_FLAGS] = 3;
            st.px = 0x1234;
            st.py = 0x5678;
            let mut expected = st.b;
            if state == 0 && sprite != 14 {
                expected[B_TYPE] = 64;
                expected[B_FSTATE] = if expected[B_FLIP] & 32 == 0 { 1 } else { 2 };
            }
            sim.dispatch_handler(0xA27E, &mut st, &layout, &tables);
            assert_eq!(st.b, expected);
            assert_eq!((st.px, st.py), (0x1234, 0x5678));
            assert_eq!(sim.sfx2, if state == 0 && sprite != 14 { 9 } else { 0xA7 });
            assert_eq!(
                (
                    sim.boss_hits_remaining,
                    sim.remove_mode,
                    sim.boss_clear_flag
                ),
                (1, 2, 0)
            );
            assert_eq!((sim.boss_hit_latch, sim.done), (0x67, 0x89));
            assert_eq!(sim.unported.contains(&0xA27E), state > 2);
        }
    }
}

#[test]
fn handler_0xa27e_parent_command_is_consumed_in_same_driver_pass_and_reused() {
    let tables = tables();
    let layout = layout();
    for (state, flip, x, heading, end_x, reset_x) in
        [(3, 0xC7, 16, 1, 1, 254), (6, 0xE7, 160, 2, 183, 186)]
    {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.spawn_boss(4);
        // Only main and anim-17 child; anim 18 remains a separate ticket.
        sim.boss_def_count = 1;
        sim.door_phase = 2;
        sim.slots[0].b[B_FSTATE] = state;
        sim.slots[0].b[B_T28] = 1;
        sim.slots[0].b[B_SCRX] = x;
        sim.slots[0].b[B_FLIP] = flip;
        sim.slots[1].b[B_TYPE] = 0xA5;
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(
            (
                sim.slots[1].b[B_FSTATE],
                sim.slots[1].b[B_SPRITE],
                sim.slots[1].b[B_TYPE]
            ),
            (heading, 59, 64)
        );
        assert_eq!(
            (
                sim.slots[1].b[B_SCRX],
                sim.slots[1].b[B_SCRY],
                sim.slots[1].b[B_FLIP]
            ),
            (if state == 3 { 8 } else { 168 }, 51, flip)
        );
        assert_eq!((sim.sfx2, sim.door_phase), (9, 1));
        // Next pass is an even movement tick; the following odd tick waits.
        sim.boss_engine(0, 0, &layout, &tables);
        let moved_x = if heading == 1 { 5 } else { 171 };
        assert_eq!(sim.slots[1].b[B_SCRX], moved_x);
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(sim.slots[1].b[B_SCRX], moved_x);
        // Offscreen tests use the new wrapped X. Recycling preserves contact.
        sim.slots[1].b[B_SCRX] = end_x;
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(
            (
                sim.slots[1].b[B_FSTATE],
                sim.slots[1].b[B_SPRITE],
                sim.slots[1].b[B_SCRX]
            ),
            (0, 14, reset_x)
        );
        assert_eq!(sim.slots[1].b[B_TYPE], 64);
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(sim.slots[1].b[B_SPRITE], 14);
        // An idle child can consume another volley on that same frame.
        sim.slots[0].b[B_T28] = 1;
        sim.boss_engine(0, 0, &layout, &tables);
        assert_eq!(
            (sim.slots[1].b[B_FSTATE], sim.slots[1].b[B_SPRITE]),
            (heading, 59)
        );
        assert_eq!(sim.door_phase, 0);
        assert!(sim.unported.is_empty());
    }
}
