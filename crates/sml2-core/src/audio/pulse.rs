//! Pulse-channel sound effects (Slot 0 / Channel 1), matching ROM $1007E..$106C5.
//!
//! Ground truth references:
//! - Slot 0 dispatch: ROM $100DE (`main.asm:9469..9512`)
//! - Start pointer table: ROM $1007E (`main.asm:9459..9462`, 24 entries)
//! - Tick pointer table: ROM $100AE (`main.asm:9463..9466`, 24 entries)
//! - Common start helpers: ROM $10680, $10685, $1068A (`main.asm:10008..10025`)
//! - Common stop helper: ROM $106A8 (`main.asm:10031..10043`)
//! - Silence stop helper: ROM $106A2 (`main.asm:10027..10030`)
//! - Frequency slide helper: ROM $11214..$11243 (`main.asm:11078..11120`)

pub mod tables;

#[cfg(test)]
mod tests;

use tables::*;

/// Ordered control writes for Pulse 1 (NR10..NR14) and Pulse 2 (NR21..NR24).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulseWrite {
    Sweep(u8),
    DutyLength(u8),
    Envelope(u8),
    FrequencyLow(u8),
    FrequencyControl(u8),
    Ch2DutyLength(u8),
    Ch2Envelope(u8),
    Ch2FrequencyLow(u8),
    Ch2FrequencyControl(u8),
}

/// Music channel 1 flags (sMusicCh1Flags $A41F). Bit 7 indicates sfx ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PulseMusicFlags(u8);

impl PulseMusicFlags {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PulseMusicState {
    pub flags: PulseMusicFlags,
    pub vibrato_timer: u8,
}

/// Complete state for Slot 0 (Pulse 1) sound effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PulseEffectState {
    pub request: u8,
    pub active_effect: u8,
    pub timer: u8,
    pub step: u8,
    pub duration: u8,
    pub busy: u8,
    pub freq_lo: u8,
    pub freq_hi: u8,
    pub phase: u8,
}

#[derive(Clone, Copy)]
pub struct PulseFrame {
    pub sweep: u8,
    pub duty_length: u8,
    pub envelope: u8,
    pub frequency_low: u8,
    pub frequency_control: u8,
    pub ticks: u8,
}

impl PulseEffectState {
    /// Dispatch start of a pulse effect (ROM $1007E table via $1011D).
    pub fn start(
        &mut self,
        id: u8,
        music: &mut PulseMusicState,
        stomp_chain: u8,
        out: &mut Vec<PulseWrite>,
    ) {
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
            10 => {} // No-op per ROM $102B0
            11 => self.start_effect11(music, out),
            12 => self.start_effect12(music, out),
            13 => self.start_effect13(music, out),
            14 => self.start_effect14(music, out),
            15 => self.start_effect15(music, out),
            16 => self.start_effect16(music, stomp_chain, out),
            17 => self.start_effect17(music, out),
            18 => self.start_effect18(music, out),
            19 => self.start_effect19(music, out),
            20 => self.start_effect20(music, out),
            21 => self.start_effect21(music, out),
            22 => self.start_effect22(music, out),
            23 => self.start_effect23(music, out),
            24 => self.start_effect24(music, out),
            _ => {}
        }
    }

    /// Dispatch per-frame tick of the active pulse effect (ROM $100AE table).
    pub fn tick(
        &mut self,
        music: &mut PulseMusicState,
        stomp_chain: u8,
        out: &mut Vec<PulseWrite>,
    ) {
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
            10 => {} // No-op per ROM $102B1
            11 => self.tick_effect11(music, out),
            12 => self.tick_effect12(music, out),
            13 => self.tick_effect13(music, out),
            14 => self.tick_effect14(music, out),
            15 => self.tick_effect15(music, out),
            16 => self.tick_effect16(music, stomp_chain, out),
            17 => self.tick_effect17(music, out),
            18 => self.tick_effect18(music, out),
            19 => self.tick_effect19(music, out),
            20 => self.tick_effect20(music, out),
            21 => self.tick_effect21(music, out),
            22 => self.tick_effect22(music, out),
            23 => self.tick_effect23(music, out),
            24 => self.tick_effect24(music, out),
            _ => {}
        }
    }

    // Effect 1: ROM $10127 / $1012D
    pub fn start_effect1(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, E1_START, out);
    }

    pub fn tick_effect1(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.stop(music, out);
        }
    }

    // Effect 2: ROM $1013A / $10140
    pub fn start_effect2(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, E2_START, out);
    }

    pub fn tick_effect2(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 2 {
            self.stop(music, out);
        } else {
            self.emit_frame(E2_TICK, out);
        }
    }

    // Effect 3: ROM $1015E / $1016A
    pub fn start_effect3(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        if self.active_effect == 3 {
            return;
        }
        self.start_frame(music, E3_START, out);
    }

    pub fn tick_effect3(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 2 {
            self.stop(music, out);
        } else {
            self.emit_frame(E3_TICK, out);
        }
    }

    // Effect 4: ROM $10188 / $10193
    pub fn start_effect4(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.duration = 5;
        self.start_frame(music, E4_FRAME, out);
    }

    pub fn tick_effect4(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.duration = self.duration.wrapping_sub(1);
        if self.duration == 0 {
            self.stop(music, out);
        } else {
            self.emit_frame(E4_FRAME, out);
        }
    }

    // Effect 5: ROM $101A9 / $101B4
    pub fn start_effect5(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.duration = 2;
        self.start_frame_busy(music, FRAME_SILENCE, out);
    }

    pub fn tick_effect5(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 5 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.stop(music, out);
                return;
            }
            self.step = 1;
        }
        self.emit_frame(E5_FRAMES[(self.step as usize) - 1], out);
    }

    // Effect 6: ROM $101F3 / $101F9
    pub fn start_effect6(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, E6_FIRST, out);
    }

    pub fn tick_effect6(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 2 {
            self.stop(music, out);
        } else {
            self.emit_frame(E6_SECOND, out);
        }
    }

    // Effect 7: ROM $10217 / $1021D
    pub fn start_effect7(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, E7_FIRST, out);
    }

    pub fn tick_effect7(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 2 {
            self.stop(music, out);
        } else {
            self.emit_frequency_frame(0x9d, 0x07, 80, out);
        }
    }

    // Effect 8: ROM $10238 / $1023E
    pub fn start_effect8(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame_busy(music, E8_START, out);
    }

    pub fn tick_effect8(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 4 {
            self.stop(music, out);
        } else {
            self.emit_frame(E8_FRAMES[(self.step as usize) - 1], out);
        }
    }

    // Effect 9: ROM $10271 / $1027F
    pub fn start_effect9(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_slide(music, E9_INIT, 3, out);
    }

    pub fn tick_effect9(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.timer = 5;
        self.tick_slide(music, &E9_DELTAS, 7, 1, out);
    }

    // Effect 11: ROM $102B2 / $102B8
    pub fn start_effect11(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, E11_START, out);
    }

    pub fn tick_effect11(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 4 {
            self.stop_silence(music, out);
        } else {
            let (lo, ctrl, ticks) = E11_FREQS[(self.step as usize) - 1];
            self.emit_frequency_frame(lo, ctrl, ticks, out);
        }
    }

    // Effect 12: ROM $102E2 / $102F0
    pub fn start_effect12(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.duration = 6;
        self.phase = 0;
        self.start_frame_busy(music, E12_START, out);
    }

    pub fn tick_effect12(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.phase = self.phase.wrapping_add(1);
        if self.phase == 3 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.stop(music, out);
                return;
            }
            self.phase = 1;
        }
        if self.phase == 1 {
            out.extend([
                PulseWrite::Ch2DutyLength(0x80),
                PulseWrite::Ch2Envelope(0x94),
                PulseWrite::Ch2FrequencyLow(0xCC),
                PulseWrite::Ch2FrequencyControl(0x87),
            ]);
        } else {
            self.emit_frame(E12_START, out);
        }
        self.step = self.step.wrapping_add(1);
        self.timer = E12_TIMERS[(self.step.wrapping_sub(1) as usize) % E12_TIMERS.len()];
    }

    // Effect 13: ROM $1034E / $1035C
    pub fn start_effect13(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_slide(music, E13_INIT, 12, out);
    }

    pub fn tick_effect13(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.tick_slide(music, &E13_DELTAS, 11, 1, out);
        self.timer = 1;
    }

    // Effect 14: ROM $1038E / $10399
    pub fn start_effect14(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.duration = 4;
        self.start_frame(music, E14_INIT, out);
    }

    pub fn tick_effect14(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 2 {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration != 0 {
                self.step = 1;
                self.emit_frame(E14_FRAME1, out);
            } else {
                self.step = 2;
                self.emit_frame(E14_FRAME2, out);
            }
        } else if self.step == 6 {
            self.stop(music, out);
        } else {
            // $43A9: `ld hl,$43C0 / call $51A6` indexes the frame table by
            // step-1. Entry 1 points at the louder frame ($43D0, envelope $59);
            // entries 2-5 all alias $43D6 (envelope $29).
            let frame = if self.step == 1 {
                E14_FRAME1
            } else {
                E14_FRAME2
            };
            self.emit_frame(frame, out);
        }
    }

    // Effect 15: ROM $103DC / $103E5
    pub fn start_effect15(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame_vibrato_busy(music, E15_INIT, out);
    }

    pub fn tick_effect15(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 5 {
            self.stop(music, out);
        } else {
            self.emit_frame(E15_INIT, out);
        }
    }

    // Effect 16: ROM $10408 / $10419
    pub fn start_effect16(
        &mut self,
        music: &mut PulseMusicState,
        stomp_chain: u8,
        out: &mut Vec<PulseWrite>,
    ) {
        if stomp_chain == 0 || stomp_chain >= 5 {
            return;
        }
        let frame = E16_START_FRAMES[(stomp_chain as usize) - 1];
        self.start_frame_vibrato_busy(music, frame, out);
    }

    pub fn tick_effect16(
        &mut self,
        music: &mut PulseMusicState,
        stomp_chain: u8,
        out: &mut Vec<PulseWrite>,
    ) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 2 {
            self.stop(music, out);
        } else {
            let idx = (stomp_chain.saturating_sub(1) as usize).min(3);
            let (lo, ctrl, ticks) = E16_TICK_FREQS[idx];
            self.emit_frequency_frame(lo, ctrl, ticks, out);
        }
    }

    // Effect 17: ROM $10465 / $104AA
    pub fn start_effect17(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_effect19(music, out);
    }

    pub fn tick_effect17(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.tick_effect19(music, out);
    }

    // Effect 18: ROM $1046B / $10471
    pub fn start_effect18(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, E18_INIT, out);
    }

    pub fn tick_effect18(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 3 {
            self.stop(music, out);
        } else {
            self.emit_frame(E18_FRAMES[(self.step as usize) - 1], out);
        }
    }

    // Effect 19: ROM $1049C / $104AA
    pub fn start_effect19(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_slide(music, E19_INIT, 10, out);
    }

    pub fn tick_effect19(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.tick_slide(music, &E19_DELTAS, 9, 1, out);
        self.timer = 1;
    }

    // Effect 20: ROM $104D8 / $104DE
    pub fn start_effect20(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame_busy(music, E20_INIT, out);
    }

    pub fn tick_effect20(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 24 {
            self.stop(music, out);
            return;
        }
        let env = if self.step >= 16 {
            32
        } else if self.step >= 8 {
            64
        } else {
            160
        };
        let low = E20_LOWS[((self.step as usize) - 1) % 8];
        out.extend([
            PulseWrite::Sweep(47),
            PulseWrite::DutyLength(187),
            PulseWrite::Envelope(env),
            PulseWrite::FrequencyLow(low),
            PulseWrite::FrequencyControl(0xC7),
        ]);
        self.timer = 3;
    }

    // Effect 21: ROM $10560 / $1056E
    pub fn start_effect21(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_slide(music, E21_INIT, 4, out);
    }

    pub fn tick_effect21(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.timer = 4;
        self.tick_slide(music, &E21_DELTAS, 5, 1, out);
    }

    // Effect 22: ROM $1059B / $105A1
    pub fn start_effect22(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, E22_INIT, out);
    }

    pub fn tick_effect22(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 9 {
            self.stop_silence(music, out);
        } else if self.step == 8 {
            self.emit_frame(E22_STEP8, out);
        } else {
            let env = if self.step & 1 != 0 { 48 } else { 176 };
            let low = E22_FREQS[(self.step as usize) - 1];
            out.extend([
                PulseWrite::Envelope(env),
                PulseWrite::FrequencyLow(low),
                PulseWrite::FrequencyControl(0xC7),
            ]);
            self.timer = 3;
        }
    }

    // Effect 23: ROM $105F1 / $105FF
    pub fn start_effect23(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_slide(music, E23_INIT, 4, out);
    }

    pub fn tick_effect23(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.timer = 4;
        self.tick_slide(music, &E23_DELTAS, 5, 1, out);
    }

    // Effect 24: ROM $1062C / $1063A
    pub fn start_effect24(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_slide(music, E24_INIT, 8, out);
    }

    pub fn tick_effect24(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.tick_slide(music, &E24_DELTAS, 21, 9, out);
        self.timer = 1;
    }

    // Shared start routine (ROM $1068A)
    pub fn start_frame(
        &mut self,
        music: &mut PulseMusicState,
        frame: PulseFrame,
        out: &mut Vec<PulseWrite>,
    ) {
        self.active_effect = self.request;
        self.timer = 0;
        self.step = 0;
        music.flags.claim_for_effect();
        self.emit_frame(frame, out);
    }

    // Shared start routine with busy = 1 (ROM $10685)
    pub fn start_frame_busy(
        &mut self,
        music: &mut PulseMusicState,
        frame: PulseFrame,
        out: &mut Vec<PulseWrite>,
    ) {
        self.busy = 1;
        self.start_frame(music, frame, out);
    }

    // Shared start routine with vibrato_timer = 1, busy = 1 (ROM $10680)
    pub fn start_frame_vibrato_busy(
        &mut self,
        music: &mut PulseMusicState,
        frame: PulseFrame,
        out: &mut Vec<PulseWrite>,
    ) {
        music.vibrato_timer = 1;
        self.start_frame_busy(music, frame, out);
    }

    // Shared start routine for frequency slides (ROM $111F9 + $10685)
    pub fn start_slide(
        &mut self,
        music: &mut PulseMusicState,
        frame: PulseFrame,
        duration: u8,
        out: &mut Vec<PulseWrite>,
    ) {
        self.freq_lo = frame.frequency_low;
        self.freq_hi = frame.frequency_control & 0x0F;
        self.duration = duration;
        self.start_frame_busy(music, frame, out);
    }

    // Frequency slide step (ROM $11214 / $11228)
    fn tick_slide(
        &mut self,
        music: &mut PulseMusicState,
        deltas: &[i16],
        loop_step: u8,
        reset_step: u8,
        out: &mut Vec<PulseWrite>,
    ) {
        self.step = self.step.wrapping_add(1);
        if self.step == loop_step {
            self.duration = self.duration.wrapping_sub(1);
            if self.duration == 0 {
                self.stop(music, out);
                return;
            }
            self.step = reset_step;
        }
        let delta = deltas[(self.step as usize) - 1];
        let cur_freq = ((self.freq_hi as u16) << 8) | (self.freq_lo as u16);
        let new_freq = (cur_freq as i32).wrapping_add(delta as i32) as u16;
        self.freq_lo = (new_freq & 0xFF) as u8;
        self.freq_hi = ((new_freq >> 8) & 0xFF) as u8;
        out.extend([
            PulseWrite::FrequencyLow(self.freq_lo),
            PulseWrite::FrequencyControl(self.freq_hi),
        ]);
    }

    // 5-register write helper (ROM $111BB / $111D3)
    pub fn emit_frame(&mut self, frame: PulseFrame, out: &mut Vec<PulseWrite>) {
        out.extend([
            PulseWrite::Sweep(frame.sweep),
            PulseWrite::DutyLength(frame.duty_length),
            PulseWrite::Envelope(frame.envelope),
            PulseWrite::FrequencyLow(frame.frequency_low),
            PulseWrite::FrequencyControl(frame.frequency_control),
        ]);
        self.timer = frame.ticks;
    }

    // 2-register frequency write helper (ROM $106CC / $111D3)
    pub fn emit_frequency_frame(
        &mut self,
        low: u8,
        control: u8,
        ticks: u8,
        out: &mut Vec<PulseWrite>,
    ) {
        out.extend([
            PulseWrite::FrequencyLow(low),
            PulseWrite::FrequencyControl(control),
        ]);
        self.timer = ticks;
    }

    // Shared stop routine (ROM $106A8)
    pub fn stop(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.active_effect = 0;
        self.timer = 0;
        out.push(PulseWrite::Sweep(0));
        self.step = 0;
        self.duration = 0;
        music.vibrato_timer = 0;
        self.busy = 0;
        music.flags.release_effect();
    }

    // Silence stop routine (ROM $106A2)
    pub fn stop_silence(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.emit_frame(FRAME_SILENCE, out);
        self.stop(music, out);
    }
}
