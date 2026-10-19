use super::*;

#[test]
fn test_noise_effect1_full_lifecycle() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    // Start effect 1
    state.start(1, &mut flags, &mut out);
    assert_eq!(state.active_effect, 1);
    assert_eq!(state.step, 0);
    assert_eq!(state.timer, 2);
    assert!(flags.effect_owns_channel());
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0x3A),
            NoiseWrite::Envelope(0x40),
            NoiseWrite::Polynomial(0x70),
            NoiseWrite::Control(0xC0),
        ]
    );

    // Frame 1: timer decrements 2 -> 1
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.timer, 1);
    assert_eq!(state.step, 0);
    assert!(out.is_empty());

    // Frame 2: timer expires, advances to step 1 (frame 0x4EE1)
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.step, 1);
    assert_eq!(state.timer, 2);
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0x3A),
            NoiseWrite::Envelope(0x90),
            NoiseWrite::Polynomial(0x50),
            NoiseWrite::Control(0xC0),
        ]
    );

    // Advance through steps 2..7 (each lasting 2 frames)
    for expected_step in 2..=7 {
        // First tick: decrement timer
        out.clear();
        state.tick(&mut flags, &mut out);
        assert_eq!(state.timer, 1);
        assert!(out.is_empty());

        // Second tick: step advance
        out.clear();
        state.tick(&mut flags, &mut out);
        assert_eq!(state.step, expected_step);
        assert_eq!(state.timer, 2);
        assert_eq!(out.len(), 4);
    }

    // Step 7 was reached. Next 2 ticks:
    // Tick 1: timer 2 -> 1
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.timer, 1);

    // Tick 2: step 7 -> 8 => stop!
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.active_effect, 0);
    assert_eq!(state.timer, 0);
    assert_eq!(state.step, 0);
    assert!(!flags.effect_owns_channel());
}

#[test]
fn test_noise_effect2_duration_and_envelope_alternation() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    state.start(2, &mut flags, &mut out);
    assert_eq!(state.active_effect, 2);
    assert_eq!(state.duration, 3);
    assert_eq!(state.timer, 4);
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0x2F),
            NoiseWrite::Envelope(0x49),
            NoiseWrite::Polynomial(0x35),
            NoiseWrite::Control(0xC0),
        ]
    );

    // 3 decrements
    for t in (1..4).rev() {
        out.clear();
        state.tick(&mut flags, &mut out);
        assert_eq!(state.timer, t);
        assert!(out.is_empty());
    }

    // 4th tick: step 1 (odd -> envelope 0x20)
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.step, 1);
    assert_eq!(state.timer, 4);
    assert_eq!(
        out,
        vec![
            NoiseWrite::Envelope(0x20),
            NoiseWrite::Length(0x2F),
            NoiseWrite::Polynomial(0x35),
            NoiseWrite::Control(0xC0),
        ]
    );

    // 3 decrements
    for _ in 0..3 {
        state.tick(&mut flags, &mut out);
    }

    // Next expiry: step 2 (even -> envelope 0x49)
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.step, 2);
    assert_eq!(
        out,
        vec![
            NoiseWrite::Envelope(0x49),
            NoiseWrite::Length(0x2F),
            NoiseWrite::Polynomial(0x34),
            NoiseWrite::Control(0xC0),
        ]
    );
}

#[test]
fn test_noise_effect3_direct_two_frame_burst() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    // Effect 3 does NOT claim flags, sets active = 3 directly
    state.start(3, &mut flags, &mut out);
    assert_eq!(state.active_effect, 3);
    assert!(!flags.effect_owns_channel());
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0x3B),
            NoiseWrite::Envelope(0x80),
            NoiseWrite::Polynomial(0x07),
            NoiseWrite::Control(0xC0),
        ]
    );

    // On tick: immediately clears active = 0 and outputs tick frame
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.active_effect, 0);
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0x00),
            NoiseWrite::Envelope(0x42),
            NoiseWrite::Polynomial(0x02),
            NoiseWrite::Control(0xC0),
        ]
    );
}

#[test]
fn test_noise_effect4_retrigger_guard() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    state.start(4, &mut flags, &mut out);
    assert_eq!(state.active_effect, 4);
    assert_eq!(state.timer, 6);
    assert_eq!(out.len(), 4);

    // Attempting to re-trigger effect 4 while active effect is 4 does nothing
    out.clear();
    state.start(4, &mut flags, &mut out);
    assert!(out.is_empty());
    assert_eq!(state.timer, 6);

    // 5 decrements
    for _ in 0..5 {
        state.tick(&mut flags, &mut out);
    }
    assert_eq!(state.timer, 1);
    assert_eq!(state.active_effect, 4);

    // Expiry: stops
    state.tick(&mut flags, &mut out);
    assert_eq!(state.active_effect, 0);
    assert!(!flags.effect_owns_channel());
}

#[test]
fn test_noise_effect7_retrigger_guard() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    state.start(7, &mut flags, &mut out);
    assert_eq!(state.active_effect, 7);
    assert_eq!(state.timer, 2);

    out.clear();
    state.start(7, &mut flags, &mut out);
    assert!(out.is_empty());
}

#[test]
fn test_noise_effect8_silence_stop() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    state.start(8, &mut flags, &mut out);
    assert_eq!(state.active_effect, 8);
    assert_eq!(state.timer, 3);

    // Advance to step 9
    state.step = 9;
    state.timer = 1;

    out.clear();
    state.tick(&mut flags, &mut out);
    // At step 10, effect 8 calls stop_silence
    assert_eq!(state.active_effect, 0);
    assert!(!flags.effect_owns_channel());
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0),
            NoiseWrite::Envelope(0),
            NoiseWrite::Polynomial(0),
            NoiseWrite::Control(0),
        ]
    );
}

#[test]
fn test_noise_effect9_custom_steps() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    state.start(9, &mut flags, &mut out);
    assert_eq!(state.active_effect, 9);
    assert_eq!(state.timer, 4);

    // Step 4 writes 4 registers (E9_STEP4)
    state.step = 3;
    state.timer = 1;
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.step, 4);
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0x2F),
            NoiseWrite::Envelope(0x40),
            NoiseWrite::Polynomial(0x24),
            NoiseWrite::Control(0x80),
        ]
    );

    // Step 8 writes 4 registers (E9_STEP8)
    state.step = 7;
    state.timer = 1;
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.step, 8);
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0x2F),
            NoiseWrite::Envelope(0x10),
            NoiseWrite::Polynomial(0x24),
            NoiseWrite::Control(0x80),
        ]
    );

    // Step 12 stops
    state.step = 11;
    state.timer = 1;
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.active_effect, 0);
    assert!(!flags.effect_owns_channel());
}

#[test]
fn test_noise_effect10_duration_and_silence_stop() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    state.start(10, &mut flags, &mut out);
    assert_eq!(state.active_effect, 10);
    assert_eq!(state.duration, 5);
    assert_eq!(state.timer, 8);

    // Step 7 calls stop_silence
    state.step = 6;
    state.timer = 1;
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.active_effect, 0);
    assert!(!flags.effect_owns_channel());
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0),
            NoiseWrite::Envelope(0),
            NoiseWrite::Polynomial(0),
            NoiseWrite::Control(0),
        ]
    );
}

#[test]
fn test_noise_effect11_multistep() {
    let mut state = NoiseEffectState::default();
    let mut flags = NoiseMusicFlags::from_bits(0x00);
    let mut out = Vec::new();

    state.start(11, &mut flags, &mut out);
    assert_eq!(state.active_effect, 11);
    assert_eq!(state.timer, 4);

    // Step 1: frame 0x5159 (ticks = 5)
    state.timer = 1;
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.step, 1);
    assert_eq!(state.timer, 5);
    assert_eq!(
        out,
        vec![
            NoiseWrite::Length(0x2F),
            NoiseWrite::Envelope(0x09),
            NoiseWrite::Polynomial(0x45),
            NoiseWrite::Control(0xC0),
        ]
    );

    // Step 8: stop
    state.step = 7;
    state.timer = 1;
    out.clear();
    state.tick(&mut flags, &mut out);
    assert_eq!(state.active_effect, 0);
    assert!(!flags.effect_owns_channel());
}
