use super::*;
use crate::objects::*;

// Independent ROM witnesses: $B1A3 and the nonzero prefix of $9595.
const ENTRANCE: [u8; 13] = [0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2];
const SWEEP: [u8; 112] = {
    let prefix = [
        0, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0,
    ];
    let mut table = [0; 112];
    let mut i = 0;
    while i < prefix.len() {
        table[i] = prefix[i];
        i += 1;
    }
    table
};

fn inputs(frame: u8) -> Inputs<'static> {
    Inputs {
        frame: FrameTick::new(u64::from(frame)),
        fade_idle: true,
        hit: None,
        primary: true,
        timer_reload: 80,
        removal_pending: false,
        definition_sprite: 1,
        saved_sprite: 0xA9,
        saved_state: 7,
        remaining_hits: 6,
        alternate_phase: 0,
        entrance_profile: &ENTRANCE,
        sweep_profile: &SWEEP,
    }
}

fn slot(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(0x93));
    raw[B_FSTATE] = state;
    raw
}

#[test]
fn handler_0x93a7_intro_all_frame_gates_and_hit_priority_preserve_slot_bytes() {
    for frame in 0..=255 {
        for fade_idle in [false, true] {
            let raw = slot(0);
            let mut actor = Actor::from_raw_slot(&raw);
            let mut input = inputs(frame);
            input.fade_idle = fade_idle;
            assert_eq!(update(&mut actor, input), Ok(Effects::default()));
            let mut expected = raw;
            if frame % 4 == 0 {
                expected[B_SPRITE] ^= 6;
            }
            if fade_idle {
                expected[B_TYPE] = 69;
                expected[B_SPRITE] = 3;
                expected[B_FSTATE] = 3;
            }
            assert_eq!(actor.to_raw_slot(), expected);
        }
        for state in [0, 1, 2, 3, 4, 5, 6, 7, 255] {
            for stomp in [false, true] {
                let raw = slot(state);
                let mut actor = Actor::from_raw_slot(&raw);
                let mut input = inputs(frame);
                input.hit = Some(stomp);
                let effect = update(&mut actor, input).unwrap();
                let mut expected = raw;
                if stomp {
                    expected[B_SPRITE] = 0;
                    expected[B_T28] = 80;
                }
                expected[B_TYPE] = 0;
                expected[B_FSTATE] = 1;
                assert_eq!(actor.to_raw_slot(), expected);
                assert_eq!(
                    effect,
                    Effects {
                        saved_hit: Some((raw[B_SPRITE], state, ContactKind::from_raw(raw[B_TYPE]))),
                        ..Effects::default()
                    }
                );
            }
        }
    }
}

#[test]
fn handler_0x93a7_stun_exact_sixteen_frame_recovery_and_defeat_boundaries() {
    for frame in 0..=255 {
        for timer in [0u8, 1, 2, 255] {
            for animation in [0u8, 14, 15, 16, 255] {
                for removal_pending in [false, true] {
                    let mut raw = slot(1);
                    raw[B_T28] = timer;
                    raw[B_ANIMFRAME] = animation;
                    let mut actor = Actor::from_raw_slot(&raw);
                    let mut input = inputs(frame);
                    input.removal_pending = removal_pending;
                    assert_eq!(update(&mut actor, input), Ok(Effects::default()));
                    let mut expected = raw;
                    if frame % 4 == 0 {
                        expected[B_FLASH] ^= 0x10;
                    }
                    if timer != 0 {
                        if frame % 2 == 0 {
                            expected[B_SPRITE] ^= 1;
                        }
                        expected[B_T28] = timer - 1;
                    }
                    if timer <= 1 {
                        if removal_pending {
                            expected[B_FSTATE] = 2;
                            expected[B_T28] = 32;
                            expected[B_SPRITE] = 1;
                        } else {
                            expected[B_SPRITE] = 4;
                            expected[B_ANIMFRAME] = animation.wrapping_add(1);
                            if animation == 15 {
                                expected[B_ANIMFRAME] = 0;
                                expected[B_FLASH] = 0;
                                expected[B_SPRITE] = 0xA9;
                                expected[B_FSTATE] = 7;
                                expected[B_TYPE] = 69;
                            }
                        }
                    }
                    assert_eq!(
                        actor.to_raw_slot(),
                        expected,
                        "frame={frame} timer={timer} anim={animation} remove={removal_pending}"
                    );
                }
            }
        }
    }
}

#[test]
fn handler_0x93a7_entrance_reads_before_increment_and_caps_at_twelve() {
    for phase in 0..=12 {
        let mut raw = slot(3);
        raw[B_PHASE] = phase;
        raw[B_SCRY] = 255;
        let mut actor = Actor::from_raw_slot(&raw);
        let effect = update(&mut actor, inputs(0)).unwrap();
        let mut expected = raw;
        expected[B_SCRY] = 255u8.wrapping_add(ENTRANCE[usize::from(phase)]);
        expected[B_PHASE] = phase + u8::from(phase != 12);
        if phase >= 11 {
            expected[B_PHASE] = 1;
            expected[B_FSTATE] = 4;
        }
        assert_eq!(actor.to_raw_slot(), expected);
        assert_eq!(effect.sound2, (phase >= 11).then_some(2));
    }
}

#[test]
fn handler_0x93a7_forward_sweeps_wrap_threshold_and_low_health_latch() {
    for state in [4, 6] {
        for phase in [0u8, 1, 11, 12, 26, 92, 103, 104, 105, 111] {
            for hits in [0, 1, 2, 3, 6, 255] {
                for x in [0u8, 255] {
                    let mut raw = slot(state);
                    raw[B_PHASE] = phase;
                    raw[B_SCRX] = x;
                    raw[B_SCRY] = 0;
                    let mut actor = Actor::from_raw_slot(&raw);
                    let mut input = inputs(0);
                    input.remaining_hits = hits;
                    let effect = update(&mut actor, input).unwrap();
                    let mut expected = raw;
                    if phase >= 12 {
                        expected[B_SCRY] = 255;
                    }
                    let distance = SWEEP[usize::from(phase)];
                    expected[B_SCRX] = if state == 4 {
                        x.wrapping_add(distance)
                    } else {
                        x.wrapping_sub(distance)
                    };
                    expected[B_PHASE] = phase + 1;
                    if phase == 104 {
                        expected[B_PHASE] = if hits < 3 { 92 } else { 104 };
                        expected[B_FSTATE] = state + 1;
                        expected[B_T28] = 80;
                    }
                    assert_eq!(actor.to_raw_slot(), expected);
                    assert_eq!(
                        effect,
                        Effects {
                            alternate_phase: (phase == 104 && hits < 3).then_some(1),
                            ..Effects::default()
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn handler_0x93a7_return_sweeps_delay_blink_order_phase_wrap_and_turns() {
    for state in [5, 7] {
        for phase in [0u8, 1, 11, 12, 26, 92, 104, 111] {
            for timer in [0u8, 1, 2, 255] {
                for alternate in [0, 1, 255] {
                    for frame in 0..4 {
                        let mut raw = slot(state);
                        raw[B_PHASE] = phase;
                        raw[B_T28] = timer;
                        raw[B_SCRX] = 0;
                        raw[B_SCRY] = 255;
                        let mut actor = Actor::from_raw_slot(&raw);
                        let mut input = inputs(frame);
                        input.alternate_phase = alternate;
                        let effect = update(&mut actor, input).unwrap();
                        let mut expected = raw;
                        let mut expected_effect = Effects::default();
                        if timer != 0 {
                            if frame == 0 {
                                expected[B_SPRITE] ^= 1;
                            }
                            expected[B_T28] = timer - 1;
                        }
                        if timer <= 1 {
                            if timer == 1 {
                                expected[B_SPRITE] = 5;
                                expected_effect.sound1 = Some(8);
                            }
                            if frame == 0 {
                                expected[B_SPRITE] ^= 6;
                            }
                            if alternate != 0 || phase >= 12 {
                                expected[B_SCRY] = 0;
                            }
                            let distance = SWEEP[usize::from(phase)];
                            expected[B_SCRX] = if state == 5 {
                                0u8.wrapping_sub(distance)
                            } else {
                                distance
                            };
                            expected[B_PHASE] = phase.wrapping_sub(1);
                            if phase == 1 {
                                expected[B_PHASE] = 1;
                                expected[B_FSTATE] = if state == 5 { 6 } else { 4 };
                                expected[B_SPRITE] = 2;
                                expected[B_FLIP] ^= 0x20;
                                expected_effect.sound2 = Some(2);
                            }
                        }
                        assert_eq!(actor.to_raw_slot(), expected,
                            "state={state} phase={phase} timer={timer} alt={alternate} frame={frame}");
                        assert_eq!(effect, expected_effect);
                    }
                }
            }
        }
    }
}

#[test]
fn handler_0x93a7_missing_profiles_and_unknown_states_report_errors() {
    for state in [3, 4, 5, 6, 7, 8, 255] {
        let mut actor = Actor::from_raw_slot(&slot(state));
        actor.timer = 0;
        let mut input = inputs(0);
        input.entrance_profile = &[];
        input.sweep_profile = &[];
        assert!(update(&mut actor, input).is_err());
    }
}
