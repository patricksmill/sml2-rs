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
            busy: 0x81
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
                busy: 0x81
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
                busy: 0x81
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
