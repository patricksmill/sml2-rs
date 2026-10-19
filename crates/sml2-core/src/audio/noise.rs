//! Noise-channel sound effects (Slot 2 / Channel 4), matching ROM $10E64..$11199.
//!
//! Ground truth references:
//! - Slot 2 dispatch: ROM $10E90 (`main.asm:10706..10729`)
//! - Start pointer table: ROM $10E64 (`main.asm:10700..10703`, 11 entries)
//! - Tick pointer table: ROM $10E7A (`main.asm:10700..10703`, 11 entries)
//! - Common start helper: ROM $11163 (`main.asm:10930..10939`)
//! - Common stop helper: ROM $11181 (`main.asm:10945..10955`)
//! - Silence stop helper: ROM $1117B (`main.asm:10941..10944`)
//! - 4-register write helper: ROM $111CD (`main.asm:11008..11025`)
//! - 2-register write helper: ROM $1119F (`main.asm:10961..10964`)

#[cfg(test)]
mod tests;

/// Ordered control writes for the noise channel (Channel 4 / APU NR41..NR44).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseWrite {
    Length(u8),     // NR41 ($FF20)
    Envelope(u8),   // NR42 ($FF21)
    Polynomial(u8), // NR43 ($FF22)
    Control(u8),    // NR44 ($FF23)
}

/// Music channel 4 flags (sMusicCh4Flags $A44F). Bit 7 indicates sfx ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoiseMusicFlags(u8);

impl NoiseMusicFlags {
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

/// Complete state for Slot 2 (Noise) sound effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoiseEffectState {
    pub request: u8,
    pub active_effect: u8,
    pub timer: u8,
    pub step: u8,
    pub duration: u8,
    pub busy: u8,
}

#[derive(Clone, Copy)]
struct NoiseFrame {
    length: u8,
    envelope: u8,
    polynomial: u8,
    control: u8,
    ticks: u8,
}

const E1_START: NoiseFrame = NoiseFrame {
    length: 0x3A,
    envelope: 0x40,
    polynomial: 0x70,
    control: 0xC0,
    ticks: 2,
};
const E1_FRAMES: [NoiseFrame; 7] = [
    NoiseFrame { length: 0x3A, envelope: 0x90, polynomial: 0x50, control: 0xC0, ticks: 2 },
    NoiseFrame { length: 0x3A, envelope: 0xE0, polynomial: 0x30, control: 0xC0, ticks: 2 },
    NoiseFrame { length: 0x3A, envelope: 0xE0, polynomial: 0x30, control: 0xC0, ticks: 2 },
    NoiseFrame { length: 0x3A, envelope: 0x90, polynomial: 0x50, control: 0xC0, ticks: 2 },
    NoiseFrame { length: 0x3A, envelope: 0x90, polynomial: 0x50, control: 0xC0, ticks: 2 },
    NoiseFrame { length: 0x3A, envelope: 0x40, polynomial: 0x70, control: 0xC0, ticks: 2 },
    NoiseFrame { length: 0x3A, envelope: 0x40, polynomial: 0x70, control: 0xC0, ticks: 2 },
];

const E2_START: NoiseFrame = NoiseFrame {
    length: 0x2F,
    envelope: 0x49,
    polynomial: 0x35,
    control: 0xC0,
    ticks: 4,
};
const E2_POLY: [u8; 11] = [
    0x35, 0x34, 0x34, 0x33, 0x33, 0x32, 0x32, 0x31, 0x31, 0x30, 0x30,
];

const E3_START: NoiseFrame = NoiseFrame {
    length: 0x3B,
    envelope: 0x80,
    polynomial: 0x07,
    control: 0xC0,
    ticks: 1,
};
const E3_TICK: NoiseFrame = NoiseFrame {
    length: 0x00,
    envelope: 0x42,
    polynomial: 0x02,
    control: 0xC0,
    ticks: 4,
};

const E4_FRAME: NoiseFrame = NoiseFrame {
    length: 0x3C,
    envelope: 0x81,
    polynomial: 0x00,
    control: 0xC0,
    ticks: 6,
};

const E5_START: NoiseFrame = NoiseFrame {
    length: 0x00,
    envelope: 0x86,
    polynomial: 0x57,
    control: 0x80,
    ticks: 5,
};
const E5_STEPS: [(u8, u8, u8); 7] = [
    (0x55, 0x80, 5),
    (0x47, 0x80, 4),
    (0x45, 0x80, 4),
    (0x37, 0x80, 3),
    (0x35, 0x80, 3),
    (0x27, 0x80, 2),
    (0x25, 0x80, 0x40),
];

const E6_START: NoiseFrame = NoiseFrame {
    length: 0x00,
    envelope: 0x20,
    polynomial: 0x15,
    control: 0x80,
    ticks: 2,
};
const E6_FRAMES: [NoiseFrame; 5] = [
    NoiseFrame { length: 0x00, envelope: 0x30, polynomial: 0x17, control: 0x80, ticks: 2 },
    NoiseFrame { length: 0x00, envelope: 0x50, polynomial: 0x25, control: 0x80, ticks: 2 },
    NoiseFrame { length: 0x00, envelope: 0x70, polynomial: 0x27, control: 0x80, ticks: 3 },
    NoiseFrame { length: 0x00, envelope: 0xA0, polynomial: 0x35, control: 0x80, ticks: 3 },
    NoiseFrame { length: 0x00, envelope: 0xD1, polynomial: 0x37, control: 0x80, ticks: 12 },
];

const E7_START: NoiseFrame = NoiseFrame {
    length: 0x3B,
    envelope: 0xA0,
    polynomial: 0x20,
    control: 0xC0,
    ticks: 2,
};
const E7_TICK: NoiseFrame = NoiseFrame {
    length: 0x39,
    envelope: 0x40,
    polynomial: 0x20,
    control: 0xC0,
    ticks: 4,
};

const E8_START: NoiseFrame = NoiseFrame {
    length: 0x00,
    envelope: 0x80,
    polynomial: 0x3C,
    control: 0x80,
    ticks: 3,
};
const E8_STEPS_1_TO_4: [NoiseFrame; 4] = [
    NoiseFrame { length: 0x00, envelope: 0x70, polynomial: 0x3D, control: 0x80, ticks: 3 },
    NoiseFrame { length: 0x00, envelope: 0x60, polynomial: 0x3E, control: 0x80, ticks: 3 },
    NoiseFrame { length: 0x00, envelope: 0x50, polynomial: 0x3F, control: 0x80, ticks: 3 },
    NoiseFrame { length: 0x00, envelope: 0x40, polynomial: 0x4C, control: 0x80, ticks: 3 },
];
const E8_STEPS_5_TO_9_POLY: [u8; 5] = [0x3C, 0x3D, 0x3E, 0x3F, 0x4C];

const E9_START: NoiseFrame = NoiseFrame {
    length: 0x00,
    envelope: 0x80,
    polynomial: 0x24,
    control: 0x80,
    ticks: 4,
};
const E9_STEP4: NoiseFrame = NoiseFrame {
    length: 0x2F,
    envelope: 0x40,
    polynomial: 0x24,
    control: 0x80,
    ticks: 4,
};
const E9_STEP8: NoiseFrame = NoiseFrame {
    length: 0x2F,
    envelope: 0x10,
    polynomial: 0x24,
    control: 0x80,
    ticks: 4,
};
const E9_POLY_CTRL: [(u8, u8); 3] = [(0x26, 0x80), (0x34, 0x80), (0x36, 0xC0)];

const E10_START: NoiseFrame = NoiseFrame {
    length: 0x00,
    envelope: 0x80,
    polynomial: 0x65,
    control: 0x80,
    ticks: 8,
};
const E10_STEPS: [(u8, u8, u8); 6] = [
    (0x64, 0x80, 8),
    (0x57, 0x80, 10),
    (0x64, 0x80, 8),
    (0x65, 0x80, 12),
    (0x66, 0x80, 14),
    (0x67, 0x80, 16),
];

const E11_START: NoiseFrame = NoiseFrame {
    length: 0x2F,
    envelope: 0x0A,
    polynomial: 0x35,
    control: 0xC0,
    ticks: 4,
};
const E11_FRAMES: [NoiseFrame; 7] = [
    NoiseFrame { length: 0x2F, envelope: 0x09, polynomial: 0x45, control: 0xC0, ticks: 5 },
    NoiseFrame { length: 0x2F, envelope: 0x0A, polynomial: 0x34, control: 0xC0, ticks: 4 },
    NoiseFrame { length: 0x2F, envelope: 0x09, polynomial: 0x45, control: 0xC0, ticks: 5 },
    NoiseFrame { length: 0x2F, envelope: 0x0A, polynomial: 0x33, control: 0xC0, ticks: 4 },
    NoiseFrame { length: 0x2F, envelope: 0x09, polynomial: 0x45, control: 0xC0, ticks: 5 },
    NoiseFrame { length: 0x2F, envelope: 0x0A, polynomial: 0x32, control: 0xC0, ticks: 4 },
    NoiseFrame { length: 0x2F, envelope: 0x09, polynomial: 0x45, control: 0xC0, ticks: 5 },
];

impl NoiseEffectState {
    /// Dispatch start of a noise effect (ROM $10E64 table via $10EA9).
    pub fn start(&mut self, id: u8, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        match id {
            1 => self.start_effect1(flags, out),
            2 => self.start_effect2(flags, out),
            3 => self.start_effect3(out),
            4 => self.start_effect4(flags, out),
            5 => self.start_effect5(flags, out),
            6 => self.start_effect6(flags, out),
            7 => self.start_effect7(flags, out),
            8 => self.start_effect8(flags, out),
            9 => self.start_effect9(flags, out),
            10 => self.start_effect10(flags, out),
            11 => self.start_effect11(flags, out),
            _ => {}
        }
    }

    /// Dispatch per-frame tick of the active noise effect (ROM $10E7A table).
    pub fn tick(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        match self.active_effect {
            1 => self.tick_effect1(flags, out),
            2 => self.tick_effect2(flags, out),
            3 => self.tick_effect3(out),
            4 => self.tick_effect4(flags, out),
            5 => self.tick_effect5(flags, out),
            6 => self.tick_effect6(flags, out),
            7 => self.tick_effect7(flags, out),
            8 => self.tick_effect8(flags, out),
            9 => self.tick_effect9(flags, out),
            10 => self.tick_effect10(flags, out),
            11 => self.tick_effect11(flags, out),
            _ => {}
        }
    }

    // Effect 1: ROM $10EB3 / $10EB9
    pub fn start_effect1(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.shared_start(1, flags, E1_START, out);
    }

    pub fn tick_effect1(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 8 {
            self.stop(flags);
        } else {
            let idx = (self.step as usize) - 1;
            self.emit_frame(E1_FRAMES[idx], out);
        }
    }

    // Effect 2: ROM $10EEB / $10EF6
    pub fn start_effect2(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.duration = 3;
        self.shared_start(2, flags, E2_START, out);
    }

    pub fn tick_effect2(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 12 {
            self.stop(flags);
            return;
        }
        let env = if self.step & 1 != 0 { 0x20 } else { 0x49 };
        let poly = E2_POLY[(self.step as usize) - 1];
        out.extend([
            NoiseWrite::Envelope(env),
            NoiseWrite::Length(0x2F),
            NoiseWrite::Polynomial(poly),
            NoiseWrite::Control(0xC0),
        ]);
        self.timer = 4;
    }

    // Effect 3: ROM $10F4B / $10F56
    pub fn start_effect3(&mut self, out: &mut Vec<NoiseWrite>) {
        self.active_effect = 3;
        self.emit_frame(E3_START, out);
    }

    pub fn tick_effect3(&mut self, out: &mut Vec<NoiseWrite>) {
        self.active_effect = 0;
        self.emit_frame(E3_TICK, out);
    }

    // Effect 4: ROM $10F6A / $10F76
    pub fn start_effect4(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        if self.active_effect == 4 {
            return;
        }
        self.shared_start(4, flags, E4_FRAME, out);
    }

    pub fn tick_effect4(&mut self, flags: &mut NoiseMusicFlags, _out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.stop(flags);
    }

    // Effect 5: ROM $10F82 / $10F88
    pub fn start_effect5(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.shared_start(5, flags, E5_START, out);
    }

    pub fn tick_effect5(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 8 {
            self.stop(flags);
        } else {
            let (poly, ctrl, ticks) = E5_STEPS[(self.step as usize) - 1];
            self.emit_poly_control(poly, ctrl, ticks, out);
        }
    }

    // Effect 6: ROM $10FC5 / $10FCB
    pub fn start_effect6(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.shared_start(6, flags, E6_START, out);
    }

    pub fn tick_effect6(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 6 {
            self.stop(flags);
        } else {
            let idx = (self.step as usize) - 1;
            self.emit_frame(E6_FRAMES[idx], out);
        }
    }

    // Effect 7: ROM $11008 / $11014
    pub fn start_effect7(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        if self.active_effect == 7 {
            return;
        }
        self.shared_start(7, flags, E7_START, out);
    }

    pub fn tick_effect7(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 2 {
            self.stop(flags);
        } else {
            self.emit_frame(E7_TICK, out);
        }
    }

    // Effect 8: ROM $11030 / $11036
    pub fn start_effect8(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.shared_start(8, flags, E8_START, out);
    }

    pub fn tick_effect8(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 10 {
            self.stop_silence(flags, out);
        } else if self.step < 5 {
            let idx = (self.step as usize) - 1;
            self.emit_frame(E8_STEPS_1_TO_4[idx], out);
        } else {
            let poly = E8_STEPS_5_TO_9_POLY[(self.step as usize) - 5];
            out.push(NoiseWrite::Envelope(0x20));
            self.emit_poly_control(poly, 0x80, 3, out);
        }
    }

    // Effect 9: ROM $11086 / $1108C
    pub fn start_effect9(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.shared_start(9, flags, E9_START, out);
    }

    pub fn tick_effect9(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 12 {
            self.stop(flags);
        } else if self.step == 4 {
            self.emit_frame(E9_STEP4, out);
        } else if self.step == 8 {
            self.emit_frame(E9_STEP8, out);
        } else {
            let sub = ((self.step as usize) - 1) % 4;
            let (poly, ctrl) = E9_POLY_CTRL[sub];
            self.emit_poly_control(poly, ctrl, 4, out);
        }
    }

    // Effect 10: ROM $110DC / $110E7
    pub fn start_effect10(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.duration = 5;
        self.shared_start(10, flags, E10_START, out);
    }

    pub fn tick_effect10(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 7 {
            self.stop_silence(flags, out);
        } else {
            let (poly, ctrl, ticks) = E10_STEPS[(self.step as usize) - 1];
            self.emit_poly_control(poly, ctrl, ticks, out);
        }
    }

    // Effect 11: ROM $1111C / $11122
    pub fn start_effect11(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.shared_start(11, flags, E11_START, out);
    }

    pub fn tick_effect11(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        self.step = self.step.wrapping_add(1);
        if self.step == 8 {
            self.stop(flags);
        } else {
            let idx = (self.step as usize) - 1;
            self.emit_frame(E11_FRAMES[idx], out);
        }
    }

    // Shared start routine (ROM $11163)
    fn shared_start(
        &mut self,
        id: u8,
        flags: &mut NoiseMusicFlags,
        frame: NoiseFrame,
        out: &mut Vec<NoiseWrite>,
    ) {
        self.active_effect = id;
        self.timer = 0;
        self.step = 0;
        flags.claim_for_effect();
        self.emit_frame(frame, out);
    }

    // 4-register write helper (ROM $111CD / $111D3)
    fn emit_frame(&mut self, frame: NoiseFrame, out: &mut Vec<NoiseWrite>) {
        out.extend([
            NoiseWrite::Length(frame.length),
            NoiseWrite::Envelope(frame.envelope),
            NoiseWrite::Polynomial(frame.polynomial),
            NoiseWrite::Control(frame.control),
        ]);
        self.timer = frame.ticks;
    }

    // 2-register write helper (ROM $1119F)
    fn emit_poly_control(
        &mut self,
        polynomial: u8,
        control: u8,
        ticks: u8,
        out: &mut Vec<NoiseWrite>,
    ) {
        out.extend([
            NoiseWrite::Polynomial(polynomial),
            NoiseWrite::Control(control),
        ]);
        self.timer = ticks;
    }

    // Shared stop routine (ROM $11181)
    pub fn stop(&mut self, flags: &mut NoiseMusicFlags) {
        self.active_effect = 0;
        self.timer = 0;
        self.step = 0;
        self.duration = 0;
        self.busy = 0;
        flags.release_effect();
    }

    // Silence stop routine (ROM $1117B)
    pub fn stop_silence(&mut self, flags: &mut NoiseMusicFlags, out: &mut Vec<NoiseWrite>) {
        out.extend([
            NoiseWrite::Length(0),
            NoiseWrite::Envelope(0),
            NoiseWrite::Polynomial(0),
            NoiseWrite::Control(0),
        ]);
        self.stop(flags);
    }
}
