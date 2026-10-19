//! Data frames and frequency delta tables for Pulse 1 effects (ROM $1007E..$106C5).

use super::PulseFrame;

pub const E1_START: PulseFrame = PulseFrame {
    sweep: 0x37,
    duty_length: 0x40,
    envelope: 0xD2,
    frequency_low: 0x27,
    frequency_control: 0x86,
    ticks: 48,
};

pub const E2_START: PulseFrame = PulseFrame {
    sweep: 0x1E,
    duty_length: 0xA0,
    envelope: 0xD0,
    frequency_low: 0xAC,
    frequency_control: 0xC5,
    ticks: 4,
};
pub const E2_TICK: PulseFrame = PulseFrame {
    sweep: 0x14,
    duty_length: 0xAD,
    envelope: 0xD0,
    frequency_low: 0x9B,
    frequency_control: 0xC3,
    ticks: 12,
};

pub const E3_START: PulseFrame = PulseFrame {
    sweep: 0x27,
    duty_length: 0xA0,
    envelope: 0x39,
    frequency_low: 0xD6,
    frequency_control: 0xC6,
    ticks: 6,
};
pub const E3_TICK: PulseFrame = PulseFrame {
    sweep: 0x27,
    duty_length: 0xA0,
    envelope: 0x39,
    frequency_low: 0x16,
    frequency_control: 0xC7,
    ticks: 18,
};

pub const E4_FRAME: PulseFrame = PulseFrame {
    sweep: 0x17,
    duty_length: 0xBC,
    envelope: 0x64,
    frequency_low: 0x44,
    frequency_control: 0xC7,
    ticks: 2,
};

pub const FRAME_SILENCE: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x3F,
    envelope: 0x00,
    frequency_low: 0x00,
    frequency_control: 0xC1,
    ticks: 1,
};

pub const E5_FRAMES: [PulseFrame; 4] = [
    PulseFrame { sweep: 0x1D, duty_length: 0xAF, envelope: 0xC0, frequency_low: 0x90, frequency_control: 0xC7, ticks: 3 },
    PulseFrame { sweep: 0x1D, duty_length: 0xAF, envelope: 0xC0, frequency_low: 0x90, frequency_control: 0xC7, ticks: 4 },
    PulseFrame { sweep: 0x1D, duty_length: 0xAF, envelope: 0x40, frequency_low: 0x90, frequency_control: 0xC7, ticks: 3 },
    PulseFrame { sweep: 0x1D, duty_length: 0xAF, envelope: 0x40, frequency_low: 0x90, frequency_control: 0xC7, ticks: 6 },
];

pub const E6_FIRST: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0xBA,
    envelope: 0xC0,
    frequency_low: 0xB6,
    frequency_control: 0xC6,
    ticks: 8,
};
pub const E6_SECOND: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0xBA,
    envelope: 0xC0,
    frequency_low: 0x39,
    frequency_control: 0xC7,
    ticks: 2,
};

pub const E7_FIRST: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0xA7,
    frequency_low: 0x83,
    frequency_control: 0x87,
    ticks: 4,
};

pub const E8_START: PulseFrame = PulseFrame {
    sweep: 0x25,
    duty_length: 0x9F,
    envelope: 0x80,
    frequency_low: 0x32,
    frequency_control: 0xC5,
    ticks: 8,
};
pub const E8_FRAMES: [PulseFrame; 3] = [
    PulseFrame { sweep: 0x25, duty_length: 0x9F, envelope: 0xA0, frequency_low: 0x5A, frequency_control: 0xC5, ticks: 8 },
    PulseFrame { sweep: 0x25, duty_length: 0x9F, envelope: 0xC0, frequency_low: 0x72, frequency_control: 0xC5, ticks: 8 },
    PulseFrame { sweep: 0x25, duty_length: 0x9F, envelope: 0xE0, frequency_low: 0x8A, frequency_control: 0xC5, ticks: 8 },
];

pub const E9_INIT: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0xC7,
    frequency_low: 0xB5,
    frequency_control: 0x87,
    ticks: 5,
};
pub const E9_DELTAS: [i16; 6] = [8, 7, 9, 13, -7, -30];

pub const E11_START: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0xAF,
    envelope: 0xA0,
    frequency_low: 0x63,
    frequency_control: 0xC5,
    ticks: 1,
};
pub const E11_FREQS: [(u8, u8, u8); 3] = [
    (0xB2, 0xC6, 1),
    (0x59, 0xC7, 1),
    (0xAC, 0xC7, 2),
];

pub const E12_START: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0x94,
    frequency_low: 0xC0,
    frequency_control: 0x87,
    ticks: 4,
};
pub const E12_TIMERS: [u8; 12] = [4, 4, 5, 5, 7, 7, 9, 9, 11, 13, 16, 48];

pub const E13_INIT: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0xC7,
    frequency_low: 0x80,
    frequency_control: 0x87,
    ticks: 1,
};
pub const E13_DELTAS: [i16; 10] = [2, 4, 8, 4, 2, -4, -8, -16, -8, -4];

pub const E14_INIT: PulseFrame = PulseFrame {
    sweep: 0x2F,
    duty_length: 0xB5,
    envelope: 0x59,
    frequency_low: 0x39,
    frequency_control: 0xC7,
    ticks: 3,
};
pub const E14_FRAME1: PulseFrame = PulseFrame {
    sweep: 0x2F,
    duty_length: 0xB5,
    envelope: 0x59,
    frequency_low: 0x70,
    frequency_control: 0xC7,
    ticks: 3,
};
pub const E14_FRAME2: PulseFrame = PulseFrame {
    sweep: 0x2F,
    duty_length: 0xB5,
    envelope: 0x29,
    frequency_low: 0x70,
    frequency_control: 0xC7,
    ticks: 3,
};

pub const E15_INIT: PulseFrame = PulseFrame {
    sweep: 0x17,
    duty_length: 0xBC,
    envelope: 0xB4,
    frequency_low: 0x54,
    frequency_control: 0xC7,
    ticks: 2,
};

pub const E16_START_FRAMES: [PulseFrame; 4] = [
    PulseFrame { sweep: 0x00, duty_length: 0xAF, envelope: 0x80, frequency_low: 0x06, frequency_control: 0xC7, ticks: 2 },
    PulseFrame { sweep: 0x00, duty_length: 0xAF, envelope: 0x80, frequency_low: 0x21, frequency_control: 0xC7, ticks: 2 },
    PulseFrame { sweep: 0x00, duty_length: 0xAF, envelope: 0x80, frequency_low: 0x39, frequency_control: 0xC7, ticks: 2 },
    PulseFrame { sweep: 0x00, duty_length: 0xAF, envelope: 0x80, frequency_low: 0x4F, frequency_control: 0xC7, ticks: 2 },
];
pub const E16_TICK_FREQS: [(u8, u8, u8); 4] = [
    (0x44, 0xC7, 2),
    (0x59, 0xC7, 2),
    (0x6B, 0xC7, 2),
    (0x7B, 0xC7, 2),
];

pub const E18_INIT: PulseFrame = PulseFrame {
    sweep: 0x37,
    duty_length: 0xA6,
    envelope: 0x49,
    frequency_low: 0xC0,
    frequency_control: 0xC6,
    ticks: 8,
};
pub const E18_FRAMES: [PulseFrame; 2] = [
    PulseFrame { sweep: 0x37, duty_length: 0xA6, envelope: 0x09, frequency_low: 0xC0, frequency_control: 0xC6, ticks: 8 },
    PulseFrame { sweep: 0x37, duty_length: 0xA6, envelope: 0x0A, frequency_low: 0xC0, frequency_control: 0xC6, ticks: 8 },
];

pub const E19_INIT: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x00,
    envelope: 0xF5,
    frequency_low: 0x80,
    frequency_control: 0x84,
    ticks: 1,
};
pub const E19_DELTAS: [i16; 8] = [80, 48, -48, -80, -80, -48, 48, 96];

pub const E20_INIT: PulseFrame = PulseFrame {
    sweep: 0x2F,
    duty_length: 0xBB,
    envelope: 0xA0,
    frequency_low: 0xE0,
    frequency_control: 0xC7,
    ticks: 3,
};
pub const E20_LOWS: [u8; 8] = [0xC0, 0xA0, 0x80, 0x60, 0x40, 0x20, 0x00, 0xE0];

pub const E21_INIT: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x00,
    envelope: 0x87,
    frequency_low: 0xAC,
    frequency_control: 0x87,
    ticks: 4,
};
pub const E21_DELTAS: [i16; 4] = [10, 11, 13, -34];

pub const E22_INIT: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0xB0,
    frequency_low: 0x9D,
    frequency_control: 0xC7,
    ticks: 3,
};
pub const E22_FREQS: [u8; 7] = [0x9D, 0x97, 0x97, 0x9D, 0x9D, 0xBE, 0xBE];
pub const E22_STEP8: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0xB6,
    frequency_low: 0xB6,
    frequency_control: 0x87,
    ticks: 64,
};

pub const E23_INIT: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0xC5,
    frequency_low: 0xC0,
    frequency_control: 0x87,
    ticks: 4,
};
pub const E23_DELTAS: [i16; 4] = [48, -96, 64, -96];

pub const E24_INIT: PulseFrame = PulseFrame {
    sweep: 0x00,
    duty_length: 0x80,
    envelope: 0xF7,
    frequency_low: 0x00,
    frequency_control: 0x85,
    ticks: 1,
};
pub const E24_DELTAS: [i16; 20] = [
    -128, -128, -128, -128, -128, -128, -128, -128,
    384, -192, 384, -192, 384, -192,
    -384, 192, -384, 192, -384, 192,
];
