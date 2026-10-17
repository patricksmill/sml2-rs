//! Pulse-channel effects 6 and 7, sharing the source channel start/stop behavior.

/// Ordered control writes for the first pulse channel. Values retain the
/// original channel-control encodings for a future synthesizer/output adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulseWrite {
    Sweep(u8),
    DutyLength(u8),
    Envelope(u8),
    FrequencyLow(u8),
    FrequencyControl(u8),
}

/// Music's channel flags retain unrelated bits when an effect takes ownership.
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

    fn claim_for_effect(&mut self) {
        self.0 |= 0x80;
    }

    fn release_effect(&mut self) {
        self.0 &= 0x7f;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PulseMusicState {
    pub flags: PulseMusicFlags,
    pub vibrato_timer: u8,
}

/// Persistent pulse effect values. Unknown request IDs and noncanonical counter
/// values are preserved; the caller selects the appropriate effect entrypoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PulseEffectState {
    pub request: u8,
    pub active_effect: u8,
    pub timer: u8,
    pub step: u8,
    pub duration: u8,
    pub busy: u8,
}

#[derive(Clone, Copy)]
struct PulseFrame {
    sweep: u8,
    duty_length: u8,
    envelope: u8,
    frequency_low: u8,
    frequency_control: u8,
    ticks: u8,
}

const EFFECT6_FIRST: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0xba,
    envelope: 0xc0,
    frequency_low: 0xb6,
    frequency_control: 0xc6,
    ticks: 8,
};

const EFFECT6_SECOND: PulseFrame = PulseFrame {
    frequency_low: 0x39,
    frequency_control: 0xc7,
    ticks: 2,
    ..EFFECT6_FIRST
};

const EFFECT7_FIRST: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0xa7,
    frequency_low: 0x83,
    frequency_control: 0x87,
    ticks: 4,
};

impl PulseEffectState {
    /// Source start entry $10217. As with effect 6, the caller selects the
    /// entrypoint; this copies any pending request verbatim without consuming it.
    pub fn start_effect7(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, EFFECT7_FIRST, out);
    }

    /// Source tick entry $1021D. The second command changes only frequency,
    /// retaining the current sweep, duty, and envelope controls.
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

    /// Source start entry $101F3. Copies the request verbatim without consuming
    /// it; request acceptance and clearing belong to the future dispatcher.
    pub fn start_effect6(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.start_frame(music, EFFECT6_FIRST, out);
    }

    /// Source tick entry $101F9. The caller must select this entry for effect 6;
    /// source has no active-ID, busy, or zero-timer guard at this entrypoint.
    pub fn tick_effect6(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 2 {
            self.stop(music, out);
        } else {
            self.emit_frame(EFFECT6_SECOND, out);
        }
    }

    // Shared source start $1068A: does not change duration, busy, or vibrato.
    fn start_frame(
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

    // Source $111BB/$111D3: control writes precede the frame timer store.
    fn emit_frame(&mut self, frame: PulseFrame, out: &mut Vec<PulseWrite>) {
        out.extend([
            PulseWrite::Sweep(frame.sweep),
            PulseWrite::DutyLength(frame.duty_length),
            PulseWrite::Envelope(frame.envelope),
            PulseWrite::FrequencyLow(frame.frequency_low),
            PulseWrite::FrequencyControl(frame.frequency_control),
        ]);
        self.timer = frame.ticks;
    }

    // Source $106CC/$111D3: only the two frequency controls, then the timer.
    fn emit_frequency_frame(&mut self, low: u8, control: u8, ticks: u8, out: &mut Vec<PulseWrite>) {
        out.extend([
            PulseWrite::FrequencyLow(low),
            PulseWrite::FrequencyControl(control),
        ]);
        self.timer = ticks;
    }

    // Shared source stop $106A8; preserves the pending request and lower flags.
    fn stop(&mut self, music: &mut PulseMusicState, out: &mut Vec<PulseWrite>) {
        self.active_effect = 0;
        self.timer = 0;
        out.push(PulseWrite::Sweep(0));
        self.step = 0;
        self.duration = 0;
        music.vibrato_timer = 0;
        self.busy = 0;
        music.flags.release_effect();
    }
}

#[cfg(test)]
mod tests;
