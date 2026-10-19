use super::*;
fn layout(tile: u8) -> LevelLayout {
    LevelLayout::from_bytes(&vec![tile; 0x3000]).unwrap()
}
fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}
fn staging() -> Staging {
    let mut b = [0xa5; 32];
    b[0] = 0;
    b[1] = 32;
    b[3] = 0;
    b[4] = 32;
    b[15] = 1;
    b[16] = 0;
    b[24] = 0;
    b[25] = 0;
    b[26] = 0;
    b[28] = 0;
    Staging {
        b,
        px: 0x1111,
        py: 0x2222,
    }
}
#[test]
fn handler_0x58848_dispatch_and_full_raw_boundary() {
    for (player, state, flip, facing) in [(31, 1, 0, 255), (32, 1, 0, 0), (33, 2, 32, 1)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.mario_ref_x = player;
        let mut st = staging();
        st.b[15] = 0;
        st.b[13] = 255;
        let mut expected = st.b;
        expected[15] = state;
        expected[13] = flip;
        expected[18] = 77;
        expected[28] = 1;
        sim.dispatch_handler(0x58848, &mut st, &layout(0x60), &tables());
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py, sim.facing), (0x1111, 0x2222, facing));
        assert!(sim.unported.is_empty());
    }
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging();
    st.b[15] = 2;
    st.b[28] = 1;
    let mut expected = st.b;
    expected[1] = 33;
    expected[4] = 29;
    expected[11] = 56;
    expected[16] = 1;
    expected[28] = 0;
    sim.handler_0x58848(&mut st, &layout(0x60));
    assert_eq!(st.b, expected);
    assert_eq!((st.px, st.py, sim.move_flags), (39, 27, 0x10));
}
#[test]
fn handler_0x58848_mixed_wall_thresholds_and_order() {
    // Y=24 makes the upper and lower probes occupy different metatiles.
    for right in [false, true] {
        for (upper, lower, blocked, final_y) in [
            (55, 96, true, 13),
            (56, 55, true, 19),
            (56, 56, false, 19),
            (96, 96, false, 19),
        ] {
            let mut sim = ObjectSim::new(vec![], 0);
            let mut st = staging();
            st.b[4] = 24;
            st.b[15] = if right { 2 } else { 1 };
            let mut map = layout(0x60);
            let x = if right { 39 } else { 25 };
            map.cells[(13 / 16) * 256 + x / 16] = upper;
            map.cells[(19 / 16) * 256 + x / 16] = lower;
            let mut expected = st.b;
            if blocked {
                expected[15] ^= 3;
                expected[13] ^= 32;
                expected[25] = 0;
            } else {
                expected[1] = if right { 33 } else { 31 };
                expected[4] = 21;
                expected[16] = 1;
            }
            sim.handler_0x58848(&mut st, &map);
            assert_eq!(
                st.b, expected,
                "right={right}, upper={upper}, lower={lower}"
            );
            assert_eq!((st.px, st.py), (x as u16, final_y));
            assert_eq!(
                sim.move_flags,
                match (right, blocked) {
                    (true, true) => 0x11,
                    (true, false) => 0x10,
                    (false, true) => 0x22,
                    (false, false) => 0x20,
                }
            );
        }
    }
}
#[test]
fn handler_0x58848_ground_threshold_alignment_and_full_boundary() {
    for (y, tile, grounded) in [
        (32, 71, true),
        (32, 72, false),
        (33, 71, true),
        (36, 71, false),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.mario_ref_x = 200;
        let mut st = staging();
        st.b[15] = 2;
        st.b[24] = 1;
        st.b[4] = y;
        st.b[16] = 2;
        let mut map = layout(0x60);
        map.cells[(y as usize / 16) * 256 + 2] = tile;
        let mut expected = st.b;
        expected[1] = 33;
        if grounded {
            expected[16] = 0;
            expected[24] = 0;
            expected[28] = 8;
            expected[25] = 1;
        } else {
            expected[16] = 3;
            expected[4] = y + 1;
        }
        sim.handler_0x58848(&mut st, &map);
        assert_eq!(st.b, expected, "y={y}, tile={tile}");
        assert_eq!(sim.move_flags, if grounded { 0x88 } else { 0x80 });
        // Misalignment returns before writing probe X.
        assert_eq!(
            (st.px, st.py),
            (if y & 12 == 0 { 33 } else { 39 }, y as u16)
        );
    }
}
#[test]
fn handler_0x58848_motion_source_samples_caps_and_diagnostics() {
    for (leg, phase, y, next, expected_leg) in [
        (0, 0, 29, 1, 0),
        (0, 11, 32, 0, 1),
        (0, 12, 32, 0, 1),
        (1, 11, 35, 12, 1),
        (1, 12, 34, 12, 1),
        (1, 13, 35, 14, 1),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staging();
        st.b[24] = leg;
        st.b[16] = phase;
        let mut expected = st.b;
        expected[1] = 31;
        expected[4] = y;
        expected[16] = next;
        expected[24] = expected_leg;
        if leg == 0 && phase >= 11 {
            expected[11] = 52;
        }
        sim.handler_0x58848(&mut st, &layout(0x60));
        assert_eq!(st.b, expected, "leg={leg} phase={phase}");
        assert!(sim.unported.is_empty());
    }
    for (state, phase) in [(1, 13), (1, 255), (5, 0)] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staging();
        st.b[15] = state;
        st.b[16] = phase;
        let mut expected = st.b;
        if state == 1 {
            expected[1] = 31;
        }
        sim.handler_0x58848(&mut st, &layout(0x60));
        assert_eq!(st.b, expected);
        assert_eq!(sim.unported, [0x58848]);
    }
}
#[test]
fn handler_0x58848_near_distance_wrap_and_equal_x_quirk() {
    // Object moves right before checking wrapped low-byte distance.
    for (x, player, state, count, flip, facing) in [
        (100, 148, 4, 1, 0xa5, 1),
        (100, 149, 2, 1, 0xa5, 0),
        (250, 42, 3, 2, 0x85, 255),
        (126, 127, 4, 1, 0xa5, 0),
        (127, 128, 3, 2, 0x85, 0),
        (100, 357, 4, 1, 0xa5, 1),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.mario_ref_x = player;
        let mut st = staging();
        set_x(&mut st.b, x);
        st.b[15] = 2;
        st.b[24] = 1;
        let mut map = layout(0x60);
        map.cells[2 * 256 + (x as usize + 1) / 16] = 0x47;
        let mut expected = st.b;
        set_x(&mut expected, x + 1);
        expected[24] = 0;
        expected[28] = 8;
        expected[15] = state;
        expected[25] = count;
        expected[13] = flip;
        sim.handler_0x58848(&mut st, &map);
        assert_eq!(st.b, expected, "x={x} player={player}");
        assert_eq!(sim.facing, facing);
    }
}
#[test]
fn handler_0x58848_real_step_anim31_dispatch_and_dead_freeze() {
    let tables = tables();
    let map = layout(0x60);
    assert_eq!(tables.profile_for(0, 31).unwrap().handler, 0x58848);
    for dead in [0, 255] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.phase = false;
        sim.fff0 = 3;
        sim.mario_dead = dead;
        sim.slots[1].b[B_STATE] = 2;
        sim.slots[1].b[B_DEF] = 31;
        set_x(&mut sim.slots[1].b, 100);
        set_y(&mut sim.slots[1].b, 100);
        let mario = MarioContact {
            x: 0,
            y: 68,
            travelling: true,
            ..Default::default()
        };
        sim.step(100, 100, &map, &tables, &mario, false);
        assert_eq!(
            (
                sim.slots[1].b[B_FSTATE],
                sim.slots[1].b[B_TYPE],
                sim.slots[1].b[B_T28]
            ),
            if dead == 0 { (1, 77, 1) } else { (0, 0, 0) }
        );
        sim.phase = false;
        sim.fff0 = 7;
        sim.step(100, 100, &map, &tables, &mario, false);
        assert_eq!(
            (sim.slots[1].x(), sim.slots[1].y(), sim.slots[1].b[B_PHASE]),
            if dead == 0 {
                (99, 97, 1)
            } else {
                (100, 100, 0)
            }
        );
        assert!(sim.unported.is_empty());
    }
}
#[test]
fn handler_0x58848_all_states_timer_and_cadence_full_boundary() {
    for state in 1..=4 {
        for timer in [0, 1, 2] {
            for frame in 0..4 {
                let mut sim = ObjectSim::new(vec![], 0);
                sim.fff0 = frame;
                let mut st = staging();
                st.b[15] = state;
                st.b[28] = timer;
                st.b[11] = 56;
                let mut expected = st.b;
                if timer > 0 {
                    expected[28] -= 1;
                }
                if state <= 2 && timer <= 1 {
                    expected[1] = if state == 1 { 31 } else { 33 };
                    expected[4] = 29;
                    expected[16] = 1;
                } else if state >= 3 && timer == 0 && frame == 0 {
                    expected[1] = if state == 3 { 28 } else { 36 };
                    expected[11] = 57;
                    expected[26] = 1;
                }
                sim.handler_0x58848(&mut st, &layout(0x60));
                assert_eq!(st.b, expected, "state={state} timer={timer} frame={frame}");
            }
        }
    }
}
#[test]
fn handler_0x58848_misaligned_ground_preserves_previous_probe_high_byte() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging();
    st.b[15] = 2;
    st.b[24] = 1;
    st.b[16] = 2;
    set_y(&mut st.b, 0x104);
    let mut expected = st.b;
    expected[1] = 33;
    expected[4] = 5;
    expected[16] = 3;
    sim.handler_0x58848(&mut st, &layout(0x60));
    assert_eq!(st.b, expected);
    // Last wall probe was Y=$00FF. Ground writes low $04, then exits.
    assert_eq!((st.px, st.py, sim.move_flags), (39, 4, 0x80));
}
