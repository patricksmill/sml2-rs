use super::*;

#[test]
fn test_apu_initialization_and_registers() {
    let mut apu = Apu::new();
    assert_eq!(apu.nr50, 0x77);
    assert_eq!(apu.nr51, 0xFF);
    assert_eq!(apu.nr52, 0x80);
    assert_eq!(apu.left_volume, 7);
    assert_eq!(apu.right_volume, 7);

    apu.set_nr50(0x32);
    assert_eq!(apu.left_volume, 3);
    assert_eq!(apu.right_volume, 2);

    apu.set_nr51(0x11);
    assert_eq!(apu.nr51, 0x11);

    // Disabling APU disables channels
    apu.ch1.enabled = true;
    apu.set_nr52(0x00);
    assert!(!apu.ch1.enabled);
}

#[test]
fn test_pulse_channel_synthesis() {
    let mut apu = Apu::new();
    // Configure Ch1: 50% duty (index 2), volume 15, trigger with freq 0x600
    apu.apply_pulse_write(PulseWrite::DutyLength(0x80)); // 50% duty
    apu.apply_pulse_write(PulseWrite::Envelope(0xF0)); // Initial volume 15
    apu.apply_pulse_write(PulseWrite::FrequencyLow(0x00));
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86)); // Trigger with high freq 6

    assert!(apu.ch1.enabled);
    assert_eq!(apu.ch1.volume, 15);
    assert_eq!(apu.ch1.duty, 2);

    let mut samples = Vec::new();
    apu.generate_samples(44100, 100, &mut samples);
    assert_eq!(samples.len(), 200); // 100 stereo pairs

    // Buffer must contain non-zero samples within [-1.0, 1.0]
    let non_zero = samples.iter().any(|&s| s.abs() > 0.0);
    assert!(non_zero, "Pulse channel generated only silence");
    for &s in &samples {
        assert!(s >= -1.0 && s <= 1.0, "Sample out of bounds: {s}");
    }
}

#[test]
fn test_wave_channel_synthesis() {
    let mut apu = Apu::new();
    let pattern = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];
    apu.apply_wave_write(WaveWrite::Enable(0x80));
    apu.apply_wave_write(WaveWrite::Level(0x20)); // 100% volume
    apu.apply_wave_write(WaveWrite::WavePattern(pattern));
    apu.apply_wave_write(WaveWrite::FrequencyLow(0xC0));
    apu.apply_wave_write(WaveWrite::FrequencyControl(0x86)); // Trigger

    assert!(apu.ch3.enabled);
    assert_eq!(apu.ch3.volume_code, 1);

    let mut samples = Vec::new();
    apu.generate_samples(44100, 100, &mut samples);
    assert_eq!(samples.len(), 200);

    let non_zero = samples.iter().any(|&s| s.abs() > 0.0);
    assert!(non_zero, "Wave channel generated only silence");
}

#[test]
fn test_noise_channel_synthesis() {
    let mut apu = Apu::new();
    apu.apply_noise_write(NoiseWrite::Envelope(0xF0)); // Volume 15
    apu.apply_noise_write(NoiseWrite::Polynomial(0x30)); // Shift clock 3, divisor 0
    apu.apply_noise_write(NoiseWrite::Control(0x80)); // Trigger

    assert!(apu.ch4.enabled);
    assert_eq!(apu.ch4.volume, 15);

    let mut samples = Vec::new();
    apu.generate_samples(44100, 100, &mut samples);
    assert_eq!(samples.len(), 200);

    let non_zero = samples.iter().any(|&s| s.abs() > 0.0);
    assert!(non_zero, "Noise channel generated only silence");
}

#[test]
fn test_stereo_panning_routing() {
    let mut apu = Apu::new();

    // Pan Ch1 exclusively to Left ($10), Ch2 exclusively to Right ($02)
    apu.set_nr51(0x12);

    apu.apply_pulse_write(PulseWrite::DutyLength(0x80));
    apu.apply_pulse_write(PulseWrite::Envelope(0xF0));
    apu.apply_pulse_write(PulseWrite::FrequencyLow(0x00));
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86)); // Trigger Ch1

    let mut samples = Vec::new();
    apu.generate_samples(44100, 50, &mut samples);

    // Left channel has audio, Right channel has zero (since Ch2 is inactive)
    let left_active = samples.iter().step_by(2).any(|&s| s.abs() > 0.0);
    let right_active = samples.iter().skip(1).step_by(2).any(|&s| s.abs() > 0.0);

    assert!(left_active, "Left channel should contain Ch1 audio");
    assert!(!right_active, "Right channel should be silent without Ch2");
}

#[test]
fn test_ingest_driver_and_sample_generation() {
    let mut driver = SoundDriver::new();
    let mut apu = Apu::new();

    // Start a song on frame 1, tick music and start SFX on frame 2 per ROM $10030
    driver.music_request = 1;
    driver.update_sound();
    driver.sfx_request_0 = 1;
    driver.update_sound();

    // Ingest events into APU
    apu.ingest_driver(&driver);

    let mut buffer = Vec::new();
    // One console frame ≈ 738.35 stereo pairs at 44.1 kHz
    apu.generate_samples(44100, 738, &mut buffer);

    assert_eq!(buffer.len(), 1476);
    let has_signal = buffer.iter().any(|&s| s.abs() > 0.0);
    assert!(has_signal, "Driver playback produced no audible APU signal");
}

#[test]
fn test_apu_sensitivity() {
    let mut apu1 = Apu::new();
    let mut apu2 = Apu::new();

    apu1.apply_pulse_write(PulseWrite::DutyLength(0x80));
    apu1.apply_pulse_write(PulseWrite::Envelope(0xF0));
    apu1.apply_pulse_write(PulseWrite::FrequencyLow(0x50));
    apu1.apply_pulse_write(PulseWrite::FrequencyControl(0x86));

    // Mutated frequency in apu2
    apu2.apply_pulse_write(PulseWrite::DutyLength(0x80));
    apu2.apply_pulse_write(PulseWrite::Envelope(0xF0));
    apu2.apply_pulse_write(PulseWrite::FrequencyLow(0x80));
    apu2.apply_pulse_write(PulseWrite::FrequencyControl(0x86));

    let mut samples1 = Vec::new();
    let mut samples2 = Vec::new();

    apu1.generate_samples(44100, 100, &mut samples1);
    apu2.generate_samples(44100, 100, &mut samples2);

    assert_ne!(
        samples1, samples2,
        "Mutating frequency must alter output waveform"
    );
}

#[test]
fn test_ingest_driver_applies_master_volume() {
    use crate::audio::driver::{SoundDriver, MASTER_VOLUME_LOW_TIME, MASTER_VOLUME_NORMAL};

    let mut driver = SoundDriver::new();
    let mut apu = Apu::new();

    apu.ingest_driver(&driver);
    assert_eq!(
        (apu.nr50, apu.left_volume, apu.right_volume),
        (MASTER_VOLUME_NORMAL, 7, 7)
    );

    // $10044..$1005A rewrites rAUDVOL to $33 while sLowTimeFlag is set; the
    // normal path only runs when no music request is pending.
    driver.low_time_flag = true;
    driver.update_sound();
    apu.ingest_driver(&driver);
    assert_eq!(
        (apu.nr50, apu.left_volume, apu.right_volume),
        (MASTER_VOLUME_LOW_TIME, 3, 3)
    );
}

#[test]
fn test_ch1_sweep_glides_and_overflows() {
    // NR10 = $14: pace 1, add, shift 4.
    let mut apu = Apu::new();
    apu.apply_music_write(MusicWrite::Ch1Sweep(0x14));
    apu.apply_music_write(MusicWrite::Ch1FrequencyLow(0x00));
    apu.apply_music_write(MusicWrite::Ch1FrequencyControl(0x86)); // freq $600
    assert_eq!(apu.ch1.freq, 0x600);
    assert_eq!(apu.ch1.sweep_shadow, 0x600);

    // One 128 Hz sweep clock is four 512 Hz steps of 8192 cycles.
    apu.advance_frame_sequencer(32_768);
    assert_eq!(apu.ch1.freq, 0x660, "$600 + $600>>4");
    apu.advance_frame_sequencer(32_768);
    assert_eq!(apu.ch1.freq, 0x6C6, "$660 + $660>>4");

    // An overflow past 2047 turns the channel off ($10E1D's stop writes 0, so
    // only effects with a live sweep reach this).
    let mut apu = Apu::new();
    apu.apply_music_write(MusicWrite::Ch1Sweep(0x17)); // pace 1, add, shift 7
    apu.apply_music_write(MusicWrite::Ch1FrequencyLow(0xE0));
    apu.apply_music_write(MusicWrite::Ch1FrequencyControl(0x87)); // freq $7E0
    for _ in 0..4 {
        apu.advance_frame_sequencer(32_768);
    }
    assert!(!apu.ch1.enabled, "sweep overflow disables CH1");

    // The music writes NR10 = 0 on every note; that must disable the sweep.
    let mut apu = Apu::new();
    apu.apply_music_write(MusicWrite::Ch1Sweep(0x14));
    apu.apply_music_write(MusicWrite::Ch1Sweep(0));
    apu.apply_music_write(MusicWrite::Ch1FrequencyLow(0x00));
    apu.apply_music_write(MusicWrite::Ch1FrequencyControl(0x86));
    apu.advance_frame_sequencer(32_768);
    assert_eq!(apu.ch1.freq, 0x600, "NR10=0 keeps the note steady");
}

#[test]
fn test_frame_sample_count_matches_the_console_cadence() {
    // 70,224 cycles per frame at 4,194,304 Hz is 738.3533 samples at 44.1 kHz.
    let mut apu = Apu::new();
    let first: Vec<usize> = (0..3).map(|_| apu.frame_sample_count(44100)).collect();
    assert_eq!(first, vec![738, 738, 739]);

    let mut apu = Apu::new();
    let total: usize = (0..100).map(|_| apu.frame_sample_count(44100)).sum();
    assert_eq!(total, 73_835, "100 frames = 73835.33 samples");
    assert!(
        total > 735 * 100,
        "the fixed 735/frame count lost 0.456 % of every second"
    );
}

/// The port now requests the walk (`$5384`/`$53FE`, SFX 4) and map-entry
/// (`$3C1EC`, SFX 22) effects; both must reach the pulse channel, not just
/// the driver's slot bookkeeping.
#[test]
fn walk_and_level_entry_sfx_reach_the_apu() {
    for id in [4u8, 22u8] {
        let mut driver = SoundDriver::new();
        driver.sfx_request_0 = id;
        driver.update_sound();
        assert_eq!(driver.pulse_effect.active_effect, id, "slot 0 started {id}");
        let mut apu = Apu::new();
        apu.ingest_driver(&driver);
        let mut buffer = Vec::new();
        apu.generate_samples(44100, 735, &mut buffer);
        assert!(
            buffer.iter().any(|&s| s.abs() > 0.0),
            "SFX {id} produced no APU signal"
        );
    }
}

/// `$13FE7`: the engine reset writes an all-channel silence frame. A stop-all
/// (`sMusicRequest = $FF`) must therefore leave the APU quiet instead of
/// holding the last note through a map/level transition.
#[test]
fn the_stop_all_silences_a_sounding_channel() {
    let mut driver = SoundDriver::new();
    let mut apu = Apu::new();
    let mut buffer = Vec::new();
    for _ in 0..5 {
        driver.sfx_request_0 = 4;
        driver.update_sound();
        apu.ingest_driver(&driver);
        buffer.clear();
        apu.generate_samples(44_100, 735, &mut buffer);
    }
    assert_eq!(
        driver.pulse_effect.active_effect, 4,
        "the walk SFX is active"
    );
    assert!(
        buffer.iter().any(|&s| s.abs() > 1e-3),
        "SFX 4 sounds before the stop"
    );

    driver.music_request = 0xFF;
    driver.update_sound();
    apu.ingest_driver(&driver);
    assert_eq!(apu.ch1.volume, 0, "NR12 = $08 zeroes the pulse volume");
    assert_eq!(apu.ch2.volume, 0, "NR22 = $08");
    assert_eq!(apu.ch4.volume, 0, "NR42 = $08");
    assert!(!apu.ch3.dac_enabled, "NR30 = 0 turns the wave DAC off");

    buffer.clear();
    apu.generate_samples(44_100, 735, &mut buffer);
    // What remains is only the AC-coupling capacitor's decaying DC offset:
    // no periodic tone, so the magnitude falls monotonically. The first 40
    // samples still carry the band-limited tails of the last steps.
    let left: Vec<f32> = buffer.iter().step_by(2).skip(40).copied().collect();
    for w in left.windows(2) {
        assert!(
            w[1].abs() <= w[0].abs() + 1e-6,
            "a tone survives the stop-all: {w:?}"
        );
    }
}

/// A retrigger keeps the duty position; SML2 retriggers every note and every
/// software-envelope step, so a reset would put a phase jump in each one.
#[test]
fn trigger_keeps_the_duty_position_and_power_off_resets_it() {
    let mut apu = Apu::new();
    apu.apply_pulse_write(PulseWrite::Envelope(0xF0));
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86));
    apu.ch1.duty_step = 5;
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86));
    assert_eq!(apu.ch1.duty_step, 5, "NR14 trigger leaves the duty step");

    apu.set_nr52(0x00);
    assert_eq!(apu.ch1.duty_step, 0, "NR52 power-off resets it");
}

/// `run_cycles` reports every duty step at its cycle offset with the new
/// level, which is what the band-limited mixer places its steps from.
#[test]
fn run_cycles_reports_each_duty_step_at_its_offset() {
    let mut apu = Apu::new();
    apu.apply_pulse_write(PulseWrite::DutyLength(0x80)); // 50%: 1,0,0,0,0,1,1,1
    apu.apply_pulse_write(PulseWrite::Envelope(0xF0));
    apu.apply_pulse_write(PulseWrite::FrequencyLow(0x00));
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x87)); // freq $700
    let period = apu.ch1.period();
    let mut steps = Vec::new();
    apu.ch1
        .run_cycles(period * 8, |at, level| steps.push((at, level)));
    let expected: Vec<(u32, u8)> = [0, 0, 0, 0, 15, 15, 15, 15]
        .iter()
        .enumerate()
        .map(|(i, &level)| (period * (i as u32 + 1), level))
        .collect();
    assert_eq!(steps, expected);
}

/// A pulse far above Nyquist (freq 2047: 131 kHz) comes out as its steady
/// mean. Point sampling instead jumped between silence and full volume,
/// aliasing the tone down into the audible band.
#[test]
fn an_ultrasonic_pulse_averages_instead_of_aliasing() {
    let mut apu = Apu::new();
    apu.apply_pulse_write(PulseWrite::DutyLength(0x80));
    apu.apply_pulse_write(PulseWrite::Envelope(0xF0));
    apu.apply_pulse_write(PulseWrite::FrequencyLow(0xFF));
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x87));
    let mut samples = Vec::new();
    apu.generate_samples(44_100, 400, &mut samples);
    // Skip the note's onset: its step rings for the kernel's width.
    let left: Vec<f32> = samples.iter().step_by(2).skip(64).copied().collect();
    for w in left.windows(2) {
        assert!((w[1] - w[0]).abs() < 1e-3, "sample jump {w:?}");
    }
}

/// gbdev wiki: `out = in - cap; cap = in - out * 0.999958^cycles`. A constant
/// level therefore decays by the charge factor every sample — about 0.996 at
/// 44.1 kHz on the DMG, and a per-sample constant at any host rate.
#[test]
fn high_pass_charge_matches_the_dmg_capacitor_at_each_rate() {
    for rate in [44_100u32, 48_000] {
        let mut apu = Apu::new();
        // Wave DAC on with volume code 0: the mix is a constant 0, so seed the
        // capacitor and watch it discharge.
        apu.apply_wave_write(WaveWrite::Enable(0x80));
        apu.cap_left = 0.5;
        let mut samples = Vec::new();
        apu.generate_samples(rate, 200, &mut samples);
        let left: Vec<f64> = samples.iter().step_by(2).map(|&s| s as f64).collect();
        let expected = 0.999958f64.powf(4_194_304.0 / rate as f64);
        let measured = (left[199] / left[0]).powf(1.0 / 199.0);
        assert!(
            (measured - expected).abs() < 1e-5,
            "{rate} Hz: charge {measured} vs {expected}"
        );
    }
}

/// With every DAC off the mixer is disconnected: silence, not the decaying
/// capacitor, and the charge is held for when a DAC powers back on.
#[test]
fn all_dacs_off_disconnects_the_high_pass_output() {
    let mut apu = Apu::new();
    apu.cap_left = 0.5;
    let mut samples = Vec::new();
    apu.generate_samples(44_100, 50, &mut samples);
    assert!(samples.iter().all(|&s| s == 0.0));
    assert_eq!(apu.cap_left, 0.5);

    // NRx2 = $08 (volume 0, increase) powers the DAC even though it is silent.
    apu.apply_pulse_write(PulseWrite::Envelope(0x08));
    assert!(apu.ch1.dac_on());
    samples.clear();
    apu.generate_samples(44_100, 1, &mut samples);
    assert!(samples[0] < -0.4, "reconnected output {}", samples[0]);
}

/// Pan Docs: a pulse/noise DAC is on while `NRx2 & $F8 != 0`. Writing a value
/// that clears those bits turns the channel off, and a trigger cannot start
/// it again until the DAC is powered. The ROM's pulse silence frame (`$106A2`)
/// and a noise stop both write NRx2 = 0.
#[test]
fn nrx2_dac_off_disables_the_channel_until_a_powered_trigger() {
    let mut apu = Apu::new();
    apu.apply_pulse_write(PulseWrite::Envelope(0xF0));
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86));
    assert!(apu.ch1.enabled);

    apu.apply_pulse_write(PulseWrite::Envelope(0x00));
    assert!(!apu.ch1.enabled, "NR12 = 0 powers the DAC off");
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86));
    assert!(
        !apu.ch1.enabled,
        "a trigger with the DAC off stays disabled"
    );

    // $08 (volume 0, increase) keeps the DAC powered, so the trigger holds.
    apu.apply_pulse_write(PulseWrite::Envelope(0x08));
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86));
    assert!(apu.ch1.enabled, "NR12 = $08 is silent but powered");

    apu.apply_noise_write(NoiseWrite::Envelope(0xA0));
    apu.apply_noise_write(NoiseWrite::Control(0x80));
    assert!(apu.ch4.enabled);
    apu.apply_noise_write(NoiseWrite::Envelope(0x00));
    assert!(!apu.ch4.enabled, "NR42 = 0 powers the DAC off");
    apu.apply_noise_write(NoiseWrite::Control(0x80));
    assert!(!apu.ch4.enabled);
}

/// Pan Docs: a CH3 trigger resets the position without refilling the sample
/// buffer, so the stale nibble plays first, then nibble 1; nibble 0 only
/// plays once the wave loops.
#[test]
fn wave_trigger_plays_the_stale_buffer_then_nibble_one() {
    let mut apu = Apu::new();
    let mut pattern = [0u8; 16];
    pattern[0] = 0x9A; // nibble 0 = 9, nibble 1 = A
    apu.apply_wave_write(WaveWrite::Enable(0x80));
    apu.apply_wave_write(WaveWrite::Level(0x20));
    apu.apply_wave_write(WaveWrite::WavePattern(pattern));
    apu.ch3.sample_buffer = 0x3;
    apu.apply_wave_write(WaveWrite::FrequencyControl(0x87)); // freq $700
    let period = apu.ch3.period();

    assert_eq!(apu.ch3.sample(), 0x3, "stale buffer first");
    apu.ch3.advance_cycles(period);
    assert_eq!((apu.ch3.sample_step, apu.ch3.sample()), (1, 0xA));
    apu.ch3.advance_cycles(period * 31);
    assert_eq!(
        (apu.ch3.sample_step, apu.ch3.sample()),
        (0, 0x9),
        "after the loop"
    );
}

/// Channel timers run at volume 0 (and wave volume code 0); only the output
/// is silent. Freezing them shifted the phase a later note started from.
#[test]
fn channel_timers_run_while_silent() {
    let mut apu = Apu::new();
    apu.apply_pulse_write(PulseWrite::Envelope(0x08)); // powered, volume 0
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x87));
    apu.ch1.advance_cycles(apu.ch1.period() * 3);
    assert_eq!(apu.ch1.duty_step, 3);

    apu.apply_noise_write(NoiseWrite::Envelope(0x08));
    apu.apply_noise_write(NoiseWrite::Control(0x80));
    apu.ch4.advance_cycles(apu.ch4.period());
    assert_ne!(apu.ch4.lfsr, 0x7FFF, "LFSR clocks at volume 0");

    apu.apply_wave_write(WaveWrite::Enable(0x80));
    apu.apply_wave_write(WaveWrite::Level(0x00));
    apu.apply_wave_write(WaveWrite::FrequencyControl(0x87));
    apu.ch3.advance_cycles(apu.ch3.period() * 5);
    assert_eq!(apu.ch3.sample_step, 5);
    assert_eq!(apu.ch3.sample(), 0, "volume code 0 still mutes");
}

/// Goertzel power of `x` at `freq` Hz.
fn tone_power(x: &[f32], freq: f64, rate: f64) -> f64 {
    let w = 2.0 * std::f64::consts::PI * freq / rate;
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &v in x {
        let s0 = v as f64 + 2.0 * w.cos() * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2
}

/// An 8192 Hz square (freq $7F0) has odd harmonics at 24576, 40960, 57344 Hz
/// ... which a 44.1 kHz stream folds to 19524, 3140 and 13244 Hz. The
/// per-sample box filter left those at -14, -36 and -30 dB against the
/// fundamental; band-limited steps put them below -78 dB.
#[test]
fn a_bright_square_has_no_audible_aliases() {
    let mut apu = Apu::new();
    apu.apply_pulse_write(PulseWrite::DutyLength(0x80));
    apu.apply_pulse_write(PulseWrite::Envelope(0xF0));
    apu.apply_pulse_write(PulseWrite::FrequencyLow(0xF0));
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x07)); // freq $7F0, no length
    apu.apply_pulse_write(PulseWrite::FrequencyControl(0x87));
    let mut samples = Vec::new();
    apu.generate_samples(44_100, 44_100, &mut samples);
    let left: Vec<f32> = samples.iter().step_by(2).skip(4_100).copied().collect();
    let fundamental = tone_power(&left, 8192.0, 44_100.0);
    for alias in [3140.0, 13244.0, 19524.0] {
        let db = 10.0 * (tone_power(&left, alias, 44_100.0) / fundamental).log10();
        assert!(db < -60.0, "{alias} Hz alias at {db:.1} dB");
    }
}

/// Every step integrates back to exactly its delta, whatever its sub-sample
/// time and however the block is split, so the output cannot drift.
#[test]
fn band_limited_steps_integrate_exactly() {
    use crate::audio::blip::{StereoBlip, UNIT, WIDTH};
    let mut synth = StereoBlip::default();
    let mut levels = Vec::new();
    for time in [0.0, 0.3, 7.77, 15.999, -0.9] {
        synth.begin(16);
        synth.add_step(time, [3, -5]);
        synth.read(16, |l| levels.push(l));
    }
    synth.begin(WIDTH + 2);
    synth.read(WIDTH + 2, |l| levels.push(l));
    assert_eq!(*levels.last().unwrap(), [5 * 3 * UNIT, 5 * -5 * UNIT]);
}

/// The soft clip is the identity up to its knee, continuous with slope 1
/// there, and never exceeds full scale.
#[test]
fn soft_clip_is_transparent_below_the_knee_and_bounded_above() {
    for x in [0.0f32, 0.3, -0.5, 0.8, -0.8] {
        assert_eq!(soft_clip(x), x);
    }
    let just_above = soft_clip(0.801);
    assert!((just_above - 0.801).abs() < 1e-5, "slope 1 at the knee");
    for x in [0.9f32, 1.2, 3.0, -2.0] {
        let y = soft_clip(x);
        assert!(y.abs() <= 1.0 && y.abs() > 0.8 && y.signum() == x.signum());
    }
    assert!(soft_clip(1.2) > soft_clip(0.9), "monotonic");
}

/// `output_gain` scales the high-passed mix linearly below the knee.
#[test]
fn output_gain_scales_the_mix() {
    let render = |gain: f32| {
        let mut apu = Apu::new();
        apu.output_gain = gain;
        apu.apply_pulse_write(PulseWrite::DutyLength(0x80));
        apu.apply_pulse_write(PulseWrite::Envelope(0x80));
        apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86));
        let mut samples = Vec::new();
        apu.generate_samples(44_100, 400, &mut samples);
        samples
    };
    let (unity, default) = (render(1.0), render(OUTPUT_GAIN));
    for (u, d) in unity.iter().zip(&default) {
        assert!((u * OUTPUT_GAIN - d).abs() < 1e-6);
    }
}

#[test]
fn channel_weights_scale_one_channel_and_leave_unity_exact() {
    let run = |weights: [u16; 4]| {
        let mut apu = Apu::new();
        apu.channel_weights = weights;
        apu.apply_pulse_write(PulseWrite::DutyLength(0x80));
        apu.apply_pulse_write(PulseWrite::Envelope(0xF0));
        apu.apply_pulse_write(PulseWrite::FrequencyControl(0x86));
        apu.apply_noise_write(NoiseWrite::Envelope(0xF0));
        apu.apply_noise_write(NoiseWrite::Polynomial(0x30));
        apu.apply_noise_write(NoiseWrite::Control(0x80));
        let mut samples = Vec::new();
        apu.generate_samples(44_100, 735, &mut samples);
        samples
    };
    let unit = CHANNEL_WEIGHT_UNIT;
    let both = run([unit; 4]);
    let pulse_only = run([unit, unit, unit, 0]);
    let noise_only = run([0, unit, unit, unit]);
    assert_ne!(both, pulse_only);
    assert!(pulse_only.iter().any(|s| s.abs() > 0.0));
    assert!(noise_only.iter().any(|s| s.abs() > 0.0));
    // Below the soft-clip knee the mix is linear: the halves sum to the whole.
    for ((&b, &p), &n) in both.iter().zip(&pulse_only).zip(&noise_only) {
        if b.abs() < 0.5 {
            assert!((b - (p + n)).abs() < 1e-4, "{b} vs {p} + {n}");
        }
    }
    assert!(run([0; 4]).iter().all(|&s| s == 0.0));
}
