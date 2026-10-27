use super::*;

fn input(frame: u64) -> Inputs {
    Inputs {
        frame: FrameTick::new(frame),
        player: ScreenPosition { x: 88, y: 100 },
        remove_mode: 0,
        reward_slot: false,
        hit_processed: false,
    }
}

fn slot(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    raw[15] = state;
    raw
}

#[test]
fn patrol_child_idle_sprite_gate_arms_without_moving_or_resetting_other_fields() {
    for sprite in [0, 13, 14, 15, 49, 255] {
        let mut expected = slot(0);
        expected[11] = sprite;
        let mut actor = Actor::from_raw_slot(&expected);
        assert_eq!(update(&mut actor, input(0)), Ok(Effects::default()));
        if sprite != 14 {
            expected[15] = 3;
            expected[18] = 69;
        }
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn patrol_child_drop_matches_rom_profile_and_faces_at_cap_including_x_ties() {
    // ROM `$B214..$B223`, c=15. Source adds each selected distance to Y.
    let steps = [3u8, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0, 0];
    for (phase, step) in steps.into_iter().enumerate() {
        for target in [0, 87, 88, 89, 255] {
            let mut expected = slot(3);
            expected[9] = 255;
            expected[10] = 88;
            expected[16] = phase as u8;
            let mut actor = Actor::from_raw_slot(&expected);
            let mut inputs = input(1);
            inputs.player.x = target;
            let effects = update(&mut actor, inputs).unwrap();
            expected[9] = 255u8.wrapping_add(step);
            if phase >= 14 {
                expected[16] = 0;
                expected[11] = 50;
                expected[15] = if target >= 88 { 2 } else { 1 };
                expected[13] = if target >= 88 { 32 } else { 0 };
                expected[25] = 32;
                assert_eq!(effects.target_relation, Some(target.cmp(&88)));
            } else {
                expected[16] = phase as u8 + 1;
                assert!(effects.target_relation.is_none());
            }
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn patrol_child_flight_cadence_x_wrap_despawn_and_unsigned_vertical_ties() {
    for state in [1, 2] {
        for frame in 0..8 {
            for x in [0u8, 1, 16, 183, 184, 185, 255] {
                for target_y in [0u8, 99, 100, 101, 255] {
                    let mut expected = slot(state);
                    expected[9] = 100;
                    expected[10] = x;
                    let mut actor = Actor::from_raw_slot(&expected);
                    let mut inputs = input(frame);
                    inputs.player.y = target_y;
                    let effects = update(&mut actor, inputs).unwrap();
                    if frame % 4 == 0 {
                        expected[11] ^= 1;
                        expected[10] = if state == 1 {
                            x.wrapping_sub(1)
                        } else {
                            x.wrapping_add(1)
                        };
                        if expected[10] >= 184 {
                            expected[11] = 14;
                            expected[15] = 0;
                            assert!(effects.target_relation.is_none());
                        } else {
                            expected[9] = if target_y < 100 { 99 } else { 101 };
                            assert_eq!(effects.target_relation, Some(target_y.cmp(&100)));
                        }
                    } else {
                        assert!(effects.target_relation.is_none());
                    }
                    assert_eq!(
                        actor.to_raw_slot(),
                        expected,
                        "state={state}, frame={frame}, x={x}, targetY={target_y}"
                    );
                }
            }
        }
    }
}

#[test]
fn patrol_child_screen_y_tracking_wraps_after_unsigned_comparison() {
    for (y, target, next) in [(0, 0, 1), (0, 255, 1), (255, 0, 254), (255, 255, 0)] {
        let mut expected = slot(2);
        expected[9] = y;
        expected[10] = 88;
        let mut actor = Actor::from_raw_slot(&expected);
        let mut inputs = input(4);
        inputs.player.y = target;
        update(&mut actor, inputs).unwrap();
        expected[9] = next;
        expected[10] = 89;
        expected[11] ^= 1;
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn patrol_child_removal_override_is_exclusive_to_reward_slot_and_precedes_hits() {
    for state in 0..=255 {
        for reward in [false, true] {
            let mut expected = slot(state);
            let mut actor = Actor::from_raw_slot(&expected);
            let mut inputs = input(0);
            inputs.remove_mode = 2;
            inputs.reward_slot = reward;
            inputs.hit_processed = true;
            assert_eq!(update(&mut actor, inputs), Ok(Effects::default()));
            if reward {
                expected[9] = 112;
                expected[10] = 88;
                expected[5] = 4;
                expected[11] = 9;
                expected[15] = 0;
                expected[16] = 0;
                expected[18] = 0;
            }
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn patrol_child_processed_damage_skips_all_motion_even_for_invalid_state() {
    for state in [0, 1, 2, 3, 255] {
        for remove in [0, 1, 3, 255] {
            let expected = slot(state);
            let mut actor = Actor::from_raw_slot(&expected);
            let mut inputs = input(4);
            inputs.remove_mode = remove;
            inputs.hit_processed = true;
            assert_eq!(update(&mut actor, inputs), Ok(Effects::default()));
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn patrol_child_invalid_state_or_drop_phase_reports_error_without_mutation() {
    let mut raw = slot(3);
    raw[16] = 16;
    let mut actor = Actor::from_raw_slot(&raw);
    assert!(matches!(
        update(&mut actor, input(4)),
        Err(UpdateError::Motion(_))
    ));
    assert_eq!(actor.to_raw_slot(), raw);
    let raw = slot(4);
    let mut actor = Actor::from_raw_slot(&raw);
    assert_eq!(
        update(&mut actor, input(4)),
        Err(UpdateError::InvalidState(4))
    );
    assert_eq!(actor.to_raw_slot(), raw);
}
