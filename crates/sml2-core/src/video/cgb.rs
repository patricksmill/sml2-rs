//! Native CGB color composition, independent of DMG palette registers.
//!
//! Hardware contracts: Pan Docs `Tile_Maps.md` (BG-to-OBJ priority) and
//! `OAM.md` (attributes and drawing priority), available at
//! <https://github.com/gbdev/pandocs/tree/master/src>.

use crate::hardware_palettes::rgb555;

/// Shared CGB BG/OBJ attribute bits. Bit 4 is retained but not interpreted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CgbAttributes(u8);

impl CgbAttributes {
    pub const fn from_byte(byte: u8) -> Self {
        Self(byte)
    }
    pub const fn to_byte(self) -> u8 {
        self.0
    }
    pub const fn palette(self) -> u8 {
        self.0 & 7
    }
    pub const fn bank(self) -> usize {
        ((self.0 >> 3) & 1) as usize
    }
    pub const fn flip_x(self) -> bool {
        self.0 & 0x20 != 0
    }
    pub const fn flip_y(self) -> bool {
        self.0 & 0x40 != 0
    }
    /// BG: cover opaque OBJ; OBJ: appear behind opaque BG.
    pub const fn priority(self) -> bool {
        self.0 & 0x80 != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorError {
    PaletteLength { actual: usize },
    InvalidIndex { color: u8, palette: u8 },
    TileBounds,
    FrameLength { background: usize, objects: usize },
}

impl std::fmt::Display for ColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PaletteLength { actual } => {
                write!(f, "CGB palette RAM needs 64 bytes; got {actual}")
            }
            Self::InvalidIndex { color, palette } => {
                write!(f, "invalid CGB color {color} or palette {palette}")
            }
            Self::TileBounds => f.write_str("CGB tile sample exceeds bank or 8x8 tile bounds"),
            Self::FrameLength {
                background,
                objects,
            } => write!(f, "CGB layer lengths differ: {background} and {objects}"),
        }
    }
}

impl std::error::Error for ColorError {}

/// Eight palettes of four little-endian RGB555 values for each layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CgbPalettes {
    bg: [[u16; 4]; 8],
    obj: [[u16; 4]; 8],
}

impl CgbPalettes {
    pub fn from_bytes(bg: &[u8], obj: &[u8]) -> Result<Self, ColorError> {
        fn decode(bytes: &[u8]) -> Result<[[u16; 4]; 8], ColorError> {
            if bytes.len() != 64 {
                return Err(ColorError::PaletteLength {
                    actual: bytes.len(),
                });
            }
            // Bit 15 is unused by CGB hardware.
            Ok(std::array::from_fn(|p| {
                std::array::from_fn(|c| {
                    let offset = p * 8 + c * 2;
                    u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) & 0x7fff
                })
            }))
        }
        Ok(Self {
            bg: decode(bg)?,
            obj: decode(obj)?,
        })
    }

    pub fn background(&self) -> &[[u16; 4]; 8] {
        &self.bg
    }
    pub fn objects(&self) -> &[[u16; 4]; 8] {
        &self.obj
    }
}

/// Original tile color index plus CGB palette. Construction checks both bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexedPixel {
    color: u8,
    palette: u8,
}

impl IndexedPixel {
    pub fn new(color: u8, palette: u8) -> Result<Self, ColorError> {
        if color > 3 || palette > 7 {
            return Err(ColorError::InvalidIndex { color, palette });
        }
        Ok(Self { color, palette })
    }
    pub const fn color(self) -> u8 {
        self.color
    }
    pub const fn palette(self) -> u8 {
        self.palette
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackgroundPixel {
    pub index: IndexedPixel,
    pub priority: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectPixel {
    pub index: IndexedPixel,
    pub behind_background: bool,
}

/// Sample an 8x8 tile at an explicit byte offset within its selected VRAM bank.
/// The caller resolves signed BG tile addressing or 8x16 OBJ tile selection.
pub fn sample_tile(
    banks: [&[u8]; 2],
    byte_offset: usize,
    attributes: CgbAttributes,
    x: u8,
    y: u8,
) -> Result<IndexedPixel, ColorError> {
    if x >= 8 || y >= 8 {
        return Err(ColorError::TileBounds);
    }
    let end = byte_offset.checked_add(16).ok_or(ColorError::TileBounds)?;
    let tile = banks[attributes.bank()]
        .get(byte_offset..end)
        .ok_or(ColorError::TileBounds)?;
    let x = if attributes.flip_x() { 7 - x } else { x };
    let y = if attributes.flip_y() { 7 - y } else { y } as usize;
    let bit = 7 - x;
    let color = ((tile[y * 2] >> bit) & 1) | (((tile[y * 2 + 1] >> bit) & 1) << 1);
    IndexedPixel::new(color, attributes.palette())
}

/// Select the first opaque OBJ in CGB OAM order, before comparing it with BG.
pub fn select_object(
    objects_in_oam_order: impl IntoIterator<Item = ObjectPixel>,
) -> Option<ObjectPixel> {
    objects_in_oam_order
        .into_iter()
        .find(|pixel| pixel.index.color != 0)
}

/// Compose resolved layers. `bg_priority_enabled` is CGB LCDC bit 0.
/// Clear bit 0 disables BG priority, while BG colors remain visible.
pub fn compose_pixel(
    bg: BackgroundPixel,
    obj: Option<ObjectPixel>,
    bg_priority_enabled: bool,
    palettes: &CgbPalettes,
) -> [u8; 3] {
    if let Some(obj) = obj.filter(|obj| obj.index.color != 0) {
        let bg_wins =
            bg_priority_enabled && bg.index.color != 0 && (bg.priority || obj.behind_background);
        if !bg_wins {
            return rgb555(palettes.obj[obj.index.palette as usize][obj.index.color as usize]);
        }
    }
    rgb555(palettes.bg[bg.index.palette as usize][bg.index.color as usize])
}

/// Convert equally sized, resolved pixel layers to uncorrected RGB888.
/// Palette changes can be applied between calls (for example, per scanline).
pub fn compose_frame(
    background: &[BackgroundPixel],
    objects: &[Option<ObjectPixel>],
    bg_priority_enabled: bool,
    palettes: &CgbPalettes,
) -> Result<Vec<[u8; 3]>, ColorError> {
    if background.len() != objects.len() {
        return Err(ColorError::FrameLength {
            background: background.len(),
            objects: objects.len(),
        });
    }
    Ok(background
        .iter()
        .zip(objects)
        .map(|(&bg, &obj)| compose_pixel(bg, obj, bg_priority_enabled, palettes))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palettes() -> CgbPalettes {
        let mut bg = [0; 64];
        let mut obj = [0; 64];
        // BG color 0 and 1 both red: priority must use the index, never RGB.
        bg[..4].copy_from_slice(&[0x1f, 0x00, 0x1f, 0x00]);
        obj[2..4].copy_from_slice(&[0xe0, 0x03]); // green
        bg[62..].copy_from_slice(&[0x00, 0xfc]); // blue; unused bit 15 set
        CgbPalettes::from_bytes(&bg, &obj).unwrap()
    }

    #[test]
    fn literal_rgb555_and_palette_bounds() {
        let palettes = palettes();
        assert_eq!(palettes.background()[7][3], 0x7c00);
        assert_eq!(rgb555(palettes.background()[7][3]), [0, 0, 255]);
        assert_eq!(rgb555(palettes.objects()[0][1]), [0, 255, 0]);
        for length in [0, 63, 65] {
            assert!(matches!(
                CgbPalettes::from_bytes(&vec![0; length], &[0; 64]),
                Err(ColorError::PaletteLength { .. })
            ));
            assert!(CgbPalettes::from_bytes(&[0; 64], &vec![0; length]).is_err());
        }
        assert!(IndexedPixel::new(4, 0).is_err());
        assert!(IndexedPixel::new(0, 8).is_err());
    }

    #[test]
    fn literal_tile_banks_attributes_and_flips() {
        let mut bank1 = [0; 32];
        bank1[16..18].copy_from_slice(&[0x80, 0x01]); // corners 1, 2
        bank1[30..32].copy_from_slice(&[0x01, 0x01]); // bottom right 3
        let banks = [&[0; 32][..], &bank1[..]];
        let attributes = CgbAttributes::from_byte(0x9f);
        assert_eq!(attributes.to_byte(), 0x9f);
        assert!(attributes.priority());
        assert_eq!(attributes.bank(), 1);
        assert_eq!(attributes.palette(), 7);
        assert_eq!(
            sample_tile(banks, 16, attributes, 0, 0).unwrap(),
            IndexedPixel::new(1, 7).unwrap()
        );
        assert_eq!(
            sample_tile(banks, 16, CgbAttributes::from_byte(0x28), 0, 0)
                .unwrap()
                .color(),
            2
        );
        assert_eq!(
            sample_tile(banks, 16, CgbAttributes::from_byte(0x68), 0, 0)
                .unwrap()
                .color(),
            3
        );
        assert_eq!(
            sample_tile(banks, 16, CgbAttributes::default(), 0, 0)
                .unwrap()
                .color(),
            0
        );
        assert_eq!(
            sample_tile(banks, 16, CgbAttributes::from_byte(0x48), 7, 0)
                .unwrap()
                .color(),
            3
        );
        assert!(sample_tile(banks, 17, attributes, 0, 0).is_err());
        assert!(sample_tile(banks, usize::MAX, attributes, 0, 0).is_err());
        assert!(sample_tile(banks, 16, attributes, 8, 0).is_err());
        assert!(sample_tile(banks, 16, attributes, 0, 8).is_err());
    }

    #[test]
    fn complete_hardware_priority_table_and_transparency() {
        let palettes = palettes();
        for color in [0, 1] {
            for enabled in [false, true] {
                for priority in [false, true] {
                    for behind in [false, true] {
                        let bg = BackgroundPixel {
                            index: IndexedPixel::new(color, 0).unwrap(),
                            priority,
                        };
                        let obj = ObjectPixel {
                            index: IndexedPixel::new(1, 0).unwrap(),
                            behind_background: behind,
                        };
                        let expected = if color != 0 && enabled && (priority || behind) {
                            [255, 0, 0]
                        } else {
                            [0, 255, 0]
                        };
                        assert_eq!(compose_pixel(bg, Some(obj), enabled, &palettes), expected);
                        let transparent = ObjectPixel {
                            index: IndexedPixel::new(0, 0).unwrap(),
                            ..obj
                        };
                        assert_eq!(
                            compose_pixel(bg, Some(transparent), enabled, &palettes),
                            [255, 0, 0]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn oam_winner_masks_later_objects_even_when_behind_bg() {
        let bg = BackgroundPixel {
            index: IndexedPixel::new(1, 0).unwrap(),
            priority: false,
        };
        let front = ObjectPixel {
            index: IndexedPixel::new(1, 0).unwrap(),
            behind_background: true,
        };
        let back = ObjectPixel {
            behind_background: false,
            ..front
        };
        let transparent = ObjectPixel {
            index: IndexedPixel::new(0, 0).unwrap(),
            ..front
        };
        let winner = select_object([transparent, front, back]);
        assert_eq!(winner, Some(front));
        assert_eq!(
            compose_frame(&[bg], &[winner], true, &palettes()).unwrap(),
            [[255, 0, 0]]
        );
        assert!(compose_frame(&[bg], &[], true, &palettes()).is_err());
    }
}
