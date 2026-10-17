//! Pulse-channel effect 6: ROM $101F3/$101F9 and frames $1020B/$10211.

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

impl PulseEffectState {
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
mod tests {
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
}
