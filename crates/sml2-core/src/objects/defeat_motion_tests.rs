//! Legacy death dispatch consumes the native shared defeat motion contract.
use super::*;

#[test]
fn defeat_motion_real_death_dispatch_init_rise_and_fall_handoff_exact_bytes() {
    let layout = LevelLayout::from_bytes(&[0x60; crate::level::LAYOUT_SIZE]).unwrap();
    let tables = tables();
    for initial_state in [0, 3, 255] {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        let mut expected: [u8; 32] =
            std::array::from_fn(|i| (i as u8).wrapping_mul(17).wrapping_add(3));
        expected[3..5].copy_from_slice(&2u16.to_be_bytes());
        expected[B_FSTATE] = initial_state;
        let mut st = Staging {
            b: expected,
            ..Staging::default()
        };
        sim.dispatch_handler(0x581B1, &mut st, &layout, &tables);
        let mut y = 2u16.wrapping_sub(20);
        expected[3..5].copy_from_slice(&y.to_be_bytes());
        expected[B_FLIP] ^= 0x40;
        expected[B_FSTATE] = 1;
        expected[B_PHASE] = 0;
        expected[B_TYPE] = 0;
        assert_eq!(st.b, expected);
        // Eight calls from phase zero, with the transition on the last call.
        for (phase, distance) in [4, 2, 4, 2, 2, 2, 0, 0].into_iter().enumerate() {
            sim.dispatch_handler(0x581B1, &mut st, &layout, &tables);
            y = y.wrapping_sub(distance);
            expected[3..5].copy_from_slice(&y.to_be_bytes());
            expected[B_FSTATE] = if phase == 7 { 2 } else { 1 };
            expected[B_PHASE] = if phase == 7 { 0 } else { phase as u8 + 1 };
            assert_eq!(st.b, expected, "rise phase={phase}");
        }
        // Existing fall arm retains its doubled fall profile and does not hop again.
        expected[B_PHASE] = 8;
        expected[B_SCRY] = 0x9F;
        st.b = expected;
        sim.dispatch_handler(0x581B1, &mut st, &layout, &tables);
        y = y.wrapping_add(4); // ROM $5B490 = 2, doubled by $581E2.
        expected[3..5].copy_from_slice(&y.to_be_bytes());
        expected[B_PHASE] = 9;
        assert_eq!(st.b, expected);
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn defeat_motion_real_death_dispatch_terminal_and_invalid_phase_diagnostics() {
    let layout = LevelLayout::from_bytes(&[0x60; crate::level::LAYOUT_SIZE]).unwrap();
    let tables = tables();
    let mut sim = ObjectSim::new(Vec::new(), 0);
    for phase in [8, 9, 255] {
        let mut expected: [u8; 32] =
            std::array::from_fn(|i| (i as u8).wrapping_mul(11).wrapping_add(7));
        expected[B_FSTATE] = 1;
        expected[B_PHASE] = phase;
        let mut st = Staging {
            b: expected,
            ..Staging::default()
        };
        sim.dispatch_handler(0x581B1, &mut st, &layout, &tables);
        if phase == 8 {
            expected[B_FSTATE] = 2;
            expected[B_PHASE] = 0;
            assert!(sim.unported.is_empty());
        } else {
            assert_eq!(sim.unported, [0x581CE]);
        }
        assert_eq!(st.b, expected);
    }
}

fn tables() -> ObjectTables {
    ObjectTables {
        schema: String::new(),
        notes: Vec::new(),
        anim_to_sprite_a: vec![0; 96],
        anim_to_sprite_b: Vec::new(),
        hitbox_records_a: vec![[0; 4]],
        hitbox_records_b: Vec::new(),
        hitbox_index_a: vec![0; 96],
        hitbox_index_b: Vec::new(),
        boss_anim_handlers: Vec::new(),
        boss_picker_profile: Vec::new(),
        boss_picker_profile_15: Vec::new(),
        boss_sweep_profile: Vec::new(),
        draw_records: Vec::new(),
        draw_index: Vec::new(),
        sprite_mappings_a: Vec::new(),
        sprite_mappings_b: Vec::new(),
        anim_profiles_a: vec![
            crate::object::AnimProfile {
                handler: 0,
                type_sets: Vec::new(),
                states: Vec::new()
            };
            96
        ],
        anim_profiles_b: Vec::new(),
        player_mappings: Vec::new(),
        // 118 empty lists: the boss render pass must find a list for any
        // sprite id a def installs, so the synthetic table keeps the shape
        // of the real one (see `boss_oam_tables` for a populated one).
        boss_oam_lists: vec![vec![0x80]; 118],
    }
}
