use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}
fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    sim.move_flags = 0x97;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}
fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}
fn check(sim: &mut ObjectSim, st: &mut Staging, t: &ObjectTables, expected: [u8; 32], facing: u8) {
    sim.handler_0x5902d(st, t);
    assert_eq!(st.b, expected);
    assert_eq!(
        (st.px, st.py, sim.move_flags, sim.facing),
        (0x1234, 0x5678, 0x97, facing)
    );
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5902d_initial_offsets_all_directions_and_circular_allocation() {
    let t = tables();
    // Independent signed ROM bytes $59082..$59091.
    let ys: [i16; 8] = [4, -20, -32, 40, 28, 16, -44, -8];
    let xs: [i16; 8] = [0, -16, 24, -24, 32, -40, -16, 40];
    for direction in 0..8u8 {
        for (x, y) in [(0u16, 0u16), (65535, 65535), (0x12FF, 0x3400)] {
            for only_free in [false, true] {
                let (mut sim, mut st) = fixture(0);
                sim.div = if only_free { 0xFF } else { 0xA0 | direction };
                position(&mut st.b, x, y);
                st.b[31] = if only_free { !(1 << direction) } else { 0 };
                let mut expected = st.b;
                position(
                    &mut expected,
                    x.wrapping_add_signed(xs[direction as usize]),
                    y.wrapping_add_signed(ys[direction as usize]),
                );
                expected[13] = 0;
                expected[14] = 16;
                expected[15] = 3;
                expected[18] = 0;
                expected[28] = 48;
                expected[31] |= 1 << direction;
                check(&mut sim, &mut st, &t, expected, 99);
            }
        }
    }
}

#[test]
fn handler_0x5902d_tracking_all_profile_entries_states_and_frame_cadence() {
    let t = tables();
    // Literal ROM $59125 profile, indexed after acceleration. Motion magnitude
    // reduces horizontal movement; total component magnitude is three.
    let speed = [-3i16, -2, -2, -1, -2, -1, -1, 0, 0, 0, 1, 1, 2, 1, 2, 2, 3];
    for state in [1, 2] {
        for frame in [0, 1, 2, 3, 4, 255] {
            for (leg, &dy) in speed.iter().enumerate() {
                for above in [false, true] {
                    let (mut sim, mut st) = fixture(state);
                    sim.fff0 = frame;
                    let (x, y) = if above {
                        (0u16, 0x0100u16)
                    } else {
                        (65535, 0x00FF)
                    };
                    position(&mut st.b, x, y);
                    st.b[24] = if above {
                        leg as u8 + 1
                    } else {
                        (leg as u8).wrapping_sub(1)
                    };
                    sim.mario_ref_y = if above { 0x00FF } else { 0x0100 };
                    sim.mario_ref_x = 0x1234;
                    let mut expected = st.b;
                    expected[11] ^= 1; // $5B518 is unconditional, including odd frames.
                    let mut facing = 99;
                    if frame & 3 == 0 {
                        let dx = 3 - dy.unsigned_abs();
                        let nx = if state == 1 {
                            x.wrapping_sub(dx)
                        } else {
                            x.wrapping_add(dx)
                        };
                        position(&mut expected, nx, y.wrapping_add_signed(dy));
                        expected[24] = leg as u8;
                        let right = sim.mario_ref_x > nx;
                        expected[15] = if right { 2 } else { 1 };
                        expected[13] = if right { 32 } else { 0 };
                        facing = if right {
                            1
                        } else if sim.mario_ref_x == nx {
                            0
                        } else {
                            255
                        };
                    }
                    check(&mut sim, &mut st, &t, expected, facing);
                }
            }
        }
    }
}

#[test]
fn handler_0x5902d_tracking_facing_after_move_ties_and_vertical_wrap() {
    let t = tables();
    for (state, x, y, target_x, target_y, leg, nx, ny, next_leg, next_state, flip, facing) in [
        (1, 0u16, 0u16, 65533, 0, 255, 0, 65533, 0, 2, 32, 1),
        (2, 65535, 65535, 65535, 65535, 16, 65535, 2, 16, 1, 0, 0),
        (1, 256, 200, 253, 201, 7, 253, 200, 8, 1, 0, 0),
        (2, 255, 200, 257, 199, 7, 257, 199, 6, 1, 0, 0),
    ] {
        let (mut sim, mut st) = fixture(state);
        position(&mut st.b, x, y);
        st.b[24] = leg;
        sim.mario_ref_x = target_x;
        sim.mario_ref_y = target_y;
        let mut expected = st.b;
        position(&mut expected, nx, ny);
        expected[11] ^= 1;
        expected[24] = next_leg;
        expected[15] = next_state;
        expected[13] = flip;
        check(&mut sim, &mut st, &t, expected, facing);
    }
}

#[test]
fn handler_0x5902d_wait_timers_wrap_and_transition_without_motion() {
    let t = tables();
    for state in [3, 4] {
        for frame in [0, 1, 3, 4, 7, 8, 255] {
            for timer in [0u8, 1, 2, 24, 48, 255] {
                for (target_x, next_state, flip, facing) in
                    [(0x1233, 1, 0, 255), (0x1234, 1, 0, 0), (0x1235, 2, 32, 1)]
                {
                    let (mut sim, mut st) = fixture(state);
                    position(&mut st.b, 0x1234, 0x5678);
                    st.b[28] = timer;
                    sim.fff0 = frame;
                    sim.mario_ref_x = target_x;
                    let mut expected = st.b;
                    if frame % if state == 3 { 8 } else { 4 } == 0 {
                        expected[11] ^= 1;
                    }
                    expected[28] = timer.wrapping_sub(1);
                    let mut expected_facing = 99;
                    if timer == 1 {
                        if state == 3 {
                            expected[15] = 4;
                            expected[18] = 82;
                            expected[11] = 110;
                            expected[28] = 24;
                        } else {
                            expected[15] = next_state;
                            expected[13] = flip;
                            expected[24] = 8;
                            expected_facing = facing;
                        }
                    }
                    check(&mut sim, &mut st, &t, expected, expected_facing);
                }
            }
        }
    }
}

#[test]
fn handler_0x5902d_full_mask_reuses_removal_mode_two_without_rewards_or_rearm() {
    let t = tables();
    for activity in [1, 2, 255] {
        let (mut sim, mut st) = fixture(0);
        sim.spawned = vec![true];
        sim.standby = 0;
        sim.active = 0;
        sim.money_bag = 0x99;
        sim.spawn_anim = 77;
        st.b[8] = activity;
        st.b[31] = 255;
        sim.handler_0x5902d(&mut st, &t);
        let mut expected = [0; 32];
        // Sprite zero's engine-derived collision extents after the clear.
        let hb = t.hitbox_for(0, 0).unwrap();
        expected[19..23].copy_from_slice(&hb.map(|v| v as u8));
        assert_eq!(st.b, expected);
        assert_eq!(
            (sim.standby, sim.active),
            if activity == 1 { (255, 0) } else { (0, 255) }
        );
        assert_eq!(
            (sim.done, sim.remove_mode, sim.money_bag, sim.spawn_anim),
            (2, 0, 0x99, 77)
        );
        assert_eq!(sim.spawned, vec![true]);
        assert_eq!(
            (st.px, st.py, sim.facing, sim.move_flags),
            (0x1234, 0x5678, 99, 0x97)
        );
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x5902d_invalid_states_and_advanced_profiles_are_diagnostic() {
    let t = tables();
    for state in 5..=255 {
        let (mut sim, mut st) = fixture(state);
        let original = st.b;
        sim.handler_0x5902d(&mut st, &t);
        assert_eq!(st.b, original);
        assert_eq!(sim.unported, vec![0x5902D]);
    }
    for (leg, player_y, advanced, facing) in [(255, 99, 254, 255), (17, 101, 18, 1)] {
        let (mut sim, mut st) = fixture(1);
        position(&mut st.b, 100, 100);
        st.b[24] = leg;
        sim.mario_ref_y = player_y;
        let mut expected = st.b;
        expected[11] ^= 1;
        expected[24] = advanced;
        sim.handler_0x5902d(&mut st, &t);
        assert_eq!(st.b, expected);
        assert_eq!(sim.facing, facing);
        assert_eq!(sim.unported, vec![0x5902D]);
    }
}

#[test]
fn handler_0x5902d_real_anim53_step_dispatch_death_and_parity_gates() {
    let t = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(t.profile_for(0, 53).unwrap().handler, 0x5902D);
    for dead in [0, 255] {
        for travelling in [false, true] {
            for initial_state in [0, 3] {
                for phase in [false, true] {
                    let mut sim = ObjectSim::new(vec![], 0);
                    sim.fff0 = 3;
                    sim.phase = phase;
                    sim.mario_dead = dead;
                    let mut actor = Actor::default();
                    actor.position = Position { x: 100, y: 100 };
                    actor.activity = 2;
                    actor.definition = 53;
                    actor.state = initial_state;
                    actor.timer = 5;
                    sim.slots[1].b = actor.to_raw_slot();
                    let mario = MarioContact {
                        x: 1000,
                        travelling,
                        ..Default::default()
                    };
                    sim.step(100, 100, &layout, &t, &mario, false);
                    let runs = dead == 0 && (initial_state == 0 || !phase);
                    let result = &sim.slots[1].b;
                    assert_eq!(result[15], if runs { 3 } else { initial_state });
                    assert_eq!(
                        result[28],
                        if runs {
                            if initial_state == 0 {
                                48
                            } else {
                                4
                            }
                        } else {
                            5
                        }
                    );
                    assert_eq!(result[31], if runs && initial_state == 0 { 1 } else { 0 });
                    assert!(sim.unported.is_empty());
                }
            }
        }
    }
}
