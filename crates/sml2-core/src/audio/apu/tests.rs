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
    apu.apply_pulse_write(PulseWrite::Envelope(0xF0));   // Initial volume 15
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
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
        0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10,
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

    assert_ne!(samples1, samples2, "Mutating frequency must alter output waveform");
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
