use super::*;
fn raw(state: u8) -> [u8; 32] {
    let mut b = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    b[9] = 100;
    b[10] = 100;
    b[11] = 90;
    b[12] = 128;
    b[15] = state;
    b[16] = 0;
    b[18] = 69;
    b[24] = if matches!(state, 16 | 27) { 3 } else { 1 };
    b[25] = 2;
    b[26] = 3;
    b[28] = 0;
    b
}
#[test]
fn handler_0xa372_every_movement_state_rom_byte_fixture() {
    // Expected writes independently transcribed from the 29 state-table entries.
    let cases: &[(u8, &[(usize, u8)])] = &[
        (3, &[(10, 98), (25, 0), (12, 0), (15, 4)]),
        (4, &[(10, 98), (16, 1)]),
        (
            5,
            &[(10, 98), (25, 0), (15, 6), (11, 69), (26, 0), (18, 68)],
        ),
        (6, &[(9, 97), (16, 1)]),
        (
            7,
            &[
                (9, 99),
                (24, 0),
                (15, 8),
                (11, 66),
                (26, 0),
                (25, 34),
                (10, 102),
            ],
        ),
        (8, &[(10, 98), (25, 36), (15, 9), (28, 64), (13, 32)]),
        (9, &[(10, 102), (25, 0), (12, 0), (15, 10)]),
        (10, &[(10, 102), (16, 1)]),
        (
            11,
            &[(10, 102), (25, 0), (15, 12), (11, 69), (26, 0), (18, 68)],
        ),
        (12, &[(9, 97), (16, 1)]),
        (
            13,
            &[(9, 99), (24, 0), (15, 14), (11, 66), (26, 0), (13, 64)],
        ),
        (14, &[(10, 97), (16, 1)]),
        (
            15,
            &[(10, 98), (25, 0), (15, 16), (24, 96), (11, 69), (26, 0)],
        ),
        (
            16,
            &[
                (9, 103),
                (24, 0),
                (15, 17),
                (25, 28),
                (13, 32),
                (11, 66),
                (26, 0),
            ],
        ),
        (17, &[(10, 102), (25, 0), (15, 18)]),
        (18, &[(9, 97), (16, 1)]),
        (19, &[(10, 102), (25, 36), (15, 20), (13, 0), (28, 64)]),
        (20, &[(10, 98), (25, 0), (12, 0), (15, 21)]),
        (21, &[(10, 99), (16, 1)]),
        (
            22,
            &[(10, 98), (25, 0), (15, 23), (11, 69), (26, 0), (18, 68)],
        ),
        (23, &[(9, 97), (16, 1)]),
        (
            24,
            &[(9, 99), (24, 0), (15, 25), (11, 66), (26, 0), (13, 96)],
        ),
        (25, &[(10, 103), (16, 1)]),
        (
            26,
            &[
                (10, 102),
                (25, 0),
                (15, 27),
                (11, 69),
                (26, 0),
                (13, 64),
                (24, 96),
            ],
        ),
        (
            27,
            &[
                (9, 103),
                (24, 0),
                (15, 28),
                (25, 56),
                (11, 66),
                (26, 0),
                (13, 32),
            ],
        ),
        (
            28,
            &[
                (10, 102),
                (25, 0),
                (15, 29),
                (11, 69),
                (26, 0),
                (18, 68),
                (13, 32),
                (24, 96),
            ],
        ),
        (29, &[(9, 97), (16, 1)]),
        (
            30,
            &[
                (9, 99),
                (24, 0),
                (15, 31),
                (11, 66),
                (26, 0),
                (13, 32),
                (25, 36),
                (10, 96),
            ],
        ),
        (31, &[(10, 102), (25, 36), (15, 3), (13, 0), (28, 64)]),
    ];
    for &(state, writes) in cases {
        let mut expected = raw(state);
        let mut actor = Actor::from_raw_slot(&expected);
        assert_eq!(update(&mut actor), Ok(()));
        for &(offset, value) in writes {
            expected[offset] = value;
        }
        assert_eq!(actor.to_raw_slot(), expected, "state {state}");
    }
}

#[test]
fn handler_0xa372_path_samples_and_sentinel_endpoints() {
    // ROM tables, paired as vertical/horizontal steps, including sentinel phase.
    let drop_y = &[
        0u8, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    ];
    let drop_x = &[
        2u8, 2, 2, 2, 2, 2, 1, 2, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0,
    ];
    let arc = &[3u8, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0];
    let slide = &[0u8, 0, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1];
    let fall_y = &[0u8, 1, 1, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4];
    let fall_x = &[2u8, 2, 2, 2, 2, 1, 2, 1, 1, 1, 1, 1, 1, 1, 0, 0];
    let short_y = &[3u8, 2, 3, 2, 2, 2, 1, 0, 1, 0, 0];
    let short_x = &[0u8, 1, 1, 1, 2, 2, 2, 2, 2, 2, 1];
    let land_y = &[0u8, 0, 1, 0, 1, 2, 2, 2, 3, 2, 3];
    let land_x = &[1u8, 2, 2, 2, 2, 2, 2, 1, 1, 1, 0];
    let cases: &[(u8, &[u8], &[u8], bool, bool, usize, u8)] = &[
        (4, drop_y, drop_x, false, true, 25, 76),
        (6, arc, slide, false, false, 24, 28),
        (10, fall_y, fall_x, true, true, 25, 80),
        (12, arc, slide, true, false, 24, 68),
        (14, slide, arc, false, false, 25, 28),
        (18, short_y, short_x, true, false, 25, 36),
        (21, land_y, land_x, false, true, 25, 84),
        (23, arc, slide, false, false, 24, 68),
        (25, slide, arc, true, false, 25, 28),
        (29, arc, slide, true, false, 24, 60),
    ];
    for &(state, ys, xs, right, down, offset, value) in cases {
        for phase in 0..=ys.len() {
            let mut expected = raw(state);
            expected[16] = phase as u8;
            expected[9] = 0;
            expected[10] = 255;
            expected[12] = 0;
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(update(&mut actor), Ok(()));
            if phase == ys.len() {
                expected[16] = 0;
                expected[15] += 1;
                expected[offset] = value;
                if state == 18 {
                    expected[12] = 128;
                }
            } else {
                expected[9] = if down {
                    ys[phase]
                } else {
                    0u8.wrapping_sub(ys[phase])
                };
                expected[10] = if right {
                    255u8.wrapping_add(xs[phase])
                } else {
                    255u8.wrapping_sub(xs[phase])
                };
                expected[16] += 1;
            }
            assert_eq!(
                actor.to_raw_slot(),
                expected,
                "state {state}, phase {phase}"
            );
        }
    }
}

#[test]
fn handler_0xa372_countdown_returns_on_one_and_motion_counters_wrap() {
    for state in [3, 9, 20] {
        for timer in 1..=255 {
            let mut expected = raw(state);
            expected[28] = timer;
            let mut actor = Actor::from_raw_slot(&expected);
            update(&mut actor).unwrap();
            expected[28] -= 1;
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
    for state in [3, 5, 8, 9, 11, 15, 17, 19, 20, 22, 26, 28, 31] {
        for span in 0u8..=255 {
            if span == 2 {
                continue;
            } // exact terminal writes are pinned above.
            let mut expected = raw(state);
            expected[25] = span;
            expected[10] = 0;
            let mut actor = Actor::from_raw_slot(&expected);
            update(&mut actor).unwrap();
            expected[25] = span.wrapping_sub(2);
            expected[10] = if matches!(state, 3 | 5 | 8 | 15 | 20 | 22) {
                254
            } else {
                2
            };
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
    for state in [7, 13, 16, 24, 27, 30] {
        let step = if matches!(state, 16 | 27) { 3u8 } else { 1 };
        for leg in 0u8..=255 {
            if leg == step {
                continue;
            }
            let mut expected = raw(state);
            expected[24] = leg;
            expected[9] = 255;
            let mut actor = Actor::from_raw_slot(&expected);
            update(&mut actor).unwrap();
            expected[24] = leg.wrapping_sub(step);
            expected[9] = if step == 3 { 2 } else { 254 };
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn handler_0xa372_bad_motion_phase_is_diagnostic_without_synthesized_steps() {
    for state in [4, 6, 10, 12, 14, 18, 21, 23, 25, 29] {
        let mut expected = raw(state);
        expected[16] = 255;
        let mut actor = Actor::from_raw_slot(&expected);
        assert_eq!(update(&mut actor), Err(()));
        assert_eq!(actor.to_raw_slot(), expected);
    }
}
