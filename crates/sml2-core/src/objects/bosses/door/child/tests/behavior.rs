use super::*;

fn input(phase: u8, frame: u64, active: bool) -> Inputs {
    Inputs {
        frame: FrameTick::new(frame),
        door_phase: phase,
        sequence_active: active,
    }
}

fn slot(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    raw[15] = state;
    raw
}

#[test]
fn door_child_init_flip_checks_and_animation_preserve_other_slot_bytes() {
    for flip in 0..=255 {
        for frame in 0..8 {
            let mut expected = slot(0);
            expected[9] = 12;
            expected[10] = 250;
            expected[13] = flip;
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(
                update(&mut actor, input(2, frame, false)),
                Ok(Effects::default())
            );
            if frame % 4 == 0 {
                expected[11] ^= 1;
            }
            expected[18] = 64;
            expected[15] = if flip & 0x20 == 0 { 1 } else { 2 };
            expected[9] = 240;
            expected[10] = if flip == 0 { 234 } else { 10 };
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn door_child_all_rom_profile_samples_match_independent_source_patterns() {
    // Source `$99E2..$9C18`: lengths and zero positions transcribed from
    // the seven db tables, independently of the implementation's arrays.
    for (door_phase, length) in [(0, 128), (1, 80), (2, 60), (3, 76), (4, 128), (255, 128)] {
        for phase in 0..length {
            let dx = match door_phase {
                2 => u8::from(phase % 5 != 4),
                3 => u8::from(phase % 5 == 0),
                _ => 1,
            };
            let dy = if door_phase == 1 {
                u8::from(phase % 20 != 19)
            } else {
                1
            };
            for state in [1, 2] {
                let mut expected = slot(state);
                expected[9] = 255;
                expected[10] = 0;
                expected[16] = phase;
                let mut actor = Actor::from_raw_slot(&expected);
                assert_eq!(
                    update(&mut actor, input(door_phase, 1, false)),
                    Ok(Effects::default())
                );
                expected[9] = 255u8.wrapping_add(dy);
                expected[10] = if state == 1 { 0u8.wrapping_sub(dx) } else { dx };
                expected[16] += 1;
                assert_eq!(
                    actor.to_raw_slot(),
                    expected,
                    "door={door_phase}, phase={phase}, state={state}"
                );
            }
        }
    }
}

#[test]
fn door_child_sentinels_pause_without_moving_or_advancing_phase() {
    for (door_phase, phase) in [(0, 128), (1, 80), (2, 60), (3, 76), (255, 128)] {
        for state in [1, 2] {
            let mut expected = slot(state);
            expected[11] = 255;
            expected[16] = phase;
            let mut actor = Actor::from_raw_slot(&expected);
            update(&mut actor, input(door_phase, 1, false)).unwrap();
            expected[11] = 1;
            expected[15] = 3;
            expected[28] = 32;
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn door_child_pause_wraps_and_produces_sequence_only_for_nonzero_door_phase() {
    for phase in [0, 1, 2, 3, 255] {
        for timer in [0u8, 1, 2, 32, 255] {
            let mut expected = slot(3);
            expected[28] = timer;
            let mut actor = Actor::from_raw_slot(&expected);
            let effects = update(&mut actor, input(phase, 1, false)).unwrap();
            expected[28] = timer.wrapping_sub(1);
            if phase != 0 && timer == 1 {
                expected[15] = 4;
            }
            assert_eq!(effects.activate_sequence, phase != 0 && timer == 1);
            assert_eq!(effects.counted_remove, phase == 0 && timer == 1);
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn door_child_waits_for_consumer_clear_and_still_animates() {
    for active in [false, true] {
        for frame in 0..8 {
            let mut expected = slot(4);
            let mut actor = Actor::from_raw_slot(&expected);
            let effects = update(&mut actor, input(2, frame, active)).unwrap();
            assert_eq!(effects.counted_remove, !active);
            assert!(!effects.activate_sequence);
            if frame % 4 == 0 {
                expected[11] ^= 1;
            }
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn door_child_invalid_inputs_report_errors_after_animation() {
    for (door_phase, phase) in [(0, 129), (1, 81), (2, 61), (3, 77)] {
        let mut expected = slot(1);
        expected[16] = phase;
        let mut actor = Actor::from_raw_slot(&expected);
        assert_eq!(
            update(&mut actor, input(door_phase, 0, false)),
            Err(UpdateError::InvalidPhase(phase))
        );
        expected[11] ^= 1;
        assert_eq!(actor.to_raw_slot(), expected);
    }
    let mut expected = slot(5);
    let mut actor = Actor::from_raw_slot(&expected);
    assert_eq!(
        update(&mut actor, input(0, 0, false)),
        Err(UpdateError::InvalidState(5))
    );
    expected[11] ^= 1;
    assert_eq!(actor.to_raw_slot(), expected);
}
