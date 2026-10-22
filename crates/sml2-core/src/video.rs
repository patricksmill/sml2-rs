//! Display helpers shared by the extraction tool and the runtime: the
//! greyscale shade convention, BGP palette expansion and 2bpp tile decoding.
//!
//! The color helpers deliberately stay palette-index-based (a 4-shade
//! mapping over a 2-bit color value) so a runtime can swap in texture packs
//! and alternate palettes without touching stored tile data — see
//! `sml2-rs/DESIGN.md`.

pub mod cgb;

/// Greyscale value for each GB color value (0 = lightest).
pub const SHADES: [u8; 4] = [255, 170, 85, 0];
/// Size of the composed BG tile block (`$8800-$97FF`).
pub const BG_VRAM_LEN: usize = 0x1000;

/// Map a BGP palette value to display greys: `out[c] = SHADES[(bgp >> 2c) & 3]`.
pub fn bgp_shades(bgp: u8) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (c, o) in out.iter_mut().enumerate() {
        *o = SHADES[((bgp >> (2 * c)) & 0x03) as usize];
    }
    out
}

/// Decode one 2bpp tile (16 bytes) into 8x8 color values 0..=3.
pub fn tile_colors(bytes: &[u8; 16]) -> [u8; 64] {
    let mut tile = [0u8; 64];
    for y in 0..8 {
        let (lo, hi) = (bytes[y * 2], bytes[y * 2 + 1]);
        for x in 0..8 {
            let bit = 7 - x;
            tile[y * 8 + x] = (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1);
        }
    }
    tile
}

/// VRAM byte offset of a background tile number in the composed block
/// (`$8800` base; LCDC bit 4 = 0 addressing: `< 128` reads `$9000 + t*16`,
/// `>= 128` reads `$8800 + (t-128)*16`).
pub fn bg_tile_offset(tile: u8) -> usize {
    if tile < 128 {
        0x800 + tile as usize * 16
    } else {
        (tile as usize - 128) * 16
    }
}

/// Hardware palette that supplied the winning pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PaletteSource {
    Bg = 0,
    Obj0 = 1,
    Obj1 = 2,
}

/// A composed pixel after the BGP/OBP register has mapped its tile colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PalettePixel {
    pub source: PaletteSource,
    /// Hardware shade, from lightest (0) to darkest (3).
    pub shade: u8,
}

impl PalettePixel {
    /// Decode the compact tokens emitted by palette-aware rasterizers.
    pub fn from_token(token: u8) -> Self {
        let source = match token >> 2 {
            0 => PaletteSource::Bg,
            1 => PaletteSource::Obj0,
            2 => PaletteSource::Obj1,
            _ => panic!("invalid palette pixel token"),
        };
        Self {
            source,
            shade: token & 3,
        }
    }

    pub fn grayscale(self) -> u8 {
        SHADES[self.shade as usize]
    }
}

/// Rasterizer output representation. Palette indices retain source provenance
/// in bits 2..3 and the register-mapped hardware shade in bits 0..1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Grayscale,
    PaletteIndices,
}

impl OutputMode {
    pub fn palette(self, register: u8, source: PaletteSource) -> [u8; 4] {
        std::array::from_fn(|color| {
            let shade = (register >> (color * 2)) & 3;
            match self {
                Self::Grayscale => SHADES[shade as usize],
                Self::PaletteIndices => (source as u8) * 4 + shade,
            }
        })
    }

    pub fn blank(self, shade: u8) -> u8 {
        self.palette(0xE4, PaletteSource::Bg)[shade as usize]
    }

    pub fn is_white(self, pixel: u8) -> bool {
        match self {
            Self::Grayscale => pixel == 255,
            Self::PaletteIndices => pixel & 3 == 0,
        }
    }
}
