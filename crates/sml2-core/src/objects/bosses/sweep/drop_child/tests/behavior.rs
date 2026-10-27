use super::*;
// ROM `$B1A3..$B1B8`, selected by `$B18F` with cap 21.
const DROP: [u8; 22] = [
    0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3,
];
fn input() -> Inputs<'static> {
    Inputs {
        frame: FrameTick::new(0),
        player_x: 88,
        remove_mode: 0,
        drop_profile: &DROP,
    }
}
fn dirty(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    raw[15] = state;
    raw
}

#[test]
fn handler_0xa2be_init_and_unconditional_reward_morph_preserve_whole_slot() {
    for state in [0, 1, 2, 3, 4, 5, 6, 255] {
        for sprite in [0, 14, 60, 255] {
            for mode in [0, 1, 2, 3, 255] {
                if mode != 2 && state != 0 {
                    continue;
                }
                let mut expected = dirty(state);
                expected[11] = sprite;
                let mut actor = Actor::from_raw_slot(&expected);
                assert_eq!(
                    update(
                        &mut actor,
                        Inputs {
                            remove_mode: mode,
                            ..input()
                        }
                    ),
                    Ok(None)
                );
                if mode == 2 {
                    expected[5] = 4;
                    expected[9] = 128;
                    expected[10] = 88;
                    expected[11] = 9;
                    expected[15] = 0;
                    expected[16] = 0;
                    expected[18] = 0;
                } else if sprite != 14 {
                    expected[15] = 3;
                    expected[18] = 64;
                }
                assert_eq!(actor.to_raw_slot(), expected);
            }
        }
    }
}

#[test]
fn handler_0xa2be_drop_profile_cap_wrapping_threshold_and_flip_bit() {
    for phase in 0u8..=21 {
        for y in [0u8, 125, 126, 127, 128, 254, 255] {
            for flip in [0, 0x20, 0x40, 0xDF, 0xFF] {
                for frame in [0, 1] {
                    let mut expected = dirty(3);
                    expected[16] = phase;
                    expected[9] = y;
                    expected[13] = flip;
                    let mut actor = Actor::from_raw_slot(&expected);
                    assert_eq!(
                        update(
                            &mut actor,
                            Inputs {
                                frame: FrameTick::new(frame),
                                ..input()
                            }
                        ),
                        Ok(None)
                    );
                    expected[9] = y.wrapping_add(DROP[phase as usize]);
                    expected[16] = if phase == 21 { 21 } else { phase + 1 };
                    if expected[9] >= 128 {
                        expected[15] = if flip & 32 == 0 { 1 } else { 2 };
                    }
                    assert_eq!(actor.to_raw_slot(), expected);
                }
            }
        }
    }
}

#[test]
fn handler_0xa2be_approach_uses_wrapped_distance_after_move_and_even_cadence() {
    for state in [1u8, 2] {
        for x in 0u8..=255 {
            for player_x in [0u8, 15, 16, 88, 127, 128, 240, 255] {
                for frame in 0..4 {
                    let mut expected = dirty(state);
                    expected[10] = x;
                    let mut actor = Actor::from_raw_slot(&expected);
                    assert_eq!(
                        update(
                            &mut actor,
                            Inputs {
                                player_x,
                                frame: FrameTick::new(frame),
                                ..input()
                            }
                        ),
                        Ok(None)
                    );
                    if frame % 2 == 0 {
                        expected[10] = if state == 1 {
                            x.wrapping_sub(3)
                        } else {
                            x.wrapping_add(3)
                        };
                        let delta = player_x.wrapping_sub(expected[10]);
                        let magnitude = if delta & 128 == 0 {
                            delta
                        } else {
                            0u8.wrapping_sub(delta)
                        };
                        if magnitude < 16 {
                            expected[15] = 4;
                        }
                    }
                    assert_eq!(actor.to_raw_slot(), expected);
                }
            }
        }
    }
}

#[test]
fn handler_0xa2be_sprite_increment_checks_new62_and_terminal_checks_old65() {
    for state in [4, 6] {
        for sprite in 0u8..=255 {
            for timer in [0u8, 1, 2, 255] {
                for frame in 0..4 {
                    let mut expected = dirty(state);
                    expected[11] = sprite;
                    expected[28] = timer;
                    let mut actor = Actor::from_raw_slot(&expected);
                    assert_eq!(
                        update(
                            &mut actor,
                            Inputs {
                                frame: FrameTick::new(frame),
                                ..input()
                            }
                        ),
                        Ok(None)
                    );
                    if frame == 0 {
                        if state == 4 {
                            expected[11] = sprite.wrapping_add(1);
                            if sprite == 61 {
                                expected[15] = 5;
                                expected[24] = 48;
                            }
                        } else if sprite != 65 {
                            expected[11] = sprite.wrapping_add(1);
                        } else {
                            expected[28] = timer.wrapping_sub(1);
                            if timer == 1 {
                                expected[15] = 0;
                                expected[18] = 0;
                                expected[11] = 14;
                            }
                        }
                    }
                    assert_eq!(actor.to_raw_slot(), expected);
                }
            }
        }
    }
}

#[test]
fn handler_0xa2be_rise_moves_every_tick_blinks_every_four_and_subtracts_exactly_two() {
    for leg in 0u8..=255 {
        for frame in 0..4 {
            let mut expected = dirty(5);
            expected[24] = leg;
            expected[9] = 1;
            let mut actor = Actor::from_raw_slot(&expected);
            let sound = update(
                &mut actor,
                Inputs {
                    frame: FrameTick::new(frame),
                    ..input()
                },
            )
            .unwrap();
            if frame == 0 {
                expected[11] ^= 1;
            }
            expected[9] = 255;
            expected[24] = leg.wrapping_sub(2);
            if leg == 2 {
                expected[15] = 6;
                expected[28] = 6;
            }
            assert_eq!(actor.to_raw_slot(), expected);
            assert_eq!(sound, if leg == 2 { Some(13) } else { None });
        }
    }
}

#[test]
fn handler_0xa2be_invalid_state_and_missing_motion_data_preserve_slot() {
    for state in [3, 7, 255] {
        let raw = dirty(state);
        let mut actor = Actor::from_raw_slot(&raw);
        let result = update(
            &mut actor,
            Inputs {
                drop_profile: &[],
                ..input()
            },
        );
        if state == 3 {
            assert_eq!(
                result,
                Err(UpdateError::Motion(MotionProfileError {
                    phase: raw[16],
                    cap: 21,
                    profile_len: 0
                }))
            );
        } else {
            assert_eq!(result, Err(UpdateError::State(state)));
        }
        assert_eq!(actor.to_raw_slot(), raw);
    }
}
