//! Hardware RGB555 colors for monochrome cartridges on GBC and SGB.
//!
//! GBC selection follows the original boot ROM at $0476-$04E8 (licensee,
//! sixteen-byte title sum and fourth-letter collision check). The equivalent
//! expanded tables below are from SameBoy's `BootROMs/cgb_boot.asm`, labels
//! `TitleChecksums`, `PalettePerChecksum`, `Dups4thLetterArray`,
//! `PaletteCombinations`, and `Palettes`; SameBoy-only extensions are omitted.
//! The original 79 checksum rows + 29 collision bytes, 94 packed selectors
//! (expanded by their flags), and all 240 dictionary bytes were cross-checked.
//! Original disassembly: <https://gist.github.com/drhelius/6063265>.
//! Tables: <https://raw.githubusercontent.com/LIJI32/SameBoy/26a56bd6a122a6318a456abd80558408742d5adf/BootROMs/cgb_boot.asm>.
//! SGB presets: <https://raw.githubusercontent.com/LIJI32/SameBoy/26a56bd6a122a6318a456abd80558408742d5adf/Core/sgb.c>,
//! `built_in_palettes`, ordered 1-A through 4-H. No display correction is applied.

/// Independent BG, OBP0 and OBP1 color RAM entries (RGB555, red in bits 0..4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwarePalettes {
    pub bg: [u16; 4],
    pub obj0: [u16; 4],
    pub obj1: [u16; 4],
}

/// Select the GBC boot palette for a DMG cartridge, with no held boot keys.
/// `header` is cartridge $0134..=$014B, including both licensee fields.
/// The $80 flag in the original lookup controls the boot tilemap only and is
/// deliberately absent from this color-only table.
pub fn gbc_auto(header: &[u8; 0x18]) -> HardwarePalettes {
    let nintendo = header[0x17] == 1 || (header[0x17] == 0x33 && header[0x10..0x12] == *b"01");
    let checksum = header[..16].iter().fold(0u8, |sum, &b| sum.wrapping_add(b));
    let index = if nintendo {
        TITLE_CHECKSUMS
            .iter()
            .enumerate()
            .find_map(|(i, &sum)| {
                (sum == checksum && (i < 65 || header[3] == DUPLICATE_FOURTH_LETTERS[i - 65]))
                    .then_some(PALETTE_PER_CHECKSUM[i] as usize)
            })
            .unwrap_or(0)
    } else {
        0
    };
    let [obj0, obj1, bg] = GBC_COMBINATIONS[index];
    HardwarePalettes {
        bg: gbc_colors(bg),
        obj0: gbc_colors(obj0),
        obj1: gbc_colors(obj1),
    }
}

fn gbc_colors(offset: usize) -> [u16; 4] {
    GBC_COLORS[offset..offset + 4]
        .try_into()
        .expect("boot palette table offset")
}

/// SGB system palette by zero-based index: 0 = 1-A, 7 = 1-H, 31 = 4-H.
/// A monochrome game uses the same system palette for BG and both OBJ groups.
pub fn sgb_preset(index: usize) -> Option<HardwarePalettes> {
    SGB_PRESETS
        .get(index)
        .copied()
        .map(|colors| HardwarePalettes {
            bg: colors,
            obj0: colors,
            obj1: colors,
        })
}

/// Expand RGB555 to uncorrected RGB888 by replicating the high bits.
pub fn rgb555(color: u16) -> [u8; 3] {
    [0, 5, 10].map(|shift| {
        let channel = ((color >> shift) & 31) as u8;
        (channel << 3) | (channel >> 2)
    })
}

const TITLE_CHECKSUMS: [u8; 94] = [
    0x00, 0x88, 0x16, 0x36, 0xD1, 0xDB, 0xF2, 0x3C, 0x8C, 0x92, 0x3D, 0x5C, 0x58, 0xC9, 0x3E, 0x70,
    0x1D, 0x59, 0x69, 0x19, 0x35, 0xA8, 0x14, 0xAA, 0x75, 0x95, 0x99, 0x34, 0x6F, 0x15, 0xFF, 0x97,
    0x4B, 0x90, 0x17, 0x10, 0x39, 0xF7, 0xF6, 0xA2, 0x49, 0x4E, 0x43, 0x68, 0xE0, 0x8B, 0xF0, 0xCE,
    0x0C, 0x29, 0xE8, 0xB7, 0x86, 0x9A, 0x52, 0x01, 0x9D, 0x71, 0x9C, 0xBD, 0x5D, 0x6D, 0x67, 0x3F,
    0x6B, 0xB3, 0x46, 0x28, 0xA5, 0xC6, 0xD3, 0x27, 0x61, 0x18, 0x66, 0x6A, 0xBF, 0x0D, 0xF4, 0xB3,
    0x46, 0x28, 0xA5, 0xC6, 0xD3, 0x27, 0x61, 0x18, 0x66, 0x6A, 0xBF, 0x0D, 0xF4, 0xB3,
];

const PALETTE_PER_CHECKSUM: [u8; 94] = [
    0, 4, 5, 35, 34, 3, 31, 15, 10, 5, 19, 36, 7, 37, 30, 44, 21, 32, 31, 20, 5, 33, 13, 14, 5, 29,
    5, 18, 9, 3, 2, 26, 25, 25, 41, 42, 26, 45, 42, 45, 36, 38, 26, 42, 30, 41, 34, 34, 5, 42, 6,
    5, 33, 25, 42, 42, 40, 2, 16, 25, 42, 42, 5, 0, 39, 36, 22, 25, 6, 32, 12, 36, 11, 39, 18, 39,
    24, 31, 50, 17, 46, 6, 27, 0, 47, 41, 41, 0, 0, 19, 34, 23, 18, 29,
];

const GBC_COLORS: [u16; 120] = [
    0x7FFF, 0x32BF, 0x00D0, 0x0000, 0x639F, 0x4279, 0x15B0, 0x04CB, 0x7FFF, 0x6E31, 0x454A, 0x0000,
    0x7FFF, 0x1BEF, 0x0200, 0x0000, 0x7FFF, 0x421F, 0x1CF2, 0x0000, 0x7FFF, 0x5294, 0x294A, 0x0000,
    0x7FFF, 0x03FF, 0x012F, 0x0000, 0x7FFF, 0x03EF, 0x01D6, 0x0000, 0x7FFF, 0x42B5, 0x3DC8, 0x0000,
    0x7E74, 0x03FF, 0x0180, 0x0000, 0x67FF, 0x77AC, 0x1A13, 0x2D6B, 0x7ED6, 0x4BFF, 0x2175, 0x0000,
    0x53FF, 0x4A5F, 0x7E52, 0x0000, 0x4FFF, 0x7ED2, 0x3A4C, 0x1CE0, 0x03ED, 0x7FFF, 0x255F, 0x0000,
    0x036A, 0x021F, 0x03FF, 0x7FFF, 0x7FFF, 0x01DF, 0x0112, 0x0000, 0x231F, 0x035F, 0x00F2, 0x0009,
    0x7FFF, 0x03EA, 0x011F, 0x0000, 0x299F, 0x001A, 0x000C, 0x0000, 0x7FFF, 0x027F, 0x001F, 0x0000,
    0x7FFF, 0x03E0, 0x0206, 0x0120, 0x7FFF, 0x7EEB, 0x001F, 0x7C00, 0x7FFF, 0x3FFF, 0x7E00, 0x001F,
    0x7FFF, 0x03FF, 0x001F, 0x0000, 0x03FF, 0x001F, 0x000C, 0x0000, 0x7FFF, 0x033F, 0x0193, 0x0000,
    0x0000, 0x4200, 0x037F, 0x7FFF, 0x7FFF, 0x7E8C, 0x7C00, 0x0000, 0x7FFF, 0x1BEF, 0x6180, 0x0000,
];

const DUPLICATE_FOURTH_LETTERS: &[u8; 29] = b"BEFAARBEKEK R-URAR INAILICE R";

// Offsets in 16-bit colors, rather than palette indices: four combinations
// start one color before a palette boundary, as the boot ROM does.
const GBC_COMBINATIONS: [[usize; 3]; 51] = [
    [16, 16, 116],   // 0
    [72, 72, 72],    // 1
    [80, 80, 80],    // 2
    [96, 96, 96],    // 3
    [36, 36, 36],    // 4
    [0, 0, 0],       // 5
    [108, 108, 108], // 6
    [20, 20, 20],    // 7
    [48, 48, 48],    // 8
    [104, 104, 104], // 9
    [64, 32, 32],    // 10
    [16, 112, 112],  // 11
    [16, 8, 8],      // 12
    [12, 16, 16],    // 13
    [16, 116, 116],  // 14
    [112, 16, 112],  // 15
    [8, 68, 8],      // 16
    [64, 64, 32],    // 17
    [16, 16, 28],    // 18
    [16, 16, 72],    // 19
    [16, 16, 80],    // 20
    [76, 76, 36],    // 21
    [15, 15, 44],    // 22
    [68, 68, 8],     // 23
    [16, 16, 8],     // 24
    [16, 16, 12],    // 25
    [112, 112, 0],   // 26
    [12, 12, 0],     // 27
    [0, 0, 4],       // 28
    [72, 88, 72],    // 29
    [80, 88, 80],    // 30
    [96, 88, 96],    // 31
    [64, 88, 32],    // 32
    [68, 16, 52],    // 33
    [111, 0, 56],    // 34
    [111, 16, 60],   // 35
    [76, 91, 36],    // 36
    [64, 112, 40],   // 37
    [16, 92, 112],   // 38
    [68, 88, 8],     // 39
    [16, 0, 8],      // 40
    [16, 112, 12],   // 41
    [112, 12, 0],    // 42
    [12, 112, 16],   // 43
    [84, 112, 16],   // 44
    [12, 112, 0],    // 45
    [100, 12, 112],  // 46
    [0, 112, 32],    // 47
    [16, 12, 112],   // 48
    [112, 12, 24],   // 49
    [16, 112, 116],  // 50
];

/// All 32 built-in SGB system palettes in hardware RGB555.
pub const SGB_PRESETS: [[u16; 4]; 32] = [
    [0x67BF, 0x265B, 0x10B5, 0x2866], // 1-A
    [0x637B, 0x3AD9, 0x0956, 0x0000], // 1-B
    [0x7F1F, 0x2A7D, 0x30F3, 0x4CE7], // 1-C
    [0x57FF, 0x2618, 0x001F, 0x006A], // 1-D
    [0x5B7F, 0x3F0F, 0x222D, 0x10EB], // 1-E
    [0x7FBB, 0x2A3C, 0x0015, 0x0900], // 1-F
    [0x2800, 0x7680, 0x01EF, 0x2FFF], // 1-G
    [0x73BF, 0x46FF, 0x0110, 0x0066], // 1-H
    [0x533E, 0x2638, 0x01E5, 0x0000], // 2-A
    [0x7FFF, 0x2BBF, 0x00DF, 0x2C0A], // 2-B
    [0x7F1F, 0x463D, 0x74CF, 0x4CA5], // 2-C
    [0x53FF, 0x03E0, 0x00DF, 0x2800], // 2-D
    [0x433F, 0x72D2, 0x3045, 0x0822], // 2-E
    [0x7FFA, 0x2A5F, 0x0014, 0x0003], // 2-F
    [0x1EED, 0x215C, 0x42FC, 0x0060], // 2-G
    [0x7FFF, 0x5EF7, 0x39CE, 0x0000], // 2-H
    [0x4F5F, 0x630E, 0x159F, 0x3126], // 3-A
    [0x637B, 0x121C, 0x0140, 0x0840], // 3-B
    [0x66BC, 0x3FFF, 0x7EE0, 0x2C84], // 3-C
    [0x5FFE, 0x3EBC, 0x0321, 0x0000], // 3-D
    [0x63FF, 0x36DC, 0x11F6, 0x392A], // 3-E
    [0x65EF, 0x7DBF, 0x035F, 0x2108], // 3-F
    [0x2B6C, 0x7FFF, 0x1CD9, 0x0007], // 3-G
    [0x53FC, 0x1F2F, 0x0E29, 0x0061], // 3-H
    [0x36BE, 0x7EAF, 0x681A, 0x3C00], // 4-A
    [0x7BBE, 0x329D, 0x1DE8, 0x0423], // 4-B
    [0x739F, 0x6A9B, 0x7293, 0x0001], // 4-C
    [0x5FFF, 0x6732, 0x3DA9, 0x2481], // 4-D
    [0x577F, 0x3EBC, 0x456F, 0x1880], // 4-E
    [0x6B57, 0x6E1B, 0x5010, 0x0007], // 4-F
    [0x0F96, 0x2C97, 0x0045, 0x3200], // 4-G
    [0x67FF, 0x2F17, 0x2230, 0x1548], // 4-H
];

#[cfg(test)]
mod tests;
