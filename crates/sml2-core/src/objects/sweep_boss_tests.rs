use super::*;

// Independently transcribed ROM `$B1A3`, `$B217`, and `$9595`.
const HOVER: [u8; 13] = [0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2];
const SHORT: [u8; 13] = [3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0, 0];
const RAMP: [u8; 27] = [
    0, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0,
];
fn sweep() -> [u8; 112] {
    let mut profile = [0; 112];
    profile[..27].copy_from_slice(&RAMP);
    profile
}
fn input() -> Inputs<'static> {
    Inputs {
        frame: FrameTick(0),
        div: 255,
        fade_idle: true,
        hit: None,
        primary: true,
        stun: HitStunContext {
            removal_pending: false,
            definition_sprite: 54,
            saved_sprite: 52,
            saved_state: 6,
            saved_contact_kind: ContactKind::from_raw(69),
        },
        timer_reload: 80,
        volleys: 4,
        children_busy: [false; 2],
        hover_profile: &HOVER,
        sweep_profile: &[],
    }
}
fn dirty(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    raw[15] = state;
    raw
}

#[test]
fn handler_0xa0b9_init_fade_and_hit_whole_slot() {
    for fade_idle in [false, true] {
        let raw = dirty(0);
        let mut actor = Actor::from_raw_slot(&raw);
        let effects = update(
            &mut actor,
            Inputs {
                fade_idle,
                ..input()
            },
        )
        .unwrap();
        let mut expected = raw;
        if fade_idle {
            expected[15] = 3;
            expected[18] = 69;
            expected[28] = 1;
        }
        assert_eq!(actor.to_raw_slot(), expected);
        assert_eq!(effects.volleys, if fade_idle { Some(4) } else { None });
    }
    for state in [0, 3, 6, 8, 255] {
        for stomp in [false, true] {
            let raw = dirty(state);
            let mut actor = Actor::from_raw_slot(&raw);
            let effects = update(
                &mut actor,
                Inputs {
                    hit: Some(stomp),
                    ..input()
                },
            )
            .unwrap();
            assert_eq!(
                effects.saved_hit,
                Some((raw[11], state, ContactKind::from_raw(raw[18])))
            );
            let mut expected = raw;
            expected[18] = 0;
            expected[15] = if stomp { 9 } else { 1 };
            if stomp {
                expected[11] = 53;
                expected[28] = 1;
            }
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}

#[test]
fn handler_0xa0b9_hover_profiles_phase_equality_and_timer_wrap() {
    for state in [3, 6] {
        for leg in [0u8, 1, 2, 255] {
            for phase in 0u8..=12 {
                for timer in [0u8, 1, 2, 255] {
                    let mut expected = dirty(state);
                    expected[9] = 255;
                    expected[16] = phase;
                    expected[24] = leg;
                    expected[28] = timer;
                    let mut actor = Actor::from_raw_slot(&expected);
                    let effects = update(&mut actor, input()).unwrap();
                    let step = if leg == 0 {
                        SHORT[phase as usize]
                    } else {
                        HOVER[phase as usize]
                    };
                    expected[9] = if leg == 0 {
                        255u8.wrapping_add(step)
                    } else {
                        255u8.wrapping_sub(step)
                    };
                    expected[16] = if phase == 12 { 12 } else { phase + 1 };
                    if expected[16] == 12 {
                        expected[16] = 0;
                        expected[24] ^= 1;
                    }
                    expected[28] = if timer == 1 {
                        96
                    } else {
                        timer.wrapping_sub(1)
                    };
                    assert_eq!(actor.to_raw_slot(), expected);
                    assert_eq!(effects.volleys, if timer == 1 { Some(3) } else { None });
                    assert_eq!(
                        effects.children.iter().filter(|c| c.is_some()).count(),
                        if timer == 1 { 2 } else { 0 }
                    );
                }
            }
        }
    }
}

#[test]
fn handler_0xa0b9_empty_volley_starts_arc_without_motion_or_timer_change() {
    for state in [3, 6] {
        let mut expected = dirty(state);
        let mut actor = Actor::from_raw_slot(&expected);
        let effects = update(
            &mut actor,
            Inputs {
                volleys: 0,
                ..input()
            },
        )
        .unwrap();
        expected[24] = 0;
        expected[15] += 1;
        expected[16] = 92;
        assert_eq!(actor.to_raw_slot(), expected);
        assert_eq!(effects.volleys, None);
        assert!(effects.children.iter().all(Option::is_none));
    }
}

#[test]
fn handler_0xa0b9_all_arc_samples_wrap_and_exact_endpoints() {
    let profile = sweep();
    for state in [4, 5, 7, 8] {
        for phase in 0u8..112 {
            for div in 0u8..=3 {
                let mut expected = dirty(state);
                expected[9] = if state == 4 || state == 7 { 255 } else { 0 };
                expected[10] = if state >= 7 { 255 } else { 0 };
                expected[16] = phase;
                let mut actor = Actor::from_raw_slot(&expected);
                let effects = update(
                    &mut actor,
                    Inputs {
                        div,
                        sweep_profile: &profile,
                        ..input()
                    },
                )
                .unwrap();
                let distance = if phase < 27 { RAMP[phase as usize] } else { 0 };
                expected[10] = if state >= 7 {
                    expected[10].wrapping_add(distance)
                } else {
                    expected[10].wrapping_sub(distance)
                };
                if state == 4 || state == 7 {
                    expected[9] = 0;
                    expected[16] = phase.wrapping_sub(1);
                    if phase == 1 {
                        expected[16] = 1;
                        expected[15] += 1;
                        expected[13] ^= 32;
                    }
                    assert_eq!(effects.volleys, None);
                } else {
                    expected[9] = 255;
                    expected[16] = phase + 1;
                    if phase == 92 {
                        expected[16] = 0;
                        expected[15] = if state == 5 { 6 } else { 3 };
                        expected[9] = 48;
                        expected[10] = if state == 5 { 32 } else { 144 };
                        expected[28] = 80;
                    }
                    assert_eq!(effects.volleys, if phase == 92 { Some(div) } else { None });
                }
                assert_eq!(
                    actor.to_raw_slot(),
                    expected,
                    "state {state}, phase {phase}"
                );
            }
        }
    }
}

#[test]
fn handler_0xa0b9_delayed_stomp_sound_and_shared_stun_and_morph_order() {
    for timer in [0u8, 1, 2, 255] {
        let mut expected = dirty(9);
        expected[28] = timer;
        let mut actor = Actor::from_raw_slot(&expected);
        let effects = update(&mut actor, input()).unwrap();
        expected[28] = timer.wrapping_sub(1);
        if timer == 1 {
            expected[15] = 1;
            expected[28] = 80;
        }
        assert_eq!(actor.to_raw_slot(), expected);
        assert_eq!(effects.sound1, if timer == 1 { Some(5) } else { None });
    }
    for frame in 0..4 {
        for primary in [false, true] {
            for phase in [0u8, 3, 255] {
                for timer in [0u8, 1, 2] {
                    let mut expected = dirty(2);
                    expected[26] = phase;
                    expected[28] = timer;
                    let mut actor = Actor::from_raw_slot(&expected);
                    let effects = update(
                        &mut actor,
                        Inputs {
                            frame: FrameTick(frame),
                            primary,
                            ..input()
                        },
                    )
                    .unwrap();
                    if frame == 0 {
                        expected[26] = phase.wrapping_add(1);
                        if phase == 3 {
                            expected[26] = 0;
                            expected[11] = expected[11].wrapping_sub(3);
                        } else {
                            expected[11] = expected[11].wrapping_add(1);
                        }
                        expected[14] ^= 16;
                    }
                    expected[28] = timer.wrapping_sub(1);
                    if timer == 1 {
                        expected[15] = 0;
                        expected[5] = 3;
                        expected[26] = 0;
                        expected[14] = 0;
                    }
                    assert_eq!(actor.to_raw_slot(), expected);
                    assert_eq!(effects.silence_music, timer == 1 && primary);
                }
            }
        }
    }
    let mut expected = dirty(1);
    expected[28] = 1;
    let mut actor = Actor::from_raw_slot(&expected);
    update(&mut actor, input()).unwrap();
    expected[28] = 0;
    expected[14] = 0;
    expected[11] = 52;
    expected[15] = 6;
    expected[18] = 69;
    assert_eq!(actor.to_raw_slot(), expected);
}

#[test]
fn handler_0xa0b9_missing_profiles_report_failure_after_source_prefix_only() {
    for state in [3, 4, 5, 6, 7, 8, 255] {
        let mut expected = dirty(state);
        expected[16] = 255;
        expected[24] = 1;
        let mut actor = Actor::from_raw_slot(&expected);
        assert!(update(&mut actor, input()).is_err());
        if matches!(state, 4 | 7) {
            expected[9] = expected[9].wrapping_add(1);
        }
        if matches!(state, 5 | 8) {
            expected[9] = expected[9].wrapping_sub(1);
        }
        assert_eq!(actor.to_raw_slot(), expected);
    }
}
