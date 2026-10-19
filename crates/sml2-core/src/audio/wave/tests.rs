use super::*;

#[test]
fn test_wave_flags_and_state() {
    let mut flags = WaveMusicFlags::default();
    assert!(!flags.effect_owns_channel());
    assert_eq!(flags.bits(), 0);

    flags.claim_for_effect();
    assert!(flags.effect_owns_channel());
    assert_eq!(flags.bits(), 0x80);

    flags.release_effect();
    assert!(!flags.effect_owns_channel());
    assert_eq!(flags.bits(), 0x00);

    let from_raw = WaveMusicFlags::from_bits(0xA4);
    assert!(from_raw.effect_owns_channel());
    assert_eq!(from_raw.bits(), 0xA4);
}

#[test]
fn test_wave_effect1_lifecycle() {
    let mut state = WaveEffectState::default();
    let mut music = WaveMusicState {
        flags: WaveMusicFlags::default(),
        wave_pattern: [0x55; 16],
    };
    let mut writes = Vec::new();

    state.start(1, &mut music, &mut writes);
    assert_eq!(state.active_effect, 1);
    assert_eq!(state.duration, 2);
    assert_eq!(state.timer, 2);
    assert!(music.flags.effect_owns_channel());
    assert_eq!(writes[0], WaveWrite::Enable(0));
    assert_eq!(writes[1], WaveWrite::WavePattern(PATTERN_5BD6));
    assert_eq!(writes[2], WaveWrite::Enable(0));
    assert_eq!(writes[3], WaveWrite::Enable(0x80));
    assert_eq!(writes[4], WaveWrite::Length(0xE2));
    assert_eq!(writes[5], WaveWrite::Level(0x20));
    assert_eq!(writes[6], WaveWrite::FrequencyLow(0x80));
    assert_eq!(writes[7], WaveWrite::FrequencyControl(0xC6));

    // First step occurs when timer decrements to 0
    writes.clear();
    state.tick(&mut music, &mut writes); // timer 2 -> 1
    assert!(writes.is_empty());
    assert_eq!(state.timer, 1);

    state.tick(&mut music, &mut writes); // timer 1 -> 0, step 0 -> 1
    assert_eq!(state.step, 1);
    assert_eq!(writes, vec![
        WaveWrite::FrequencyLow(0x98),
        WaveWrite::FrequencyControl(0x86),
    ]);
    assert_eq!(state.timer, 2);
}

#[test]
fn test_wave_effect2_lifecycle() {
    let mut state = WaveEffectState::default();
    let mut music = WaveMusicState::default();
    let mut writes = Vec::new();

    state.start(2, &mut music, &mut writes);
    assert_eq!(state.active_effect, 2);
    assert_eq!(writes[1], WaveWrite::WavePattern(PATTERN_5BE6));
    assert_eq!(writes[6], WaveWrite::FrequencyLow(0x8A));
    assert_eq!(writes[7], WaveWrite::FrequencyControl(0xC7));

    // Tick through until stop at step 18
    for _ in 0..100 {
        if state.active_effect == 0 {
            break;
        }
        writes.clear();
        state.tick(&mut music, &mut writes);
    }
    assert_eq!(state.active_effect, 0);
    assert!(!music.flags.effect_owns_channel());
}

#[test]
fn test_wave_effect6_variant_toggle_and_retrigger_guard() {
    let mut state = WaveEffectState::default();
    let mut music = WaveMusicState::default();
    let mut writes = Vec::new();

    // First play: starts variant 1
    state.start(6, &mut music, &mut writes);
    assert_eq!(state.active_effect, 6);
    assert_eq!(state.variant, 1);
    assert_eq!(writes[5], WaveWrite::Level(0x40)); // VAR1_START level 0x40

    // Retrigger while active is ignored
    writes.clear();
    state.start(6, &mut music, &mut writes);
    assert!(writes.is_empty());
    assert_eq!(state.variant, 1);

    // Tick to completion (3 steps, each 3 ticks = 9 ticks)
    for _ in 0..9 {
        state.tick(&mut music, &mut writes);
    }
    assert_eq!(state.active_effect, 0);

    // Second play: starts variant 0
    writes.clear();
    state.start(6, &mut music, &mut writes);
    assert_eq!(state.active_effect, 6);
    assert_eq!(state.variant, 0);
    assert_eq!(writes[5], WaveWrite::Level(0x20)); // VAR0_START level 0x20
}

#[test]
fn test_wave_effect20_busy_interruption() {
    let mut state = WaveEffectState::default();
    let mut music = WaveMusicState::default();
    let mut writes = Vec::new();

    // Start effect 20 sets busy = 1
    state.start(20, &mut music, &mut writes);
    assert_eq!(state.active_effect, 20);
    assert_eq!(state.busy, 1);
    assert_eq!(state.duration, 7);

    // Other effects cannot start while busy
    writes.clear();
    state.start(1, &mut music, &mut writes);
    assert_eq!(state.active_effect, 20); // Still 20!
    assert!(writes.is_empty());

    // Effect 20 itself CAN restart while busy
    state.start(20, &mut music, &mut writes);
    assert_eq!(state.active_effect, 20);
    assert_eq!(state.duration, 7);
    assert!(!writes.is_empty());
}

#[test]
fn test_wave_effect24_gating() {
    let mut state = WaveEffectState::default();
    let mut music = WaveMusicState::default();
    let mut writes = Vec::new();

    // When channel active, Effect 24 does nothing
    state.active_effect = 2;
    state.start(24, &mut music, &mut writes);
    assert_eq!(state.active_effect, 2);
    assert!(writes.is_empty());

    // When inactive, Effect 24 dispatches Effect 21 with active_effect = 24
    state.active_effect = 0;
    state.start(24, &mut music, &mut writes);
    assert_eq!(state.active_effect, 24);
    assert_eq!(writes[1], WaveWrite::WavePattern(PATTERN_5B96));
    assert_eq!(writes[5], WaveWrite::Level(0x40));
    assert_eq!(writes[6], WaveWrite::FrequencyLow(0xC8));
    assert_eq!(writes[7], WaveWrite::FrequencyControl(0xC7));
}

#[test]
fn test_wave_pattern_restored_on_stop() {
    let mut state = WaveEffectState::default();
    let custom_pattern = [
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
        0x0F, 0xED, 0xCB, 0xA9, 0x87, 0x65, 0x43, 0x21,
    ];
    let mut music = WaveMusicState {
        flags: WaveMusicFlags::default(),
        wave_pattern: custom_pattern,
    };
    let mut writes = Vec::new();

    // Effect 14 runs for 6 ticks then stops
    state.start(14, &mut music, &mut writes);
    assert_eq!(state.active_effect, 14);

    for _ in 0..6 {
        writes.clear();
        state.tick(&mut music, &mut writes);
    }
    assert_eq!(state.active_effect, 0);
    assert!(!music.flags.effect_owns_channel());

    // Stop must write Enable(0) then WavePattern(custom_pattern)
    assert_eq!(writes[0], WaveWrite::Enable(0));
    assert_eq!(writes[1], WaveWrite::WavePattern(custom_pattern));
}

#[test]
fn test_all_24_wave_effects_start_and_progress() {
    for id in 1..=24 {
        let mut state = WaveEffectState::default();
        let mut music = WaveMusicState::default();
        let mut writes = Vec::new();

        state.start(id, &mut music, &mut writes);
        if id == 19 {
            // Effect 19 is a no-op
            assert_eq!(state.active_effect, 0);
            assert!(writes.is_empty());
            continue;
        }

        let expected_active = if id == 24 { 24 } else { id };
        assert_eq!(state.active_effect, expected_active, "Effect {id} failed to activate");
        assert!(music.flags.effect_owns_channel(), "Effect {id} failed to claim channel");
        assert!(!writes.is_empty(), "Effect {id} produced no writes on start");

        // Tick once
        writes.clear();
        state.tick(&mut music, &mut writes);
    }
}

#[test]
fn test_wave_slides_exact_arithmetic() {
    let mut state = WaveEffectState::default();
    let mut music = WaveMusicState::default();
    let mut writes = Vec::new();

    // Effect 4: start freq is 0x0100, delta is +80
    state.start(4, &mut music, &mut writes);
    assert_eq!(state.freq_lo, 0x00);
    assert_eq!(state.freq_hi, 0x01);

    writes.clear();
    state.tick(&mut music, &mut writes);
    // 0x0100 + 80 = 0x0150
    assert_eq!(state.freq_lo, 0x50);
    assert_eq!(state.freq_hi, 0x01);
    assert_eq!(writes, vec![
        WaveWrite::FrequencyLow(0x50),
        WaveWrite::FrequencyControl(0x01),
    ]);

    // Effect 16: start freq is 0x0500, delta is -160 (0xFF60)
    writes.clear();
    state.start(16, &mut music, &mut writes);
    assert_eq!(state.freq_lo, 0x00);
    assert_eq!(state.freq_hi, 0x05);

    writes.clear();
    state.tick(&mut music, &mut writes);
    // 0x0500 - 160 = 0x0460
    assert_eq!(state.freq_lo, 0x60);
    assert_eq!(state.freq_hi, 0x04);
    assert_eq!(writes, vec![
        WaveWrite::FrequencyLow(0x60),
        WaveWrite::FrequencyControl(0x04),
    ]);
}

#[test]
fn test_wave_sensitivity() {
    // Verifies that mutating frequency/delta values directly affects output
    let mut state = WaveEffectState::default();
    let mut music = WaveMusicState::default();
    let mut writes = Vec::new();

    state.start(17, &mut music, &mut writes);
    assert_eq!(state.freq_lo, 0xC0);
    assert_eq!(state.freq_hi, 0x06);

    writes.clear();
    state.tick(&mut music, &mut writes);
    // 0x06C0 - 64 = 0x0680
    assert_eq!(state.freq_lo, 0x80);
    assert_eq!(state.freq_hi, 0x06);

    // If delta were -65 instead of -64, freq_lo would be 0x7F, not 0x80
    let mutated_delta: i16 = -65;
    let freq = u16::from_le_bytes([0xC0, 0x06]);
    let mutated_result = freq.wrapping_add(mutated_delta as u16);
    assert_ne!(mutated_result, 0x0680);
    assert_eq!(mutated_result, 0x067F);
}

#[test]
fn test_wave_driver_dispatch_and_execution() {
    use crate::audio::driver::SoundDriver;

    let mut driver = SoundDriver::new();
    driver.sfx_request_1 = 14;
    driver.update_sound();

    assert_eq!(driver.sfx_request_1, 0); // Consumed
    assert_eq!(driver.wave_slot.active_effect, 14);
    assert!(driver.wave_music.flags.effect_owns_channel());
    assert_eq!(driver.wave_writes.len(), 8); // Enable(0), WavePattern(16), Enable(0), Enable, Len, Lvl, Lo, Hi

    // Tick 6 times through update_sound until effect 14 stops
    for _ in 0..6 {
        driver.wave_writes.clear();
        driver.update_sound();
    }
    assert_eq!(driver.wave_slot.active_effect, 0);
    assert!(!driver.wave_music.flags.effect_owns_channel());
    assert_eq!(driver.wave_writes, vec![
        WaveWrite::Enable(0),
        WaveWrite::WavePattern([0; 16]),
    ]);
}

