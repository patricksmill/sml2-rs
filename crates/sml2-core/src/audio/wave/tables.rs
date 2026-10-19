//! Wave RAM patterns, frames, and frequency delta tables for Channel 3 sound effects (ROM $106FA..$10DF2).

use super::{WaveFrame, WaveStep};

// 16-byte wave RAM waveforms
pub const PATTERN_5BD6: [u8; 16] = [0x0F, 0x0F, 0x1F, 0x1F, 0x2F, 0x2F, 0x3F, 0x3F, 0x40, 0x40, 0x50, 0x50, 0x60, 0x60, 0x70, 0x70];
pub const PATTERN_5BE6: [u8; 16] = [0x08, 0x08, 0x00, 0x22, 0x44, 0x88, 0xAA, 0xCC, 0x66, 0x00, 0x44, 0x00, 0x00, 0x44, 0x00, 0x00];
pub const PATTERN_5BF6: [u8; 16] = [0x0C, 0x0C, 0x00, 0x22, 0x44, 0x88, 0xAA, 0xCC, 0xEE, 0x00, 0x6C, 0x60, 0x00, 0x6C, 0x60, 0x00];
pub const PATTERN_5B96: [u8; 16] = [0xCC, 0xCC, 0xCC, 0xCC, 0x00, 0x00, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0x00, 0x00, 0x00, 0x00];
pub const PATTERN_5B86: [u8; 16] = [0x88, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const PATTERN_4C1F: [u8; 16] = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00];
pub const PATTERN_4CD1: [u8; 16] = [0x77, 0x77, 0x77, 0x71, 0x66, 0x66, 0x66, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const PATTERN_5BA6: [u8; 16] = [0x88, 0x88, 0x88, 0x68, 0x00, 0x00, 0x00, 0x00, 0x88, 0x88, 0x88, 0x68, 0x00, 0x00, 0x00, 0x00];

pub const FRAME_SILENCE: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x00,
    frequency_low: 0x00,
    frequency_control: 0x81,
    ticks: 1,
};

// Effect 1 (ROM $10091..$10133)
pub const E1_START: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xE2,
    level: 0x20,
    frequency_low: 0x80,
    frequency_control: 0xC6,
    ticks: 2,
};
pub const E1_STEPS: [WaveStep; 23] = [
    WaveStep::Freq(0x98, 0x86, 2),
    WaveStep::Freq(0xB0, 0x86, 2),
    WaveStep::Freq(0xC8, 0x86, 3),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xE2, level: 0x40, frequency_low: 0x80, frequency_control: 0xC6, ticks: 2 }),
    WaveStep::Freq(0x98, 0x86, 2),
    WaveStep::Freq(0xB0, 0x86, 2),
    WaveStep::Freq(0xC8, 0x86, 3),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xE2, level: 0x60, frequency_low: 0x80, frequency_control: 0xC6, ticks: 2 }),
    WaveStep::Freq(0x98, 0x86, 2),
    WaveStep::Freq(0xB0, 0x86, 2),
    WaveStep::Freq(0xC8, 0x86, 1),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xE2, level: 0x20, frequency_low: 0xC0, frequency_control: 0xC6, ticks: 2 }),
    WaveStep::Freq(0xA0, 0x86, 2),
    WaveStep::Freq(0x80, 0x86, 2),
    WaveStep::Freq(0x60, 0x86, 3),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xE2, level: 0x40, frequency_low: 0xC0, frequency_control: 0xC6, ticks: 2 }),
    WaveStep::Freq(0xA0, 0x86, 2),
    WaveStep::Freq(0x80, 0x86, 2),
    WaveStep::Freq(0x60, 0x86, 3),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xE2, level: 0x60, frequency_low: 0xC0, frequency_control: 0xC6, ticks: 2 }),
    WaveStep::Freq(0xA0, 0x86, 2),
    WaveStep::Freq(0x80, 0x86, 2),
    WaveStep::Freq(0x60, 0x86, 3),
];

// Effect 2 (ROM $10134..$10164)
pub const E2_START: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xDA,
    level: 0x20,
    frequency_low: 0x8A,
    frequency_control: 0xC7,
    ticks: 2,
};
pub const E2_STEPS: [WaveStep; 17] = [
    WaveStep::Freq(0x8C, 0xC7, 2),
    WaveStep::Freq(0x8E, 0xC7, 4),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xDA, level: 0x60, frequency_low: 0x8A, frequency_control: 0xC7, ticks: 2 }),
    WaveStep::Freq(0x8C, 0xC7, 2),
    WaveStep::Freq(0x8E, 0xC7, 4),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xDA, level: 0x20, frequency_low: 0x40, frequency_control: 0xC7, ticks: 2 }),
    WaveStep::Freq(0x48, 0xC7, 2),
    WaveStep::Freq(0x50, 0xC7, 6),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xDA, level: 0x60, frequency_low: 0x40, frequency_control: 0xC7, ticks: 2 }),
    WaveStep::Freq(0x48, 0xC7, 2),
    WaveStep::Freq(0x50, 0xC7, 6),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xDA, level: 0x20, frequency_low: 0x30, frequency_control: 0xC7, ticks: 2 }),
    WaveStep::Freq(0x38, 0xC7, 2),
    WaveStep::Freq(0x40, 0xC7, 6),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xDA, level: 0x60, frequency_low: 0x30, frequency_control: 0xC7, ticks: 2 }),
    WaveStep::Freq(0x38, 0xC7, 2),
    WaveStep::Freq(0x40, 0xC7, 6),
];

// Effect 3 (ROM $10165..$10194)
pub const E3_START: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xE2,
    level: 0x20,
    frequency_low: 0xDF,
    frequency_control: 0xC7,
    ticks: 2,
};
pub const E3_STEPS: [WaveStep; 7] = [
    WaveStep::Freq(0xDE, 0xC7, 2),
    WaveStep::Freq(0xDD, 0xC7, 2),
    WaveStep::Freq(0xDC, 0xC7, 2),
    WaveStep::Frame(WaveFrame { enable: 0x80, length: 0xE2, level: 0x60, frequency_low: 0xDF, frequency_control: 0xC7, ticks: 2 }),
    WaveStep::Freq(0xDE, 0xC7, 2),
    WaveStep::Freq(0xDD, 0xC7, 2),
    WaveStep::Freq(0xDC, 0xC7, 2),
];

// Effect 4 (ROM $10195..$10217)
pub const E4_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x20,
    frequency_low: 0x00,
    frequency_control: 0x81,
    ticks: 2,
};
pub const E4_RELOAD: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x60,
    frequency_low: 0x00,
    frequency_control: 0x81,
    ticks: 2,
};
pub const E4_DELTA: i16 = 80;

// Effect 5 (ROM $10218..$10247)
pub const E5_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x10,
    level: 0x20,
    frequency_low: 0xC0,
    frequency_control: 0x86,
    ticks: 2,
};
pub const E5_RELOAD: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x10,
    level: 0x60,
    frequency_low: 0xC0,
    frequency_control: 0x86,
    ticks: 2,
};
pub const E5_DELTAS: [i16; 16] = [16, 16, 16, 16, -768, 128, -128, 128, 656, -8, -8, -8, -8, -8, -8, -8];

// Effect 6 (ROM $10248..$10285)
pub const E6_VAR0_START: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xFB,
    level: 0x20,
    frequency_low: 0xA8,
    frequency_control: 0xC7,
    ticks: 3,
};
pub const E6_VAR0_TICK: [WaveFrame; 2] = [
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x40, frequency_low: 0xA0, frequency_control: 0xC7, ticks: 3 },
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x40, frequency_low: 0xA8, frequency_control: 0xC7, ticks: 3 },
];
pub const E6_VAR1_START: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xFB,
    level: 0x40,
    frequency_low: 0xA8,
    frequency_control: 0xC7,
    ticks: 3,
};
pub const E6_VAR1_TICK: [WaveFrame; 2] = [
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x20, frequency_low: 0xA0, frequency_control: 0xC7, ticks: 3 },
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x20, frequency_low: 0xA8, frequency_control: 0xC7, ticks: 3 },
];

// Effect 7 (ROM $10286..$10308)
pub const E7_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x20,
    frequency_low: 0x00,
    frequency_control: 0x82,
    ticks: 2,
};
pub const E7_RELOAD: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x40,
    frequency_low: 0x00,
    frequency_control: 0x82,
    ticks: 2,
};
pub const E7_DELTA: i16 = 16;

// Effect 8 (ROM $10309..$10344)
pub const E8_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x20,
    frequency_low: 0x58,
    frequency_control: 0x87,
    ticks: 2,
};
pub const E8_RELOAD: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x60,
    frequency_low: 0x58,
    frequency_control: 0x87,
    ticks: 2,
};
pub const E8_DELTAS: [i16; 28] = [
    16, -16, 16, -16, 16, -16, 32, 32,
    -10, 5, -10, 5, -10, 5, -10, 5, -10, 5, -10, 5, -10, 5, -10, 5, -10, 5, -10, 5,
];

// Effect 9 (ROM $10340..$10344)
pub const E9_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x20,
    frequency_low: 0x00,
    frequency_control: 0x86,
    ticks: 1,
};
pub const E9_RELOAD: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x40,
    frequency_low: 0x00,
    frequency_control: 0x86,
    ticks: 1,
};
pub const E9_DELTA: i16 = -12;

// Effect 10 & 11 (ROM $10346..$10360)
pub const E10_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x20,
    frequency_low: 0x00,
    frequency_control: 0x84,
    ticks: 1,
};
pub const E10_DELTAS: [i16; 10] = [-256, -256, -256, 192, 192, 192, 192, 192, -32768, 8192];

// Effect 12 (ROM $10400..$10420)
pub const E12_RELOAD: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x40,
    frequency_low: 0x00,
    frequency_control: 0x84,
    ticks: 1,
};

// Effect 13 (ROM $10421..$10452)
pub const E13_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x20,
    frequency_low: 0x00,
    frequency_control: 0x83,
    ticks: 2,
};
pub const E13_RELOAD: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x60,
    frequency_low: 0x00,
    frequency_control: 0x83,
    ticks: 2,
};
pub const E13_DELTAS: [i16; 10] = [256, 256, 256, 256, 128, -80, 128, -80, 128, -80];

// Effect 14 (ROM $10453..$10463)
pub const E14_START: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xE6,
    level: 0x20,
    frequency_low: 0x44,
    frequency_control: 0xC7,
    ticks: 6,
};

// Effect 15 (ROM $10361..$10399)
pub const E15_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x20,
    frequency_low: 0x80,
    frequency_control: 0x83,
    ticks: 2,
};
pub const E15_RELOAD_DUR2: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x40,
    frequency_low: 0xA0,
    frequency_control: 0x85,
    ticks: 2,
};
pub const E15_RELOAD_DUR1: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x60,
    frequency_low: 0xA0,
    frequency_control: 0x85,
    ticks: 2,
};
pub const E15_DELTAS: [i16; 20] = [
    160, 160, 160, 160, -768, 768, -256, 192, -256, 192, -256, 192,
    -32, -32, -32, -32, -32, -32, -32, -32,
];

// Effect 16 (ROM $10464..$10483)
pub const E16_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xD8,
    level: 0x20,
    frequency_low: 0x00,
    frequency_control: 0xC5,
    ticks: 1,
};
pub const E16_DELTA: i16 = -160;

// Effect 17 (ROM $10484..$10496)
pub const E17_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xD8,
    level: 0x20,
    frequency_low: 0xC0,
    frequency_control: 0xC6,
    ticks: 2,
};
pub const E17_DELTA: i16 = -64;

// Effect 18 (ROM $10497..$10514)
pub const E18_INIT: WaveFrame = WaveFrame {
    enable: 0x00,
    length: 0x00,
    level: 0x00,
    frequency_low: 0x00,
    frequency_control: 0x80,
    ticks: 16,
};
pub const E18_FRAMES: [WaveFrame; 6] = [
    WaveFrame { enable: 0x80, length: 0xD8, level: 0x40, frequency_low: 0x40, frequency_control: 0x87, ticks: 2 },
    WaveFrame { enable: 0x80, length: 0xD8, level: 0x40, frequency_low: 0x48, frequency_control: 0x87, ticks: 2 },
    WaveFrame { enable: 0x80, length: 0xF2, level: 0x40, frequency_low: 0x50, frequency_control: 0xC7, ticks: 8 },
    WaveFrame { enable: 0x80, length: 0xD8, level: 0x40, frequency_low: 0x40, frequency_control: 0x87, ticks: 2 },
    WaveFrame { enable: 0x80, length: 0xD8, level: 0x40, frequency_low: 0x48, frequency_control: 0x87, ticks: 2 },
    WaveFrame { enable: 0x80, length: 0xF2, level: 0x40, frequency_low: 0x50, frequency_control: 0xC7, ticks: 8 },
];

// Effect 20 (ROM $10515..$10544)
pub const E20_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x20,
    frequency_low: 0x90,
    frequency_control: 0x86,
    ticks: 1,
};
pub const E20_DELTAS: [i16; 16] = [20, 18, 18, 16, 12, -20, -20, -20, 6, 8, 10, 12, 14, 16, 18, 8];

// Effect 21 & 24 (ROM $10545..$10561, $10624..$10628)
pub const E21_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xFB,
    level: 0x40,
    frequency_low: 0xC8,
    frequency_control: 0xC7,
    ticks: 4,
};
pub const E21_FRAMES: [WaveFrame; 5] = [
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x40, frequency_low: 0xC0, frequency_control: 0xC7, ticks: 4 },
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x40, frequency_low: 0xC8, frequency_control: 0xC7, ticks: 4 },
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x40, frequency_low: 0xC0, frequency_control: 0xC7, ticks: 4 },
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x40, frequency_low: 0xC8, frequency_control: 0xC7, ticks: 4 },
    WaveFrame { enable: 0x80, length: 0xFB, level: 0x40, frequency_low: 0xC0, frequency_control: 0xC7, ticks: 4 },
];

// Effect 22 (ROM $10562..$10594)
pub const E22_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0xDE,
    level: 0x40,
    frequency_low: 0xC0,
    frequency_control: 0xC6,
    ticks: 1,
};
pub const E22_DELTAS: [i16; 2] = [-22, 17];

// Effect 23 (ROM $10595..$10623)
pub const E23_INIT: WaveFrame = WaveFrame {
    enable: 0x80,
    length: 0x00,
    level: 0x40,
    frequency_low: 0xA0,
    frequency_control: 0x87,
    ticks: 2,
};
pub const E23_DELTAS: [i16; 2] = [32, -24];
