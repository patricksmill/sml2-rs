//! Wave-channel sound effects (Slot 1 / Channel 3), matching ROM $106FA..$10DF2.
//!
//! Ground truth references:
//! - Slot 1 dispatch: ROM $106D3 (`main.asm:10054..10082`)
//! - Start pointer table: ROM $106FA (`main.asm:10084..10086`, 24 entries)
//! - Tick pointer table: ROM $1072A (`main.asm:10087..10089`, 24 entries)
//! - Register write helper 5-reg: ROM $111C7 (`main.asm:11002..11025`)
//! - Register write helper 2-reg: ROM $10E5D (`main.asm:10695..10698`)
//! - Wave RAM copy helper: ROM $112A7 (`main.asm:11146..11159`)
//! - Start effect helper: ROM $10DF8 (`main.asm:10633..10645`)
//! - Common stop helper: ROM $10E1D (`main.asm:10654..10667`)
//! - Silence stop helper: ROM $10E17 (`main.asm:10650..10653`)
//! - Frequency slide helper: ROM $1121E (`main.asm:11086..11120`)

pub mod tables;

#[cfg(test)]
mod tests;

use tables::*;

/// Ordered register and buffer writes for Wave Channel 3 (NR30..NR34 and Wave RAM $FF30..$FF3F).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveWrite {
    Enable(u8),           // NR30 ($FF1A)
    Length(u8),           // NR31 ($FF1B)
    Level(u8),            // NR32 ($FF1C)
    FrequencyLow(u8),     // NR33 ($FF1D)
    FrequencyControl(u8), // NR34 ($FF1E)
    WavePattern([u8; 16]), // Wave RAM ($FF30..$FF3F)
}

/// Music channel 3 flags (sMusicCh3Flags $A441). Bit 7 indicates sfx ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WaveMusicFlags(u8);

impl WaveMusicFlags {
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn effect_owns_channel(self) -> bool {
        self.0 & 0x80 != 0
    }

    pub fn claim_for_effect(&mut self) {
        self.0 |= 0x80;
    }

    pub fn release_effect(&mut self) {
        self.0 &= 0x7f;
    }
}

/// Music Channel 3 context passed into wave effect routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WaveMusicState {
    pub flags: WaveMusicFlags,
    pub wave_pattern: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveFrame {
    pub enable: u8,
    pub length: u8,
    pub level: u8,
    pub frequency_low: u8,
    pub frequency_control: u8,
    pub ticks: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveStep {
    Frame(WaveFrame),
    Freq(u8, u8, u8), // freq_lo, freq_hi, ticks
}

/// Complete state for Slot 1 (Wave / Channel 3) sound effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WaveEffectState {
    pub request: u8,
    pub active_effect: u8,
    pub timer: u8,
    pub step: u8,
    pub duration: u8,
    pub busy: u8,
    pub variant: u8,
    pub freq_lo: u8,
    pub freq_hi: u8,
}

impl WaveEffectState {
    /// Dispatch start of a wave effect (ROM $106FA table via $106D3).
    pub fn start(&mut self, id: u8, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        if self.busy != 0 && id != 20 {
            return;
        }

        match id {
            1 => self.start_effect1(music, out),
            2 => self.start_effect2(music, out),
            3 => self.start_effect3(music, out),
            4 => self.start_effect4(music, out),
            5 => self.start_effect5(music, out),
            6 => self.start_effect6(music, out),
            7 => self.start_effect7(music, out),
            8 => self.start_effect8(music, out),
            9 => self.start_effect9(music, out),
            10 | 11 => self.start_effect10_11(id, music, out),
            12 => self.start_effect12(music, out),
            13 => self.start_effect13(music, out),
            14 => self.start_effect14(music, out),
            15 => self.start_effect15(music, out),
            16 => self.start_effect16(music, out),
            17 => self.start_effect17(music, out),
            18 => self.start_effect18(music, out),
            19 => {} // No-op per ROM $10E42
            20 => self.start_effect20(music, out),
            21 => self.start_effect21(music, out),
            22 => self.start_effect22(music, out),
            23 => self.start_effect23(music, out),
            24 => self.start_effect24(music, out),
            _ => {}
        }
    }

    /// Dispatch per-frame tick of active wave effect (ROM $1072A table).
    pub fn tick(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        match self.active_effect {
            1 => self.tick_effect1(music, out),
            2 => self.tick_effect2(music, out),
            3 => self.tick_effect3(music, out),
            4 => self.tick_effect4(music, out),
            5 => self.tick_effect5(music, out),
            6 => self.tick_effect6(music, out),
            7 => self.tick_effect7(music, out),
            8 => self.tick_effect8(music, out),
            9 => self.tick_effect9(music, out),
            10 | 11 => self.tick_effect10_11(music, out),
            12 => self.tick_effect12(music, out),
            13 => self.tick_effect13(music, out),
            14 => self.tick_effect14(music, out),
            15 => self.tick_effect15(music, out),
            16 => self.tick_effect16(music, out),
            17 => self.tick_effect17(music, out),
            18 => self.tick_effect18(music, out),
            19 => {} // No-op per ROM $10E42
            20 => self.tick_effect20(music, out),
            21 => self.tick_effect21(music, out),
            22 => self.tick_effect22(music, out),
            23 => self.tick_effect23(music, out),
            24 => self.tick_effect21(music, out),
            _ => {}
        }
    }

    // Effect 1: ROM $10091 / $10096
    pub fn start_effect1(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 2;
        self.upload_pattern(PATTERN_5BD6, out);
        self.start_frame(1, music, E1_START, out);
    }

    pub fn tick_effect1(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 24 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.stop(music, out);
                return;
            }
            self.step = 12;
        }
        self.apply_step(E1_STEPS[self.step as usize - 1], out);
    }

    // Effect 2: ROM $10134 / $10137
    pub fn start_effect2(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.upload_pattern(PATTERN_5BE6, out);
        self.start_frame(2, music, E2_START, out);
    }

    pub fn tick_effect2(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 18 {
            self.stop(music, out);
            return;
        }
        self.apply_step(E2_STEPS[self.step as usize - 1], out);
    }

    // Effect 3: ROM $10165 / $10170
    pub fn start_effect3(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 2;
        self.upload_pattern(PATTERN_5BE6, out);
        self.start_frame(3, music, E3_START, out);
    }

    pub fn tick_effect3(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 8 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.stop(music, out);
                return;
            }
            self.step = 0;
            self.emit_frame(E3_START, out);
            return;
        }
        self.apply_step(E3_STEPS[self.step as usize - 1], out);
    }

    // Effect 4: ROM $10195 / $10201
    pub fn start_effect4(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 36;
        self.upload_pattern(PATTERN_5BF6, out);
        self.start_slide(4, music, E4_INIT, out);
    }

    pub fn tick_effect4(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = self.duration.wrapping_sub(1);
        if self.duration == 0 {
            self.silence_stop(music, out);
            return;
        }
        if self.duration == 18 {
            self.step = 1;
            self.init_freq_from_frame(E4_RELOAD);
            self.emit_frame(E4_RELOAD, out);
        } else {
            self.step = 1;
            self.tick_single_slide(E4_DELTA, out);
        }
    }

    // Effect 5: ROM $10218 / $10224
    pub fn start_effect5(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 2;
        self.upload_pattern(PATTERN_5BE6, out);
        self.start_slide(5, music, E5_INIT, out);
    }

    pub fn tick_effect5(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 17 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.silence_stop(music, out);
                return;
            }
            self.step = 0;
            self.init_freq_from_frame(E5_RELOAD);
            self.emit_frame(E5_RELOAD, out);
            return;
        }
        self.timer = 2;
        self.tick_slide(&E5_DELTAS, out);
    }

    // Effect 6: ROM $10248 / $10264
    pub fn start_effect6(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        if self.active_effect == 6 {
            return;
        }
        self.upload_pattern(PATTERN_5B96, out);
        if self.variant != 0 {
            self.variant = 0;
            self.start_frame(6, music, E6_VAR0_START, out);
        } else {
            self.variant = 1;
            self.start_frame(6, music, E6_VAR1_START, out);
        }
    }

    pub fn tick_effect6(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 3 {
            self.stop(music, out);
            return;
        }
        let frame = if self.variant == 0 {
            E6_VAR0_TICK[self.step as usize - 1]
        } else {
            E6_VAR1_TICK[self.step as usize - 1]
        };
        self.emit_frame(frame, out);
    }

    // Effect 7: ROM $10286 / $10292
    pub fn start_effect7(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 48;
        self.upload_pattern(PATTERN_5B96, out);
        self.start_slide(7, music, E7_INIT, out);
    }

    pub fn tick_effect7(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = self.duration.wrapping_sub(1);
        if self.duration == 0 {
            self.silence_stop(music, out);
            return;
        }
        if self.duration == 24 {
            self.step = 1;
            self.init_freq_from_frame(E7_RELOAD);
            self.emit_frame(E7_RELOAD, out);
        } else {
            self.step = 1;
            self.tick_single_slide(E7_DELTA, out);
        }
    }

    // Effect 8: ROM $10309 / $10315
    pub fn start_effect8(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 2;
        self.upload_pattern(PATTERN_5BE6, out);
        self.start_slide(8, music, E8_INIT, out);
    }

    pub fn tick_effect8(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 29 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.silence_stop(music, out);
                return;
            }
            self.step = 0;
            self.init_freq_from_frame(E8_RELOAD);
            self.emit_frame(E8_RELOAD, out);
            return;
        }
        self.timer = 1;
        self.tick_slide(&E8_DELTAS, out);
    }

    // Effect 9: ROM $10340 / $10341
    pub fn start_effect9(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 20;
        self.busy = 1;
        self.upload_pattern(PATTERN_5BE6, out);
        self.start_slide(9, music, E9_INIT, out);
    }

    pub fn tick_effect9(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = self.duration.wrapping_sub(1);
        if self.duration == 0 {
            self.silence_stop(music, out);
            return;
        }
        if self.duration == 16 {
            self.init_freq_from_frame(E9_RELOAD);
            self.emit_frame(E9_RELOAD, out);
        } else {
            self.step = 1;
            self.tick_single_slide(E9_DELTA, out);
        }
    }

    // Effect 10 & 11: ROM $10346 / $10351
    pub fn start_effect10_11(&mut self, id: u8, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.upload_pattern(PATTERN_5B96, out);
        self.start_slide(id, music, E10_INIT, out);
    }

    pub fn tick_effect10_11(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.step = self.step.wrapping_add(1);
        if self.step == 11 {
            self.silence_stop(music, out);
            return;
        }
        self.tick_slide(&E10_DELTAS, out);
    }

    // Effect 12: ROM $10400 / $10403
    pub fn start_effect12(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 2;
        self.upload_pattern(PATTERN_5B96, out);
        self.start_slide(12, music, E10_INIT, out);
    }

    pub fn tick_effect12(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.step = self.step.wrapping_add(1);
        if self.step == 11 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.silence_stop(music, out);
                return;
            }
            self.step = 0;
            self.init_freq_from_frame(E12_RELOAD);
            self.emit_frame(E12_RELOAD, out);
            return;
        }
        self.tick_slide(&E10_DELTAS, out);
    }

    // Effect 13: ROM $10421 / $10428
    pub fn start_effect13(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 2;
        self.upload_pattern(PATTERN_5B86, out);
        self.start_slide(13, music, E13_INIT, out);
    }

    pub fn tick_effect13(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.timer = 2;
        self.step = self.step.wrapping_add(1);
        if self.step == 11 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.silence_stop(music, out);
                return;
            }
            self.step = 0;
            self.init_freq_from_frame(E13_RELOAD);
            self.emit_frame(E13_RELOAD, out);
            return;
        }
        self.tick_slide(&E13_DELTAS, out);
    }

    // Effect 14: ROM $10453 / $10457
    pub fn start_effect14(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.upload_pattern(PATTERN_4C1F, out);
        self.start_frame(14, music, E14_START, out);
    }

    pub fn tick_effect14(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.stop(music, out);
        }
    }

    // Effect 15: ROM $10361 / $10367
    pub fn start_effect15(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 3;
        self.upload_pattern(PATTERN_5BF6, out);
        self.start_slide(15, music, E15_INIT, out);
    }

    pub fn tick_effect15(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.timer = 2;
        self.step = self.step.wrapping_add(1);
        if self.step == 21 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.silence_stop(music, out);
                return;
            }
            let frame = if self.duration == 1 {
                E15_RELOAD_DUR1
            } else {
                E15_RELOAD_DUR2
            };
            self.step = 4;
            self.init_freq_from_frame(frame);
            self.emit_frame(frame, out);
            return;
        }
        self.tick_slide(&E15_DELTAS, out);
    }

    // Effect 16: ROM $10464 / $10474
    pub fn start_effect16(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 8;
        self.upload_pattern(PATTERN_5B96, out);
        self.start_slide(16, music, E16_INIT, out);
    }

    pub fn tick_effect16(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = self.duration.wrapping_sub(1);
        if self.duration == 0 {
            self.stop(music, out);
            return;
        }
        self.step = 1;
        self.tick_single_slide(E16_DELTA, out);
    }

    // Effect 17: ROM $10484 / $10487
    pub fn start_effect17(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 8;
        self.upload_pattern(PATTERN_5B96, out);
        self.start_slide(17, music, E17_INIT, out);
    }

    pub fn tick_effect17(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = self.duration.wrapping_sub(1);
        if self.duration == 0 {
            self.stop(music, out);
            return;
        }
        self.step = 1;
        self.tick_single_slide(E17_DELTA, out);
    }

    // Effect 18: ROM $10497 / $10500
    pub fn start_effect18(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.upload_pattern(PATTERN_5BE6, out);
        self.start_frame(18, music, E18_INIT, out);
    }

    pub fn tick_effect18(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 7 {
            self.stop(music, out);
            return;
        }
        self.emit_frame(E18_FRAMES[self.step as usize - 1], out);
    }

    // Effect 20: ROM $10515 / $10522
    pub fn start_effect20(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 7;
        self.busy = 1;
        self.upload_pattern(PATTERN_4CD1, out);
        self.start_slide(20, music, E20_INIT, out);
    }

    pub fn tick_effect20(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.step = self.step.wrapping_add(1);
        if self.step == 9 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration != 0 {
                self.step = 1;
            }
        } else if self.step == 17 {
            self.silence_stop(music, out);
            return;
        }
        self.tick_slide(&E20_DELTAS, out);
    }

    // Effect 21: ROM $10545 / $10549
    pub fn start_effect21(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.upload_pattern(PATTERN_5B96, out);
        self.start_frame(21, music, E21_INIT, out);
    }

    pub fn tick_effect21(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 6 {
            self.stop(music, out);
            return;
        }
        self.emit_frame(E21_FRAMES[self.step as usize - 1], out);
    }

    // Effect 22: ROM $10562 / $10568
    pub fn start_effect22(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 10;
        self.upload_pattern(PATTERN_5B96, out);
        self.start_slide(22, music, E22_INIT, out);
    }

    pub fn tick_effect22(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.step = self.step.wrapping_add(1);
        if self.step == 3 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.stop(music, out);
                return;
            }
            if self.duration == 5 {
                self.step = 0;
                self.init_freq_from_frame(E22_INIT);
                self.emit_frame(E22_INIT, out);
                return;
            }
            self.step = 1;
        }
        self.tick_slide(&E22_DELTAS, out);
    }

    // Effect 23: ROM $10595 / $10602
    pub fn start_effect23(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.duration = 7;
        self.upload_pattern(PATTERN_5BA6, out);
        self.start_slide(23, music, E23_INIT, out);
    }

    pub fn tick_effect23(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.timer = 2;
        self.step = self.step.wrapping_add(1);
        if self.step == 3 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.silence_stop(music, out);
                return;
            }
            self.step = 1;
        }
        self.tick_slide(&E23_DELTAS, out);
    }

    // Effect 24: ROM $10624..$10628
    pub fn start_effect24(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        if self.active_effect == 0 {
            self.start_effect21(music, out);
            self.active_effect = 24;
        }
    }

    // Internal helpers
    fn upload_pattern(&mut self, pattern: [u8; 16], out: &mut Vec<WaveWrite>) {
        out.push(WaveWrite::Enable(0));
        out.push(WaveWrite::WavePattern(pattern));
    }

    fn emit_frame(&mut self, frame: WaveFrame, out: &mut Vec<WaveWrite>) {
        out.push(WaveWrite::Enable(frame.enable));
        out.push(WaveWrite::Length(frame.length));
        out.push(WaveWrite::Level(frame.level));
        out.push(WaveWrite::FrequencyLow(frame.frequency_low));
        out.push(WaveWrite::FrequencyControl(frame.frequency_control));
        self.timer = frame.ticks;
    }

    fn emit_freq(&mut self, lo: u8, hi: u8, ticks: u8, out: &mut Vec<WaveWrite>) {
        out.push(WaveWrite::FrequencyLow(lo));
        out.push(WaveWrite::FrequencyControl(hi));
        self.timer = ticks;
    }

    fn apply_step(&mut self, step: WaveStep, out: &mut Vec<WaveWrite>) {
        match step {
            WaveStep::Frame(frame) => self.emit_frame(frame, out),
            WaveStep::Freq(lo, hi, ticks) => self.emit_freq(lo, hi, ticks, out),
        }
    }

    fn init_freq_from_frame(&mut self, frame: WaveFrame) {
        self.freq_lo = frame.frequency_low;
        self.freq_hi = frame.frequency_control & 0x0F;
    }

    fn start_frame(
        &mut self,
        id: u8,
        music: &mut WaveMusicState,
        frame: WaveFrame,
        out: &mut Vec<WaveWrite>,
    ) {
        self.active_effect = id;
        music.flags.claim_for_effect();
        self.timer = 0;
        self.step = 0;
        out.push(WaveWrite::Enable(0)); // rAUD3ENA cleared in ROM $10DF8
        self.emit_frame(frame, out);
    }

    fn start_slide(
        &mut self,
        id: u8,
        music: &mut WaveMusicState,
        frame: WaveFrame,
        out: &mut Vec<WaveWrite>,
    ) {
        self.init_freq_from_frame(frame);
        self.start_frame(id, music, frame, out);
    }

    fn tick_single_slide(&mut self, delta: i16, out: &mut Vec<WaveWrite>) {
        let freq = u16::from_le_bytes([self.freq_lo, self.freq_hi]);
        let next = freq.wrapping_add(delta as u16);
        let [lo, hi] = next.to_le_bytes();
        self.freq_lo = lo;
        self.freq_hi = hi;
        out.push(WaveWrite::FrequencyLow(lo));
        out.push(WaveWrite::FrequencyControl(hi));
    }

    fn tick_slide(&mut self, deltas: &[i16], out: &mut Vec<WaveWrite>) {
        if self.step == 0 || self.step as usize > deltas.len() {
            return;
        }
        let delta = deltas[(self.step - 1) as usize];
        self.tick_single_slide(delta, out);
    }

    fn stop(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        self.timer = 0;
        self.step = 0;
        self.active_effect = 0;
        self.busy = 0;
        music.flags.release_effect();
        out.push(WaveWrite::Enable(0));
        out.push(WaveWrite::WavePattern(music.wave_pattern));
    }

    fn silence_stop(&mut self, music: &mut WaveMusicState, out: &mut Vec<WaveWrite>) {
        out.push(WaveWrite::Enable(FRAME_SILENCE.enable));
        out.push(WaveWrite::Length(FRAME_SILENCE.length));
        out.push(WaveWrite::Level(FRAME_SILENCE.level));
        out.push(WaveWrite::FrequencyLow(FRAME_SILENCE.frequency_low));
        out.push(WaveWrite::FrequencyControl(FRAME_SILENCE.frequency_control));
        self.stop(music, out);
    }
}
