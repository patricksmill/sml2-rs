use super::*;

fn dirty_state() -> (PulseEffectState, PulseMusicState) {
    (
        PulseEffectState {
            request: 6,
            active_effect: 0xe1,
            timer: 0xfe,
            step: 0x99,
            duration: 0x53,
            busy: 0x81,
            freq_lo: 0,
            freq_hi: 0,
            phase: 0,
        },
        PulseMusicState {
            flags: PulseMusicFlags::from_bits(0x65),
            vibrato_timer: 0x37,
        },
    )
}

fn first_frame() -> [PulseWrite; 5] {
    [
        PulseWrite::Sweep(0),
        PulseWrite::DutyLength(0xba),
        PulseWrite::Envelope(0xc0),
        PulseWrite::FrequencyLow(0xb6),
        PulseWrite::FrequencyControl(0xc6),
    ]
}

fn second_frame() -> [PulseWrite; 5] {
    [
        PulseWrite::Sweep(0),
        PulseWrite::DutyLength(0xba),
        PulseWrite::Envelope(0xc0),
        PulseWrite::FrequencyLow(0x39),
        PulseWrite::FrequencyControl(0xc7),
    ]
}

#[test]
fn pulse_effect6_start_preserves_dirty_values_and_appends_exact_ordered_writes() {
    let (mut effect, mut music) = dirty_state();
    let mut out = vec![PulseWrite::Envelope(0x42)];
    effect.start_effect6(&mut music, &mut out);
    assert_eq!(out[0], PulseWrite::Envelope(0x42));
    assert_eq!(&out[1..], &first_frame());
    assert_eq!(
        effect,
        PulseEffectState {
            request: 6,
            active_effect: 6,
            timer: 8,
            step: 0,
            duration: 0x53,
            busy: 0x81,
            freq_lo: 0,
            freq_hi: 0,
            phase: 0,
        }
    );
    assert_eq!(
        music,
        PulseMusicState {
            flags: PulseMusicFlags::from_bits(0xe5),
            vibrato_timer: 0x37
        }
    );
    assert!(music.flags.effect_owns_channel());
}

#[test]
fn pulse_effect6_complete_eight_plus_two_tick_sequence() {
    let (mut effect, mut music) = dirty_state();
    let mut out = Vec::new();
    effect.start_effect6(&mut music, &mut out);
    out.clear();
    for remaining in (1..8).rev() {
        effect.tick_effect6(&mut music, &mut out);
        assert_eq!(effect.timer, remaining);
        assert_eq!(effect.step, 0);
        assert!(out.is_empty());
    }
    effect.tick_effect6(&mut music, &mut out);
    assert_eq!(out, second_frame());
    assert_eq!((effect.timer, effect.step), (2, 1));
    assert_eq!(
        (effect.duration, effect.busy, music.vibrato_timer),
        (0x53, 0x81, 0x37)
    );
    out.clear();
    effect.request = 0x19; // New request survives continuation and stop.
    effect.tick_effect6(&mut music, &mut out);
    assert_eq!((effect.timer, effect.step), (1, 1));
    assert!(out.is_empty());
    effect.tick_effect6(&mut music, &mut out);
    assert_eq!(out, [PulseWrite::Sweep(0)]);
    assert_eq!(
        effect,
        PulseEffectState {
            request: 0x19,
            ..PulseEffectState::default()
        }
    );
    assert_eq!(
        music,
        PulseMusicState {
            flags: PulseMusicFlags::from_bits(0x65),
            vibrato_timer: 0
        }
    );
    assert!(!music.flags.effect_owns_channel());
    assert_eq!(music.flags.bits(), 0x65);
}

#[test]
fn pulse_effect6_zero_timer_wraps_without_other_changes() {
    let (mut effect, mut music) = dirty_state();
    effect.timer = 0;
    let mut expected = effect;
    expected.timer = 255;
    let expected_music = music;
    let mut out = Vec::new();
    effect.tick_effect6(&mut music, &mut out);
    assert_eq!(effect, expected);
    assert_eq!(music, expected_music);
    assert!(out.is_empty());
}

#[test]
fn pulse_effect6_step_wrap_and_equality_only_stop_preserve_dirty_state() {
    for (old_step, new_step) in [(255, 0), (2, 3)] {
        let (mut effect, mut music) = dirty_state();
        effect.timer = 1;
        effect.step = old_step;
        let mut expected = effect;
        expected.timer = 2;
        expected.step = new_step;
        let expected_music = music;
        let mut out = Vec::new();
        effect.tick_effect6(&mut music, &mut out);
        assert_eq!(effect, expected);
        assert_eq!(music, expected_music);
        assert_eq!(out, second_frame());
    }
}

#[test]
fn pulse_effect6_start_copies_noncanonical_request_without_consuming_it() {
    for request in [0, 0xff] {
        let (mut effect, mut music) = dirty_state();
        effect.request = request;
        music.flags = PulseMusicFlags::from_bits(0xff);
        let mut out = Vec::new();
        effect.start_effect6(&mut music, &mut out);
        assert_eq!((effect.request, effect.active_effect), (request, request));
        assert_eq!(music.flags.bits(), 0xff);
        assert_eq!(out, first_frame());
    }
}

fn effect7_first_frame() -> [PulseWrite; 5] {
    [
        PulseWrite::Sweep(0),
        PulseWrite::DutyLength(0x80),
        PulseWrite::Envelope(0xa7),
        PulseWrite::FrequencyLow(0x83),
        PulseWrite::FrequencyControl(0x87),
    ]
}

#[test]
fn pulse_effect7_start_reuses_common_state_and_preserves_verbatim_request() {
    for request in [0, 7, 0xff] {
        let (mut effect, mut music) = dirty_state();
        effect.request = request;
        let mut out = vec![PulseWrite::Envelope(0x42)];
        effect.start_effect7(&mut music, &mut out);
        assert_eq!(
            effect,
            PulseEffectState {
                request,
                active_effect: request,
                timer: 4,
                step: 0,
                duration: 0x53,
                busy: 0x81,
                freq_lo: 0,
                freq_hi: 0,
                phase: 0,
            }
        );
        assert_eq!(
            music,
            PulseMusicState {
                flags: PulseMusicFlags::from_bits(0xe5),
                vibrato_timer: 0x37
            }
        );
        assert_eq!(out[0], PulseWrite::Envelope(0x42));
        assert_eq!(&out[1..], &effect7_first_frame());
    }
}

#[test]
fn pulse_effect7_four_plus_eighty_ticks_frequency_only_then_shared_stop() {
    let (mut effect, mut music) = dirty_state();
    effect.request = 7;
    let mut out = Vec::new();
    effect.start_effect7(&mut music, &mut out);
    assert_eq!(out, effect7_first_frame());
    out.clear();
    for remaining in (1..4).rev() {
        effect.tick_effect7(&mut music, &mut out);
        assert_eq!(effect.timer, remaining);
        assert_eq!(effect.step, 0);
        assert!(out.is_empty());
    }
    effect.tick_effect7(&mut music, &mut out);
    assert_eq!(
        out,
        [
            PulseWrite::FrequencyLow(0x9d),
            PulseWrite::FrequencyControl(0x07)
        ]
    );
    assert_eq!((effect.timer, effect.step), (80, 1));
    out.clear();
    effect.request = 0x18;
    for remaining in (1..80).rev() {
        effect.tick_effect7(&mut music, &mut out);
        assert_eq!(
            effect,
            PulseEffectState {
                request: 0x18,
                active_effect: 7,
                timer: remaining,
                step: 1,
                duration: 0x53,
                busy: 0x81,
                freq_lo: 0,
                freq_hi: 0,
                phase: 0,
            }
        );
        assert_eq!(
            music,
            PulseMusicState {
                flags: PulseMusicFlags::from_bits(0xe5),
                vibrato_timer: 0x37
            }
        );
        assert!(out.is_empty());
    }
    effect.tick_effect7(&mut music, &mut out);
    assert_eq!(out, [PulseWrite::Sweep(0)]);
    assert_eq!(
        effect,
        PulseEffectState {
            request: 0x18,
            ..PulseEffectState::default()
        }
    );
    assert_eq!(
        music,
        PulseMusicState {
            flags: PulseMusicFlags::from_bits(0x65),
            vibrato_timer: 0
        }
    );
}

#[test]
fn pulse_effect7_zero_timer_and_step_wrap_and_equality_only_stop() {
    let (mut effect, mut music) = dirty_state();
    effect.timer = 0;
    let mut expected = effect;
    expected.timer = 255;
    let original_music = music;
    let mut out = Vec::new();
    effect.tick_effect7(&mut music, &mut out);
    assert_eq!(effect, expected);
    assert_eq!(music, original_music);
    assert!(out.is_empty());

    for (step, next_step) in [(255, 0), (2, 3)] {
        let (mut effect, mut music) = dirty_state();
        effect.timer = 1;
        effect.step = step;
        let mut expected = effect;
        expected.timer = 80;
        expected.step = next_step;
        let original_music = music;
        effect.tick_effect7(&mut music, &mut out);
        assert_eq!(effect, expected);
        assert_eq!(music, original_music);
        assert_eq!(
            out,
            [
                PulseWrite::FrequencyLow(0x9d),
                PulseWrite::FrequencyControl(0x07)
            ]
        );
        out.clear();
    }
}

#[test]
fn test_pulse_effect1_lifecycle() {
    let mut effect = PulseEffectState {
        request: 1,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(1, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 1);
    assert_eq!(effect.timer, 48);
    assert!(music.flags.effect_owns_channel());
    assert_eq!(
        out,
        vec![
            PulseWrite::Sweep(0x37),
            PulseWrite::DutyLength(0x40),
            PulseWrite::Envelope(0xD2),
            PulseWrite::FrequencyLow(0x27),
            PulseWrite::FrequencyControl(0x86),
        ]
    );

    // 47 ticks decrement
    for t in (1..48).rev() {
        out.clear();
        effect.tick(&mut music, 0, &mut out);
        assert_eq!(effect.timer, t);
        assert!(out.is_empty());
    }

    // Expiry: stops
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 0);
    assert_eq!(out, vec![PulseWrite::Sweep(0)]);
    assert!(!music.flags.effect_owns_channel());
}

#[test]
fn test_pulse_effect2_two_steps() {
    let mut effect = PulseEffectState {
        request: 2,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(2, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 2);
    assert_eq!(effect.timer, 4);

    // Advance 4 ticks to step 1
    effect.timer = 1;
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.step, 1);
    assert_eq!(effect.timer, 12);
    assert_eq!(
        out,
        vec![
            PulseWrite::Sweep(0x14),
            PulseWrite::DutyLength(0xAD),
            PulseWrite::Envelope(0xD0),
            PulseWrite::FrequencyLow(0x9B),
            PulseWrite::FrequencyControl(0xC3),
        ]
    );

    // Expiry at step 1 stops
    effect.timer = 1;
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 0);
    assert_eq!(out, vec![PulseWrite::Sweep(0)]);
}

#[test]
fn test_pulse_effect3_retrigger_guard() {
    let mut effect = PulseEffectState {
        request: 3,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(3, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 3);
    assert_eq!(effect.timer, 6);

    out.clear();
    effect.start(3, &mut music, 0, &mut out);
    assert!(out.is_empty());
    assert_eq!(effect.timer, 6);
}

#[test]
fn test_pulse_effect9_frequency_slide() {
    let mut effect = PulseEffectState {
        request: 9,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(9, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 9);
    assert_eq!(effect.busy, 1);
    assert_eq!(effect.duration, 3);
    assert_eq!(effect.freq_lo, 0xB5);
    assert_eq!(effect.freq_hi, 0x07);
    assert_eq!(effect.timer, 5);

    // 4 decrements
    for _ in 0..4 {
        out.clear();
        effect.tick(&mut music, 0, &mut out);
        assert!(out.is_empty());
    }

    // Step 1: adds delta +8 to 0x07B5 => 0x07BD
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.step, 1);
    assert_eq!(effect.freq_lo, 0xBD);
    assert_eq!(effect.freq_hi, 0x07);
    assert_eq!(
        out,
        vec![
            PulseWrite::FrequencyLow(0xBD),
            PulseWrite::FrequencyControl(0x07),
        ]
    );
}

#[test]
fn test_pulse_effect10_noop() {
    let mut effect = PulseEffectState::default();
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(10, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 0);
    assert!(out.is_empty());

    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 0);
    assert!(out.is_empty());
}

#[test]
fn test_pulse_effect11_silence_stop() {
    let mut effect = PulseEffectState {
        request: 11,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(11, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 11);

    // Advance to step 3
    effect.step = 3;
    effect.timer = 1;

    // Step 4 calls stop_silence
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 0);
    assert!(!music.flags.effect_owns_channel());
    assert_eq!(
        out,
        vec![
            PulseWrite::Sweep(0x00),
            PulseWrite::DutyLength(0x3F),
            PulseWrite::Envelope(0x00),
            PulseWrite::FrequencyLow(0x00),
            PulseWrite::FrequencyControl(0xC1),
            PulseWrite::Sweep(0x00),
        ]
    );
}

#[test]
fn test_pulse_effect12_dual_channel() {
    let mut effect = PulseEffectState {
        request: 12,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(12, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 12);
    assert_eq!(effect.duration, 6);
    assert_eq!(effect.phase, 0);
    assert_eq!(effect.timer, 4);

    // Tick: phase becomes 1 -> writes to Ch2
    effect.timer = 1;
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.phase, 1);
    assert_eq!(
        out,
        vec![
            PulseWrite::Ch2DutyLength(0x80),
            PulseWrite::Ch2Envelope(0x94),
            PulseWrite::Ch2FrequencyLow(0xCC),
            PulseWrite::Ch2FrequencyControl(0x87),
        ]
    );

    // Next expiry: phase becomes 2 -> writes to Ch1
    effect.timer = 1;
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.phase, 2);
    assert_eq!(out.len(), 5);
}

#[test]
fn test_pulse_effect16_stomp_chain() {
    let mut effect = PulseEffectState {
        request: 16,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    // Stomp chain 0 is ignored
    effect.start(16, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 0);
    assert!(out.is_empty());

    // Stomp chain 5 is ignored
    effect.start(16, &mut music, 5, &mut out);
    assert_eq!(effect.active_effect, 0);
    assert!(out.is_empty());

    // Stomp chain 1 starts
    effect.start(16, &mut music, 1, &mut out);
    assert_eq!(effect.active_effect, 16);
    assert_eq!(effect.timer, 2);
    assert_eq!(
        out,
        vec![
            PulseWrite::Sweep(0x00),
            PulseWrite::DutyLength(0xAF),
            PulseWrite::Envelope(0x80),
            PulseWrite::FrequencyLow(0x06),
            PulseWrite::FrequencyControl(0xC7),
        ]
    );
}

#[test]
fn test_pulse_effect19_fast_slide() {
    let mut effect = PulseEffectState {
        request: 19,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(19, &mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 19);
    assert_eq!(effect.duration, 10);
    assert_eq!(effect.freq_lo, 0x80);
    assert_eq!(effect.freq_hi, 0x04);

    // Tick advances step immediately and adds delta 80 to 0x0480 => 0x04D0
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.step, 1);
    assert_eq!(effect.freq_lo, 0xD0);
    assert_eq!(effect.freq_hi, 0x04);
    assert_eq!(
        out,
        vec![
            PulseWrite::FrequencyLow(0xD0),
            PulseWrite::FrequencyControl(0x04),
        ]
    );
}

#[test]
fn test_pulse_effect14_frame_table() {
    // $43C0 = [$43D0, $43D6, $43D6, $43D6, $43D6] indexed by step-1: the first
    // expiry of every trigger writes the louder $59-envelope frame, and only
    // the final pass (duration exhausted -> a=2 -> table[1]) writes $29.
    let mut effect = PulseEffectState {
        request: 14,
        ..Default::default()
    };
    let mut music = PulseMusicState::default();
    let mut out = Vec::new();

    effect.start(14, &mut music, 0, &mut out);
    assert_eq!((effect.duration, effect.step, effect.timer), (4, 0, 3));

    let mut envelopes = Vec::new();
    for _ in 0..8 {
        effect.timer = 1;
        out.clear();
        effect.tick(&mut music, 0, &mut out);
        let env = out
            .iter()
            .find_map(|w| match w {
                PulseWrite::Envelope(v) => Some(*v),
                _ => None,
            })
            .expect("effect 14 frame carries an envelope write");
        envelopes.push(env);
    }
    assert_eq!(
        envelopes,
        vec![0x59, 0x59, 0x59, 0x59, 0x29, 0x29, 0x29, 0x29],
        "step 1 uses the $43D0 frame, steps 3-5 alias $43D6"
    );

    // Step 6 stops the slot ($43A6 -> $106A8).
    effect.timer = 1;
    out.clear();
    effect.tick(&mut music, 0, &mut out);
    assert_eq!(effect.active_effect, 0);
    assert_eq!(out.first(), Some(&PulseWrite::Sweep(0)));
}
