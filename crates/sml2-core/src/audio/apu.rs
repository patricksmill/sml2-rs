//! 4-Channel Game Boy Audio Processing Unit (APU) synthesizer.
//!
//! Synthesizes stereo PCM floating-point audio from native sound driver events.
//! Channels:
//! - Channel 1: Pulse 1 with sweep, duty, envelope, frequency (NR10..NR14)
//! - Channel 2: Pulse 2 with duty, envelope, frequency (NR21..NR24)
//! - Channel 3: Custom 32-nibble 4-bit Wave channel with volume shift (NR30..NR34)
//! - Channel 4: Noise channel with linear feedback shift register (NR41..NR44)

#[cfg(test)]
mod tests;

use crate::audio::driver::SoundDriver;
use crate::audio::music::MusicWrite;
use crate::audio::noise::NoiseWrite;
use crate::audio::pulse::PulseWrite;
use crate::audio::wave::WaveWrite;

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25.0%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50.0%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75.0%
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PulseChannel {
    pub enabled: bool,
    pub duty: u8,
    pub volume: u8,
    pub env_initial: u8,
    pub env_direction: bool,
    pub env_period: u8,
    pub env_timer: u8,
    pub length_timer: u8,
    pub length_enabled: bool,
    pub freq: u16,
    pub duty_step: u8,
    pub timer: u32,
    /// NR10: sweep pace (bits 6-4), direction (bit 3, 1 = decreasing) and shift.
    pub sweep_pace: u8,
    pub sweep_down: bool,
    pub sweep_shift: u8,
    pub sweep_timer: u8,
    /// The sweep unit's shadow frequency, reloaded on every trigger.
    pub sweep_shadow: u16,
}

impl PulseChannel {
    /// NR10 write ($FF10).
    pub fn set_sweep(&mut self, val: u8) {
        self.sweep_pace = (val >> 4) & 0x07;
        self.sweep_down = (val & 0x08) != 0;
        self.sweep_shift = val & 0x07;
    }

    /// `shadow +/- (shadow >> shift)`.
    fn sweep_calc(&self) -> u16 {
        let delta = self.sweep_shadow >> self.sweep_shift;
        if self.sweep_down {
            self.sweep_shadow.wrapping_sub(delta)
        } else {
            self.sweep_shadow.wrapping_add(delta)
        }
    }

    /// 128 Hz sweep clock: a pace of 0 disables the unit; an overflow past 2047
    /// turns the channel off.
    pub fn clock_sweep(&mut self) {
        if self.sweep_pace == 0 {
            return;
        }
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }
        if self.sweep_timer != 0 {
            return;
        }
        self.sweep_timer = self.sweep_pace;
        if self.sweep_shift == 0 {
            return;
        }
        let next = self.sweep_calc();
        if next > 2047 {
            self.enabled = false;
            return;
        }
        self.sweep_shadow = next;
        self.freq = next;
        if self.sweep_calc() > 2047 {
            self.enabled = false;
        }
    }

    pub fn trigger(&mut self) {
        self.enabled = true;
        self.volume = self.env_initial;
        self.env_timer = self.env_period;
        // A trigger reloads the frequency timer but leaves the duty position
        // where it was; only APU power-off resets it (Pan Docs, Audio details).
        self.timer = self.period();
        if self.length_timer == 0 {
            self.length_timer = 64;
        }
        // Trigger reloads the sweep's shadow register and runs the overflow
        // check once ($FF10 hardware behaviour).
        self.sweep_shadow = self.freq & 0x07FF;
        self.sweep_timer = if self.sweep_pace == 0 {
            8
        } else {
            self.sweep_pace
        };
        if self.sweep_shift != 0 && self.sweep_calc() > 2047 {
            self.enabled = false;
        }
    }

    #[inline]
    pub fn period(&self) -> u32 {
        (2048 - (self.freq as u32).min(2047)) * 4
    }

    pub fn advance_cycles(&mut self, mut cycles: u32) {
        if !self.enabled || self.volume == 0 {
            return;
        }
        let period = self.period().max(1);
        while cycles >= self.timer {
            cycles -= self.timer;
            self.duty_step = (self.duty_step + 1) & 7;
            self.timer = period;
        }
        self.timer -= cycles;
    }

    pub fn sample(&self) -> u8 {
        if !self.enabled || self.volume == 0 {
            return 0;
        }
        let duty_pattern = DUTY_TABLE[(self.duty as usize) & 3];
        if duty_pattern[self.duty_step as usize] != 0 {
            self.volume
        } else {
            0
        }
    }

    pub fn clock(&mut self) -> u8 {
        self.advance_cycles(1);
        self.sample()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WaveChannel {
    pub enabled: bool,
    pub dac_enabled: bool,
    pub volume_code: u8,
    pub length_timer: u16,
    pub length_enabled: bool,
    pub freq: u16,
    pub wave_ram: [u8; 16],
    pub sample_step: u8,
    pub timer: u32,
}

impl WaveChannel {
    pub fn trigger(&mut self) {
        self.enabled = self.dac_enabled;
        self.sample_step = 0;
        self.timer = self.period();
        if self.length_timer == 0 {
            self.length_timer = 256;
        }
    }

    #[inline]
    pub fn period(&self) -> u32 {
        (2048 - (self.freq as u32).min(2047)) * 2
    }

    pub fn advance_cycles(&mut self, mut cycles: u32) {
        if !self.enabled || !self.dac_enabled || self.volume_code == 0 {
            return;
        }
        let period = self.period().max(1);
        while cycles >= self.timer {
            cycles -= self.timer;
            self.sample_step = (self.sample_step + 1) & 31;
            self.timer = period;
        }
        self.timer -= cycles;
    }

    pub fn sample(&self) -> u8 {
        if !self.enabled || !self.dac_enabled || self.volume_code == 0 {
            return 0;
        }
        let byte = self.wave_ram[(self.sample_step as usize) / 2];
        let raw = if self.sample_step & 1 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        };

        match self.volume_code {
            1 => raw,      // 100%
            2 => raw >> 1, // 50%
            3 => raw >> 2, // 25%
            _ => 0,        // Mute
        }
    }

    pub fn clock(&mut self) -> u8 {
        self.advance_cycles(1);
        self.sample()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoiseChannel {
    pub enabled: bool,
    pub volume: u8,
    pub env_initial: u8,
    pub env_direction: bool,
    pub env_period: u8,
    pub env_timer: u8,
    pub length_timer: u8,
    pub length_enabled: bool,
    pub shift_clock: u8,
    pub width_mode_7bit: bool,
    pub divisor_code: u8,
    pub lfsr: u16,
    pub timer: u32,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 0,
            env_initial: 0,
            env_direction: false,
            env_period: 0,
            env_timer: 0,
            length_timer: 0,
            length_enabled: false,
            shift_clock: 0,
            width_mode_7bit: false,
            divisor_code: 0,
            lfsr: 0x7FFF,
            timer: 0,
        }
    }
}

impl NoiseChannel {
    pub fn trigger(&mut self) {
        self.enabled = true;
        self.volume = self.env_initial;
        self.env_timer = self.env_period;
        self.lfsr = 0x7FFF;
        self.timer = self.period();
        if self.length_timer == 0 {
            self.length_timer = 64;
        }
    }

    #[inline]
    pub fn period(&self) -> u32 {
        let divisor: u32 = match self.divisor_code {
            0 => 8,
            n => (n as u32) * 16,
        };
        divisor << (self.shift_clock as u32)
    }

    pub fn advance_cycles(&mut self, mut cycles: u32) {
        if !self.enabled || self.volume == 0 {
            return;
        }
        let period = self.period().max(1);
        while cycles > 0 {
            if cycles < self.timer {
                self.timer -= cycles;
                break;
            }
            cycles -= self.timer;
            self.timer = period;
            let b0 = self.lfsr & 1;
            let b1 = (self.lfsr >> 1) & 1;
            let feedback = b0 ^ b1;
            self.lfsr = (self.lfsr >> 1) | (feedback << 14);
            if self.width_mode_7bit {
                self.lfsr = (self.lfsr & !(1 << 6)) | (feedback << 6);
            }
        }
    }

    pub fn sample(&self) -> u8 {
        if !self.enabled || self.volume == 0 {
            return 0;
        }
        if (self.lfsr & 1) == 0 {
            self.volume
        } else {
            0
        }
    }

    pub fn clock(&mut self) -> u8 {
        self.advance_cycles(1);
        self.sample()
    }
}

/// Complete 4-channel Game Boy APU synthesizer.
#[derive(Debug, Clone, PartialEq)]
pub struct Apu {
    pub ch1: PulseChannel,
    pub ch2: PulseChannel,
    pub ch3: WaveChannel,
    pub ch4: NoiseChannel,
    pub nr50: u8, // Master volume / routing
    pub nr51: u8, // Stereo panning
    pub nr52: u8, // Sound enable
    pub left_volume: u8,
    pub right_volume: u8,
    pub cycle_acc: u32,
    pub frame_seq_cycle: u32,
    pub frame_seq_step: u8,
    /// Remainder of `70224 * sample_rate` from the last `frame_sample_count`,
    /// so a frame's sample count averages 738.3533 at 44.1 kHz and not 735.
    pub sample_frame_acc: u64,
    pub cap_left: f32, // High-pass DC-blocking filter state
    pub cap_right: f32,
}

impl Default for Apu {
    fn default() -> Self {
        Self {
            ch1: PulseChannel::default(),
            ch2: PulseChannel::default(),
            ch3: WaveChannel::default(),
            ch4: NoiseChannel::default(),
            nr50: 0x77, // Normal master volume
            nr51: 0xFF, // All channels panned to left & right
            nr52: 0x80, // APU powered on
            left_volume: 7,
            right_volume: 7,
            cycle_acc: 0,
            frame_seq_cycle: 0,
            frame_seq_step: 0,
            sample_frame_acc: 0,
            cap_left: 0.0,
            cap_right: 0.0,
        }
    }
}

impl Apu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all sound channels and control registers.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Set master volume / routing byte ($FF24 / NR50).
    pub fn set_nr50(&mut self, val: u8) {
        self.nr50 = val;
        self.left_volume = (val >> 4) & 7;
        self.right_volume = val & 7;
    }

    /// Set stereo sound selection byte ($FF25 / NR51).
    pub fn set_nr51(&mut self, val: u8) {
        self.nr51 = val;
    }

    /// Set sound enable register ($FF26 / NR52).
    pub fn set_nr52(&mut self, val: u8) {
        self.nr52 = val & 0x80;
        if self.nr52 == 0 {
            self.ch1.enabled = false;
            self.ch2.enabled = false;
            self.ch1.duty_step = 0;
            self.ch2.duty_step = 0;
            self.ch3.enabled = false;
            self.ch4.enabled = false;
        }
    }

    /// Ingest and apply all pending register writes from a SoundDriver instance.
    pub fn ingest_driver(&mut self, driver: &SoundDriver) {
        // `$10044..$1005A`: `_UpdateSound` rewrites rAUDVOL every frame — $77
        // normally, $33 while `sLowTimeFlag` is set.
        self.set_nr50(driver.master_volume);
        for &w in &driver.pulse_writes {
            self.apply_pulse_write(w);
        }
        for &w in &driver.wave_writes {
            self.apply_wave_write(w);
        }
        for &w in &driver.noise_writes {
            self.apply_noise_write(w);
        }
        for &w in &driver.music_writes {
            self.apply_music_write(w);
        }
    }

    pub fn apply_pulse_write(&mut self, write: PulseWrite) {
        match write {
            PulseWrite::Sweep(val) => self.ch1.set_sweep(val),
            PulseWrite::DutyLength(val) => {
                self.ch1.duty = val >> 6;
                self.ch1.length_timer = 64 - (val & 0x3F);
            }
            PulseWrite::Envelope(val) => {
                self.ch1.env_initial = val >> 4;
                self.ch1.env_direction = (val & 0x08) != 0;
                self.ch1.env_period = val & 0x07;
                self.ch1.env_timer = self.ch1.env_period;
                self.ch1.volume = self.ch1.env_initial;
            }
            PulseWrite::FrequencyLow(lo) => {
                self.ch1.freq = (self.ch1.freq & 0x0700) | (lo as u16);
            }
            PulseWrite::FrequencyControl(ctl) => {
                self.ch1.length_enabled = (ctl & 0x40) != 0;
                self.ch1.freq = (self.ch1.freq & 0x00FF) | (((ctl & 0x07) as u16) << 8);
                if (ctl & 0x80) != 0 {
                    self.ch1.trigger();
                }
            }
            PulseWrite::Ch2DutyLength(val) => {
                self.ch2.duty = val >> 6;
                self.ch2.length_timer = 64 - (val & 0x3F);
            }
            PulseWrite::Ch2Envelope(val) => {
                self.ch2.env_initial = val >> 4;
                self.ch2.env_direction = (val & 0x08) != 0;
                self.ch2.env_period = val & 0x07;
                self.ch2.env_timer = self.ch2.env_period;
                self.ch2.volume = self.ch2.env_initial;
            }
            PulseWrite::Ch2FrequencyLow(lo) => {
                self.ch2.freq = (self.ch2.freq & 0x0700) | (lo as u16);
            }
            PulseWrite::Ch2FrequencyControl(ctl) => {
                self.ch2.length_enabled = (ctl & 0x40) != 0;
                self.ch2.freq = (self.ch2.freq & 0x00FF) | (((ctl & 0x07) as u16) << 8);
                if (ctl & 0x80) != 0 {
                    self.ch2.trigger();
                }
            }
        }
    }

    pub fn apply_wave_write(&mut self, write: WaveWrite) {
        match write {
            WaveWrite::Enable(val) => {
                self.ch3.dac_enabled = (val & 0x80) != 0;
                if !self.ch3.dac_enabled {
                    self.ch3.enabled = false;
                }
            }
            WaveWrite::Length(val) => {
                self.ch3.length_timer = 256 - (val as u16);
            }
            WaveWrite::Level(val) => {
                self.ch3.volume_code = (val >> 5) & 3;
            }
            WaveWrite::FrequencyLow(lo) => {
                self.ch3.freq = (self.ch3.freq & 0x0700) | (lo as u16);
            }
            WaveWrite::FrequencyControl(ctl) => {
                self.ch3.length_enabled = (ctl & 0x40) != 0;
                self.ch3.freq = (self.ch3.freq & 0x00FF) | (((ctl & 0x07) as u16) << 8);
                if (ctl & 0x80) != 0 {
                    self.ch3.trigger();
                }
            }
            WaveWrite::WavePattern(pat) => {
                self.ch3.wave_ram = pat;
            }
        }
    }

    pub fn apply_noise_write(&mut self, write: NoiseWrite) {
        match write {
            NoiseWrite::Length(val) => {
                self.ch4.length_timer = 64 - (val & 0x3F);
            }
            NoiseWrite::Envelope(val) => {
                self.ch4.env_initial = val >> 4;
                self.ch4.env_direction = (val & 0x08) != 0;
                self.ch4.env_period = val & 0x07;
                self.ch4.env_timer = self.ch4.env_period;
                self.ch4.volume = self.ch4.env_initial;
            }
            NoiseWrite::Polynomial(val) => {
                self.ch4.shift_clock = val >> 4;
                self.ch4.width_mode_7bit = (val & 0x08) != 0;
                self.ch4.divisor_code = val & 0x07;
            }
            NoiseWrite::Control(ctl) => {
                self.ch4.length_enabled = (ctl & 0x40) != 0;
                if (ctl & 0x80) != 0 {
                    self.ch4.trigger();
                }
            }
        }
    }

    pub fn apply_music_write(&mut self, write: MusicWrite) {
        match write {
            MusicWrite::Ch1Sweep(val) => self.ch1.set_sweep(val),
            MusicWrite::Ch1DutyLength(val) => {
                self.ch1.duty = val >> 6;
                self.ch1.length_timer = 64 - (val & 0x3F);
            }
            MusicWrite::Ch1Envelope(val) => {
                self.ch1.env_initial = val >> 4;
                self.ch1.env_direction = (val & 0x08) != 0;
                self.ch1.env_period = val & 0x07;
                self.ch1.env_timer = self.ch1.env_period;
                self.ch1.volume = self.ch1.env_initial;
            }
            MusicWrite::Ch1FrequencyLow(lo) => {
                self.ch1.freq = (self.ch1.freq & 0x0700) | (lo as u16);
            }
            MusicWrite::Ch1FrequencyControl(ctl) => {
                self.ch1.length_enabled = (ctl & 0x40) != 0;
                self.ch1.freq = (self.ch1.freq & 0x00FF) | (((ctl & 0x07) as u16) << 8);
                if (ctl & 0x80) != 0 {
                    self.ch1.trigger();
                }
            }
            MusicWrite::Ch2DutyLength(val) => {
                self.ch2.duty = val >> 6;
                self.ch2.length_timer = 64 - (val & 0x3F);
            }
            MusicWrite::Ch2Envelope(val) => {
                self.ch2.env_initial = val >> 4;
                self.ch2.env_direction = (val & 0x08) != 0;
                self.ch2.env_period = val & 0x07;
                self.ch2.env_timer = self.ch2.env_period;
                self.ch2.volume = self.ch2.env_initial;
            }
            MusicWrite::Ch2FrequencyLow(lo) => {
                self.ch2.freq = (self.ch2.freq & 0x0700) | (lo as u16);
            }
            MusicWrite::Ch2FrequencyControl(ctl) => {
                self.ch2.length_enabled = (ctl & 0x40) != 0;
                self.ch2.freq = (self.ch2.freq & 0x00FF) | (((ctl & 0x07) as u16) << 8);
                if (ctl & 0x80) != 0 {
                    self.ch2.trigger();
                }
            }
            MusicWrite::Ch3Enable(val) => {
                self.ch3.dac_enabled = (val & 0x80) != 0;
                if !self.ch3.dac_enabled {
                    self.ch3.enabled = false;
                }
            }
            MusicWrite::Ch3Length(val) => {
                self.ch3.length_timer = 256 - (val as u16);
            }
            MusicWrite::Ch3Level(val) => {
                self.ch3.volume_code = (val >> 5) & 3;
            }
            MusicWrite::Ch3FrequencyLow(lo) => {
                self.ch3.freq = (self.ch3.freq & 0x0700) | (lo as u16);
            }
            MusicWrite::Ch3FrequencyControl(ctl) => {
                self.ch3.length_enabled = (ctl & 0x40) != 0;
                self.ch3.freq = (self.ch3.freq & 0x00FF) | (((ctl & 0x07) as u16) << 8);
                if (ctl & 0x80) != 0 {
                    self.ch3.trigger();
                }
            }
            MusicWrite::Ch3WavePattern(pat) => {
                self.ch3.wave_ram = pat;
            }
            MusicWrite::Ch4Length(val) => {
                self.ch4.length_timer = 64 - (val & 0x3F);
            }
            MusicWrite::Ch4Envelope(val) => {
                self.ch4.env_initial = val >> 4;
                self.ch4.env_direction = (val & 0x08) != 0;
                self.ch4.env_period = val & 0x07;
                self.ch4.env_timer = self.ch4.env_period;
                self.ch4.volume = self.ch4.env_initial;
            }
            MusicWrite::Ch4Polynomial(val) => {
                self.ch4.shift_clock = val >> 4;
                self.ch4.width_mode_7bit = (val & 0x08) != 0;
                self.ch4.divisor_code = val & 0x07;
            }
            MusicWrite::Ch4Control(ctl) => {
                self.ch4.length_enabled = (ctl & 0x40) != 0;
                if (ctl & 0x80) != 0 {
                    self.ch4.trigger();
                }
            }
            MusicWrite::DrumTrigger => {}
        }
    }

    /// Advances the 512 Hz APU frame sequencer by the elapsed CPU cycles.
    pub fn advance_frame_sequencer(&mut self, cycles: u32) {
        self.frame_seq_cycle += cycles;
        while self.frame_seq_cycle >= 8192 {
            self.frame_seq_cycle -= 8192;
            self.frame_seq_step = (self.frame_seq_step + 1) & 7;

            // Step 0, 2, 4, 6: Clock Length (256 Hz)
            if self.frame_seq_step.is_multiple_of(2) {
                self.clock_length();
            }
            // Step 7: Clock Envelope (64 Hz)
            if self.frame_seq_step == 7 {
                self.clock_envelope();
            }
            // Steps 2 and 6: Clock Sweep (128 Hz)
            if self.frame_seq_step == 2 || self.frame_seq_step == 6 {
                self.ch1.clock_sweep();
            }
        }
    }

    fn clock_length(&mut self) {
        if self.ch1.length_enabled && self.ch1.length_timer > 0 {
            self.ch1.length_timer -= 1;
            if self.ch1.length_timer == 0 {
                self.ch1.enabled = false;
            }
        }
        if self.ch2.length_enabled && self.ch2.length_timer > 0 {
            self.ch2.length_timer -= 1;
            if self.ch2.length_timer == 0 {
                self.ch2.enabled = false;
            }
        }
        if self.ch3.length_enabled && self.ch3.length_timer > 0 {
            self.ch3.length_timer -= 1;
            if self.ch3.length_timer == 0 {
                self.ch3.enabled = false;
            }
        }
        if self.ch4.length_enabled && self.ch4.length_timer > 0 {
            self.ch4.length_timer -= 1;
            if self.ch4.length_timer == 0 {
                self.ch4.enabled = false;
            }
        }
    }

    fn clock_envelope(&mut self) {
        if self.ch1.env_period > 0 {
            if self.ch1.env_timer > 0 {
                self.ch1.env_timer -= 1;
            }
            if self.ch1.env_timer == 0 {
                self.ch1.env_timer = self.ch1.env_period;
                if self.ch1.env_direction {
                    if self.ch1.volume < 15 {
                        self.ch1.volume += 1;
                    }
                } else if self.ch1.volume > 0 {
                    self.ch1.volume -= 1;
                }
            }
        }
        if self.ch2.env_period > 0 {
            if self.ch2.env_timer > 0 {
                self.ch2.env_timer -= 1;
            }
            if self.ch2.env_timer == 0 {
                self.ch2.env_timer = self.ch2.env_period;
                if self.ch2.env_direction {
                    if self.ch2.volume < 15 {
                        self.ch2.volume += 1;
                    }
                } else if self.ch2.volume > 0 {
                    self.ch2.volume -= 1;
                }
            }
        }
        if self.ch4.env_period > 0 {
            if self.ch4.env_timer > 0 {
                self.ch4.env_timer -= 1;
            }
            if self.ch4.env_timer == 0 {
                self.ch4.env_timer = self.ch4.env_period;
                if self.ch4.env_direction {
                    if self.ch4.volume < 15 {
                        self.ch4.volume += 1;
                    }
                } else if self.ch4.volume > 0 {
                    self.ch4.volume -= 1;
                }
            }
        }
    }

    /// Samples one console frame is worth at `sample_rate`: 70,224 cycles at
    /// 4,194,304 Hz is 738.3533 samples at 44.1 kHz. The remainder carries, so
    /// the sequence is 738/738/739/738/... and the audio clock stays locked to
    /// the frame clock (a fixed 735 ran it 0.456 % fast, +7.88 cents).
    pub fn frame_sample_count(&mut self, sample_rate: u32) -> usize {
        self.sample_frame_acc += 70_224u64 * sample_rate as u64;
        let samples = self.sample_frame_acc / 4_194_304;
        self.sample_frame_acc %= 4_194_304;
        samples as usize
    }

    /// Generates interleaved stereo f32 audio samples in `[-1.0, 1.0]`.
    /// `num_frames` is the number of stereo pairs (so `2 * num_frames` floats are appended to `out`).
    pub fn generate_samples(&mut self, _sample_rate: u32, num_frames: usize, out: &mut Vec<f32>) {
        out.reserve(num_frames * 2);

        // Scaling constants (4 channels, max combined magnitude 4.0)
        let master_l = ((self.left_volume as f32) + 1.0) / 8.0;
        let master_r = ((self.right_volume as f32) + 1.0) / 8.0;
        let norm = 0.25f32;

        let pan_ch1_r = (self.nr51 & 0x01) != 0;
        let pan_ch2_r = (self.nr51 & 0x02) != 0;
        let pan_ch3_r = (self.nr51 & 0x04) != 0;
        let pan_ch4_r = (self.nr51 & 0x08) != 0;

        let pan_ch1_l = (self.nr51 & 0x10) != 0;
        let pan_ch2_l = (self.nr51 & 0x20) != 0;
        let pan_ch3_l = (self.nr51 & 0x40) != 0;
        let pan_ch4_l = (self.nr51 & 0x80) != 0;

        let safe_sample_rate = _sample_rate.max(1);

        for _ in 0..num_frames {
            self.cycle_acc += 4_194_304;
            let cycles = self.cycle_acc / safe_sample_rate;
            self.cycle_acc %= safe_sample_rate;

            self.advance_frame_sequencer(cycles);

            self.ch1.advance_cycles(cycles);
            self.ch2.advance_cycles(cycles);
            self.ch3.advance_cycles(cycles);
            self.ch4.advance_cycles(cycles);

            // True DAC outputs in [0.0, 1.0]
            let s1 = if self.ch1.enabled && self.ch1.volume > 0 {
                (self.ch1.sample() as f32) / 15.0
            } else {
                0.0
            };

            let s2 = if self.ch2.enabled && self.ch2.volume > 0 {
                (self.ch2.sample() as f32) / 15.0
            } else {
                0.0
            };

            let s3 = if self.ch3.enabled && self.ch3.dac_enabled && self.ch3.volume_code > 0 {
                (self.ch3.sample() as f32) / 15.0
            } else {
                0.0
            };

            let s4 = if self.ch4.enabled && self.ch4.volume > 0 {
                (self.ch4.sample() as f32) / 15.0
            } else {
                0.0
            };

            let mut left = 0.0f32;
            let mut right = 0.0f32;

            if pan_ch1_l {
                left += s1;
            }
            if pan_ch1_r {
                right += s1;
            }

            if pan_ch2_l {
                left += s2;
            }
            if pan_ch2_r {
                right += s2;
            }

            if pan_ch3_l {
                left += s3;
            }
            if pan_ch3_r {
                right += s3;
            }

            if pan_ch4_l {
                left += s4;
            }
            if pan_ch4_r {
                right += s4;
            }

            // Normalize and scale with master volume
            let raw_l = left * norm * master_l;
            let raw_r = right * norm * master_r;

            // High-pass DC-blocking filter (simulates Game Boy AC-coupling capacitors)
            self.cap_left += (raw_l - self.cap_left) * 0.002;
            self.cap_right += (raw_r - self.cap_right) * 0.002;

            let out_l = (raw_l - self.cap_left).clamp(-1.0, 1.0);
            let out_r = (raw_r - self.cap_right).clamp(-1.0, 1.0);

            out.push(out_l);
            out.push(out_r);
        }
    }
}
