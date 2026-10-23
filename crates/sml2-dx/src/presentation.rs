//! Every level's DX VRAM, attributes, palettes and mappings, derived from a
//! [`Bundle`] by the DX ROM's own level loader. See docs/sml2dx-audit.md,
//! "All levels".

use crate::{
    bundle::{Bundle, LEVELS},
    ImportError,
};
use sml2_core::{
    character::Character,
    object::{sprite_map_index, MappingEntry},
    video::cgb::CgbPalettes,
};
use std::collections::BTreeMap;

/// One level as `LoadMarioGFX` ($0669), `$21:$732C`, `$26:$7CD6` and
/// `$24:$7912` leave it.
#[derive(Clone, Debug)]
pub struct LevelGraphics {
    /// VRAM0 and VRAM1 `$8800..$97FF`.
    pub background: [Vec<u8>; 2],
    /// `$21:$4000 + level * $100`: attributes indexed by BG tile ID.
    pub tile_attributes: Vec<u8>,
    /// `$26:$4000 + level * $40` and `$1000` past it.
    pub bg_palettes: Vec<u8>,
    pub obj_palettes: Vec<u8>,
    /// The HUD row's attributes, through `$24:$6000 + level * 2`.
    pub hud_attributes: Vec<u8>,
    /// `$A21A`'s high nibble: 1 for the alternate theme config nibble 5 selects.
    pub theme: u8,
    /// Index into [`Presentation::object_tables`].
    pub object_table: usize,
}

/// One character and theme: `$8000` graphics for the normal and Fire forms,
/// the VRAM1 overlay, and the player mapping table.
#[derive(Clone, Debug)]
pub struct PlayerGraphics {
    pub normal: Vec<u8>,
    pub fire: Vec<u8>,
    pub bank1: Vec<u8>,
    pub mappings: Vec<Vec<MappingEntry>>,
}

#[derive(Clone, Debug)]
pub struct Presentation {
    pub levels: Vec<LevelGraphics>,
    /// `[character][theme]`.
    pub players: [[PlayerGraphics; 2]; 2],
    /// Object mapping tables, shared by the levels that draw from the same
    /// bank and mapping mode.
    pub object_tables: Vec<Vec<Vec<MappingEntry>>>,
    /// `$24:$7C24`: the HUD row's tile IDs.
    pub hud_tiles: Vec<u8>,
    /// Bank `$1D` `$6680..$7680`: 16 animation rows of four `$40`-byte phases.
    pub animation_rows: Vec<u8>,
    /// The DX bytes behind every base-ROM address a level copies mid-level
    /// (arena reloads, scene transfers), keyed by bank.
    pub banks: BTreeMap<u8, Vec<u8>>,
}

/// One sprite or player mapping table: cells per sprite ID.
type MappingTable = Vec<Vec<MappingEntry>>;

const SHARED_BANK: u8 = 0x1B;
const EXTRA_BANK: u8 = 0x07;
const HUD_BANK: u8 = 0x20;
const ATTRIBUTE_BANK: u8 = 0x21;
const LEVEL_BANK: u8 = 0x24;
const PALETTE_BANK: u8 = 0x26;
const ANIMATION_BANK: u8 = 0x1D;

impl Presentation {
    pub fn new(bundle: &Bundle) -> Result<Self, ImportError> {
        bundle.check_shape()?;
        let mut tables: Vec<((u8, u8), MappingTable)> = Vec::new();
        let levels = (0..LEVELS as u8)
            .map(|level| {
                let graphics = level_graphics(bundle, level)?;
                // `$07EA` picks the object drawing bank; each keeps mode A at
                // `$40B1` (168) and mode B at `$4F11` (228).
                let bank = match level {
                    0x0B | 0x0F => 0x28,
                    0x0D => 0x3A,
                    _ => 0x23,
                };
                let mode = sprite_map_index(level);
                let key = (bank, mode);
                let object_table = match tables.iter().position(|(k, _)| *k == key) {
                    Some(index) => index,
                    None => {
                        let (table, count) = if mode == 0 {
                            (0x40B1, 168)
                        } else {
                            (0x4F11, 228)
                        };
                        tables.push((key, mappings(bundle, bank, table, count)?));
                        tables.len() - 1
                    }
                };
                Ok(LevelGraphics {
                    object_table,
                    ..graphics
                })
            })
            .collect::<Result<Vec<_>, ImportError>>()?;
        let player = |character: Character, theme: u8| -> Result<PlayerGraphics, ImportError> {
            // `$24:$7880`: graphics bank `$2A`/`$2B` by `$FFF6`; `$2C:$5D62`
            // and `$2D:$5E16` read mappings from `$4000`, or `$6000` when
            // `$A21A`'s high nibble is set.
            let (graphics, table) = match character {
                Character::Mario => (0x2A, 0x2C),
                Character::Luigi => (0x2B, 0x2D),
            };
            let (normal, fire) = if theme == 0 {
                (0x4200, 0x5200)
            } else {
                (0x5A00, 0x6200)
            };
            Ok(PlayerGraphics {
                normal: bundle.banked(graphics, normal, 0x800)?.to_vec(),
                fire: bundle.banked(graphics, fire, 0x800)?.to_vec(),
                bank1: bundle.banked(graphics, 0x4A00, 0x800)?.to_vec(),
                mappings: mappings(bundle, table, if theme == 0 { 0x4000 } else { 0x6000 }, 242)?,
            })
        };
        Ok(Self {
            levels,
            players: [
                [player(Character::Mario, 0)?, player(Character::Mario, 1)?],
                [player(Character::Luigi, 0)?, player(Character::Luigi, 1)?],
            ],
            object_tables: tables.into_iter().map(|(_, table)| table).collect(),
            hud_tiles: bundle.banked(LEVEL_BANK, 0x7C24, 20)?.to_vec(),
            animation_rows: bundle.banked(ANIMATION_BANK, 0x6680, 0x1000)?.to_vec(),
            banks: bundle.banks.clone(),
        })
    }

    pub fn level(&self, level: u8) -> Option<&LevelGraphics> {
        self.levels.get(usize::from(level))
    }

    pub fn player(&self, character: Character, theme: u8) -> &PlayerGraphics {
        let row = match character {
            Character::Mario => 0,
            Character::Luigi => 1,
        };
        &self.players[row][usize::from(theme.min(1))]
    }

    pub fn objects(&self, level: &LevelGraphics) -> &[Vec<MappingEntry>] {
        self.object_tables
            .get(level.object_table)
            .map_or(&[], Vec::as_slice)
    }

    /// The DX bytes at a base-ROM source address, when that bank is carried.
    pub fn rom(&self, bank: u8, address: u16, length: usize) -> Option<&[u8]> {
        let start = usize::from(address.checked_sub(0x4000)?);
        self.banks
            .get(&bank)?
            .get(start..start.checked_add(length)?)
    }
}

/// `LoadMarioGFX` with DX's config byte: four VRAM0 copies, then the VRAM1,
/// attribute, palette and HUD tables DX indexes by level.
fn level_graphics(bundle: &Bundle, level: u8) -> Result<LevelGraphics, ImportError> {
    let index = usize::from(level);
    let config = bundle.level_config[index];
    let set = config >> 4;
    if set > 3 {
        return Err(ImportError::InvalidBundle(format!(
            "level {level} graphics set {set} is outside the $0763 table"
        )));
    }
    let word = |bytes: &[u8], at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let mut vram0 = vec![0; 0x1000];
    for (bank, address, destination, length) in [
        (
            EXTRA_BANK,
            word(&bundle.extra_pointers, usize::from(set) * 2),
            0x680,
            0x380,
        ),
        (SHARED_BANK, 0x7000 + u16::from(set) * 0x300, 0, 0x300),
        (
            bundle.enemy_sets[index * 4],
            word(&bundle.enemy_sets, index * 4 + 2),
            0x300,
            0x380,
        ),
        (
            bundle.tile_sets[index * 4],
            word(&bundle.tile_sets, index * 4 + 2),
            0xA00,
            0x600,
        ),
    ] {
        vram0[destination..destination + length]
            .copy_from_slice(bundle.banked(bank, address, length)?);
    }
    // `$24:$795E`: three levels have their own VRAM1 block.
    let extra = match level {
        0x09 => 0x4D00,
        0x0C => 0x5A00,
        0x0D => 0x6700,
        _ => 0x4000,
    };
    let mut vram1 = bundle.banked(HUD_BANK, 0x5D00, 0x300)?.to_vec();
    vram1.extend_from_slice(bundle.banked(LEVEL_BANK, extra, 0xD00)?);
    let palette = 0x4000 + u16::from(level) * 0x40;
    let bg_palettes = bundle.banked(PALETTE_BANK, palette, 0x40)?.to_vec();
    let obj_palettes = bundle
        .banked(PALETTE_BANK, palette + 0x1000, 0x40)?
        .to_vec();
    CgbPalettes::from_bytes(&bg_palettes, &obj_palettes)
        .map_err(|e| ImportError::InvalidBundle(format!("level {level} palettes: {e}")))?;
    let hud = bundle.banked(LEVEL_BANK, 0x6000 + u16::from(level) * 2, 2)?;
    let hud = u16::from_le_bytes([hud[0], hud[1]]);
    Ok(LevelGraphics {
        background: [vram0, vram1],
        tile_attributes: bundle
            .banked(ATTRIBUTE_BANK, 0x4000 + u16::from(level) * 0x100, 0x100)?
            .to_vec(),
        bg_palettes,
        obj_palettes,
        hud_attributes: bundle.banked(LEVEL_BANK, hud, 20)?.to_vec(),
        // `$0669`: config nibbles 0 and 1 keep the normal theme; others set `$10`.
        theme: u8::from(!matches!(config & 0x0F, 0 | 1)),
        object_table: 0,
    })
}

/// A table of little-endian pointers inside `bank`, each to `[y, x, tile,
/// attr]` cells ended by `$80`.
pub(crate) fn mappings(
    bundle: &Bundle,
    bank: u8,
    table: u16,
    count: usize,
) -> Result<Vec<Vec<MappingEntry>>, ImportError> {
    (0..count)
        .map(|i| {
            let at = bundle.banked(bank, table + i as u16 * 2, 2)?;
            let pointer = u16::from_le_bytes([at[0], at[1]]);
            let mut result = Vec::new();
            for n in 0..=64u16 {
                let address = pointer
                    .checked_add(n * 4)
                    .ok_or(ImportError::InvalidPointer {
                        bank,
                        address: pointer,
                    })?;
                if bundle.banked(bank, address, 1)?[0] == 0x80 {
                    return Ok(result);
                }
                if n == 64 {
                    break;
                }
                let c = bundle.banked(bank, address, 4)?;
                result.push(MappingEntry {
                    y: c[0] as i8,
                    x: c[1] as i8,
                    tile: c[2],
                    attr: c[3],
                });
            }
            Err(ImportError::InvalidBundle(format!(
                "unterminated sprite mapping {i} in bank ${bank:02X}"
            )))
        })
        .collect()
}
