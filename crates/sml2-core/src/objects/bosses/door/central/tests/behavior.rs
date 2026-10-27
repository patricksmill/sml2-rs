use super::*;

// Independently transcribed ROM `$B1A3..$B1B8`, selected by `$B18F`.
const FALL: [u8; 22] = [
    0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3,
];

fn input(remove_mode: u8, active: bool, phase: u8, frame: u64) -> Inputs<'static> {
    Inputs {
        remove_mode,
        sequence_active: active,
        door_phase: phase,
        frame: FrameTick::new(frame),
        fall_profile: &FALL,
    }
}

fn slot(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    raw[15] = state;
    raw
}

#[test]
fn central_door_waits_for_exact_phase_and_preserves_whole_slot() {
    for phase in 0..=255 {
        for active in [false, true] {
            let mut expected = slot(0);
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(
                update(&mut actor, input(0, active, phase, 0)),
                Ok(Effects::default())
            );
            if active && phase == 2 {
                expected[15] = 1;
                expected[24] = 64;
                expected[28] = 64;
                expected[11] = 25;
            }
            assert_eq!(
                actor.to_raw_slot(),
                expected,
                "active={active}, phase={phase}"
            );
        }
    }
}

#[test]
fn central_door_flash_cadence_and_countdown_wrap() {
    for frame in 0..=255 {
        for timer in [0u8, 1, 2, 64, 255] {
            let mut expected = slot(1);
            expected[28] = timer;
            let mut actor = Actor::from_raw_slot(&expected);
            let effects = update(&mut actor, input(0, true, 2, frame)).unwrap();
            if frame % 4 == 0 {
                expected[13] ^= 0x20;
            }
            expected[28] = timer.wrapping_sub(1);
            if timer == 1 {
                expected[15] = 2;
                expected[11] = 19;
            }
            assert_eq!(effects.sound, if timer == 1 { Some(5) } else { None });
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn central_door_rise_rider_removal_and_byte_boundaries() {
    for remove in [0, 1, 3, 255] {
        for flags in [0, 8, 0xF7, 0xFF] {
            for y in [0u8, 1, 2, 116, 255] {
                for distance in [0u8, 1, 2, 3, 64, 255] {
                    let mut expected = slot(2);
                    expected[9] = y;
                    expected[23] = flags;
                    expected[24] = distance;
                    let mut actor = Actor::from_raw_slot(&expected);
                    let effects = update(&mut actor, input(remove, true, 2, 1)).unwrap();
                    if remove != 0 {
                        expected[18] = 0;
                    } else {
                        expected[9] = y.wrapping_sub(2);
                        expected[24] = distance.wrapping_sub(2);
                        if distance == 2 {
                            expected[15] = 3;
                            expected[28] = 32;
                        }
                    }
                    assert_eq!(
                        effects.carry_dy,
                        if remove == 0 && flags & 8 != 0 {
                            Some(-2)
                        } else {
                            None
                        }
                    );
                    assert_eq!(actor.to_raw_slot(), expected);
                }
            }
        }
    }
}

#[test]
fn central_door_pause_clears_sequence_only_on_expiry() {
    for timer in [0u8, 1, 2, 32, 255] {
        let mut expected = slot(3);
        expected[28] = timer;
        let mut actor = Actor::from_raw_slot(&expected);
        let effects = update(&mut actor, input(0, true, 2, 0)).unwrap();
        expected[28] = timer.wrapping_sub(1);
        if timer == 1 {
            expected[15] = 4;
        }
        assert_eq!(effects.clear_sequence, timer == 1);
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn central_door_fall_uses_rom_steps_and_clamps_on_following_call() {
    for (phase, step) in FALL.into_iter().enumerate() {
        let mut expected = slot(4);
        expected[9] = 115;
        expected[16] = phase as u8;
        let mut actor = Actor::from_raw_slot(&expected);
        update(&mut actor, input(0, false, 0, 0)).unwrap();
        expected[9] = 115 + step;
        expected[16] = (phase as u8 + 1).min(21);
        assert_eq!(actor.to_raw_slot(), expected);
    }
    for y in 116..=255 {
        let mut expected = slot(4);
        expected[9] = y;
        let mut actor = Actor::from_raw_slot(&expected);
        update(&mut actor, input(0, false, 0, 0)).unwrap();
        expected[9] = 116;
        expected[15] = 0;
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn central_door_reward_override_precedes_every_state_and_preserves_other_bytes() {
    for state in 0..=255 {
        let mut expected = slot(state);
        let mut actor = Actor::from_raw_slot(&expected);
        assert_eq!(
            update(&mut actor, input(2, false, 0, 0)),
            Ok(Effects::default())
        );
        expected[9] = 116;
        expected[10] = 88;
        expected[5] = 4;
        expected[11] = 9;
        expected[15] = 0;
        expected[16] = 0;
        expected[18] = 0;
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn central_door_invalid_state_or_missing_profile_reports_error_without_mutation() {
    let raw = slot(5);
    let mut actor = Actor::from_raw_slot(&raw);
    assert_eq!(
        update(&mut actor, input(0, true, 2, 0)),
        Err(UpdateError::InvalidState(5))
    );
    assert_eq!(actor.to_raw_slot(), raw);
    let mut raw = slot(4);
    raw[9] = 115;
    raw[16] = 0;
    let mut actor = Actor::from_raw_slot(&raw);
    let mut inputs = input(0, true, 2, 0);
    inputs.fall_profile = &FALL[..21];
    assert!(matches!(
        update(&mut actor, inputs),
        Err(UpdateError::Motion(_))
    ));
    assert_eq!(actor.to_raw_slot(), raw);
}
