//! Every level's DX VRAM, attributes, palettes and mappings, derived from a
//! [`Bundle`] by the DX ROM's own level loader. See docs/sml2dx-audit.md,
//! "All levels".

use crate::{
    bundle::{Bundle, HEADER_LEN, LEVELS},
    ImportError,
};
use sml2_core::{
    character::Character,
    level::{MetatileTable, LAYOUT_SIZE},
    map::{BellRoomTiles, OverworldManifest, OverworldTilemaps, OverworldTiles, RoomCommonTiles},
    object::{sprite_map_index, MappingEntry},
    room_sprites::RoomSpriteTable,
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
    /// DX's `Level_TileMap` for the level: which tiles build each metatile.
    pub metatiles: MetatileTable,
    /// DX's decompressed layout, `$B000`-addressed like the native one.
    pub layout: Vec<u8>,
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

/// A front-end screen's CGB background, as its loader leaves VRAM: both banks
/// of `$8000..$97FF`, the `$9800` tile map and its VRAM1 attribute map, and
/// both palette sets.
#[derive(Clone, Debug)]
pub struct Screen {
    pub vram: [Vec<u8>; 2],
    pub tilemap: Vec<u8>,
    pub attributes: Vec<u8>,
    pub bg_palettes: Vec<u8>,
    pub obj_palettes: Vec<u8>,
}

/// One overworld screen's DX colors (`$26:$77F1` for the hub, the
/// `$26:$7969`.. branches for the zones).
#[derive(Clone, Debug)]
pub struct MapScreen {
    pub bg_palettes: Vec<u8>,
    /// The hub's second set, used once its camera reaches X `$E7`.
    pub alternate_bg_palettes: Option<Vec<u8>>,
    /// `[Mario, Luigi]`, by `$FFF6`.
    pub obj_palettes: [Vec<u8>; 2],
    /// A zone's 32x18 VRAM1 attribute map (`$29:$6642`); the hub colors
    /// tiles by ID instead.
    pub attributes: Option<Vec<u8>>,
    /// Space Zone's map once `sMapSpacePath0` bit 0 is set.
    pub alternate_attributes: Option<Vec<u8>>,
}

/// The overworld's sprite tables in the native tables' shapes: DX keeps their
/// layout and edits their attribute bytes, adding CGB palettes.
#[derive(Clone, Debug)]
pub struct MapSprites {
    /// `$0F:$4C4D`: the hub's 17 decor metasprites.
    pub hub_decor: Vec<Vec<(i8, i8, u8, u8)>>,
    /// `$18:$62E6`: the zones' 188 decor metasprites.
    pub zone_decor: Vec<Vec<(i8, i8, u8, u8)>>,
    /// `$0F:$7251`: the hub status bar's nine OAM entries.
    pub status_bar: [(u8, u8, u8, u8); 9],
    /// `$18:$51B1`: both minigames' decoration metasprites. DX moves one
    /// of them to `$5919`, past the native table's end.
    pub room_metasprites: RoomSpriteTable,
}

/// The overworld, built by the native extractor's builders from the DX banks
/// (DX edits the tiles, maps, patches and poses in place) plus DX's colors.
#[derive(Clone, Debug)]
pub struct MapGraphics {
    pub tilemaps: OverworldTilemaps,
    pub manifest: OverworldManifest,
    pub tiles: OverworldTiles,
    pub zone_animation_tiles: Vec<u8>,
    /// `$29:$6542`: the hub's attribute for each tile ID, loaded to `$D000`.
    pub hub_attributes: Vec<u8>,
    /// Native screens 0 (hub) through 14 (both minigames).
    pub screens: Vec<MapScreen>,
    /// Bank 25 `$5000`, the tiles both bell rooms copy to `$8800`.
    pub bell_tiles: BellRoomTiles,
    /// Bank 6 `$4000`, the 128 tiles both minigames copy to `$8000`.
    pub room_tiles: RoomCommonTiles,
    pub sprites: MapSprites,
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
    /// `$20:$6A40`'s title screen.
    pub title: Screen,
    /// `$2B8B` plus `$25:$4234`'s file select.
    pub file_select: Screen,
    pub map: MapGraphics,
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
const METATILE_BANK: u8 = 0x08;
const MAP_BANK: u8 = 0x29;

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
            title: title(bundle)?,
            file_select: file_select(bundle)?,
            map: map(bundle)?,
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
    let header = &bundle.level_headers[index * HEADER_LEN..][..HEADER_LEN];
    let metatiles = MetatileTable::from_bytes(bundle.banked(
        METATILE_BANK,
        u16::from_le_bytes([header[0x0B], header[0x0C]]),
        0x200,
    )?)
    .map_err(ImportError::InvalidBundle)?;
    let layout = layout(bundle, header[0x0D], header[0x12] & 0x0F)?;
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
        metatiles,
        layout,
    })
}

/// Copy `(bank, address, length)` sources into a `$8000`-based VRAM image at
/// the given offsets.
fn vram(bundle: &Bundle, copies: &[(u8, u16, usize, usize)]) -> Result<Vec<u8>, ImportError> {
    let mut vram = vec![0; 0x1800];
    for &(bank, address, length, at) in copies {
        vram[at..at + length].copy_from_slice(bundle.banked(bank, address, length)?);
    }
    Ok(vram)
}

/// A `$9800` attribute map: the stored rows, then zeros.
fn attribute_map(
    bundle: &Bundle,
    bank: u8,
    address: u16,
    length: usize,
) -> Result<Vec<u8>, ImportError> {
    let mut map = bundle.banked(bank, address, length)?.to_vec();
    map.resize(0x400, 0);
    Ok(map)
}

fn palettes(bundle: &Bundle, bg: u16, obj: u16) -> Result<(Vec<u8>, Vec<u8>), ImportError> {
    let bg = bundle.banked(PALETTE_BANK, bg, 0x40)?.to_vec();
    let obj = bundle.banked(PALETTE_BANK, obj, 0x40)?.to_vec();
    CgbPalettes::from_bytes(&bg, &obj).map_err(|e| ImportError::InvalidBundle(e.to_string()))?;
    Ok((bg, obj))
}

/// `$20:$6A40`: VRAM0 `$8800` from `$20:$4000`, VRAM1 `$8800`/`$8B00` from
/// `$20:$5D00`/`$5000`, attributes `$20:$6800`, palettes `$26:$7870`/`$78B0`;
/// the unchanged `$05:$59E5` map, which DX edits in place.
fn title(bundle: &Bundle) -> Result<Screen, ImportError> {
    let (bg_palettes, obj_palettes) = palettes(bundle, 0x7870, 0x78B0)?;
    Ok(Screen {
        vram: [
            vram(bundle, &[(0x20, 0x4000, 0x1000, 0x800)])?,
            vram(
                bundle,
                &[(0x20, 0x5D00, 0x300, 0x800), (0x20, 0x5000, 0xD00, 0xB00)],
            )?,
        ],
        tilemap: bundle.banked(0x05, 0x59E5, 0x400)?.to_vec(),
        attributes: attribute_map(bundle, 0x20, 0x6800, 0x240)?,
        bg_palettes,
        obj_palettes,
    })
}

/// `$2B8B`'s unchanged VRAM0 copies and the `$0C:$4A52` map, DX-edited in
/// place; `$25:$4234`'s attributes from `$25:$4000` and BG palettes
/// `$26:$7D17`. VRAM1 and the OBJ palettes are what the title and the
/// player's VRAM1 load (`$20:$6000`) leave.
fn file_select(bundle: &Bundle) -> Result<Screen, ImportError> {
    let (bg_palettes, obj_palettes) = palettes(bundle, 0x7D17, 0x78B0)?;
    Ok(Screen {
        vram: [
            vram(
                bundle,
                &[
                    (0x06, 0x4000, 0x800, 0),
                    (0x1B, 0x7000, 0x300, 0x800),
                    (0x0C, 0x6ACA, 0x380, 0xB00),
                    (0x07, 0x6A00, 0x380, 0xE80),
                    (0x07, 0x5E00, 0x600, 0x1200),
                ],
            )?,
            vram(
                bundle,
                &[
                    (0x20, 0x6000, 0x800, 0),
                    (0x20, 0x5D00, 0x300, 0x800),
                    (0x20, 0x5000, 0xD00, 0xB00),
                ],
            )?,
        ],
        tilemap: bundle.banked(0x0C, 0x4A52, 0x400)?.to_vec(),
        attributes: attribute_map(bundle, 0x25, 0x4000, 0x234)?,
        bg_palettes,
        obj_palettes,
    })
}

/// The DX banks laid out at their ROM offsets, for builders that read fixed
/// addresses.
fn rom_image(bundle: &Bundle) -> Vec<u8> {
    let last = bundle.banks.keys().max().copied().unwrap_or(0);
    let mut rom = vec![0; (usize::from(last) + 1) * crate::bundle::BANK_LEN];
    for (&bank, bytes) in &bundle.banks {
        let at = usize::from(bank) * crate::bundle::BANK_LEN;
        rom[at..at + bytes.len()].copy_from_slice(bytes);
    }
    rom
}

fn map(bundle: &Bundle) -> Result<MapGraphics, ImportError> {
    use sml2_extract::map as extract;
    let bad = |what: &str, e: String| ImportError::InvalidBundle(format!("overworld {what}: {e}"));
    let rom = rom_image(bundle);
    let tilemaps = extract::build_tilemaps_binary(&rom).map_err(|e| bad("maps", e))?;
    let tiles = extract::build_tiles_binary(&rom).map_err(|e| bad("tiles", e))?;
    let manifest = extract::build_manifest(&rom).map_err(|e| bad("patches", e))?;
    let palette = |address: u16| -> Result<Vec<u8>, ImportError> {
        Ok(bundle.banked(PALETTE_BANK, address, 0x40)?.to_vec())
    };
    let zone = |bg: u16, obj: u16, attributes: u16| -> Result<MapScreen, ImportError> {
        Ok(MapScreen {
            bg_palettes: palette(bg)?,
            alternate_bg_palettes: None,
            obj_palettes: [palette(obj)?, palette(obj + 0x40)?],
            attributes: Some(bundle.banked(MAP_BANK, attributes, 0x240)?.to_vec()),
            alternate_attributes: None,
        })
    };
    // Native screen order: hub, Pumpkin, Tree, Macro, Space, Turtle, Mario,
    // Wario's castle, the castle hill. Each zone branch names its palettes
    // and the `$29:$6642` map for its index.
    let mut screens = vec![
        MapScreen {
            bg_palettes: palette(0x6380)?,
            alternate_bg_palettes: Some(palette(0x63C0)?),
            obj_palettes: [palette(0x6400)?, palette(0x6440)?],
            attributes: None,
            alternate_attributes: None,
        },
        zone(0x6780, 0x67C0, 0x4B40)?,
        zone(0x6480, 0x64C0, 0x4000)?,
        zone(0x6600, 0x6640, 0x4240)?,
        zone(0x66C0, 0x6700, 0x46C0)?,
        zone(0x6540, 0x6580, 0x4480)?,
        zone(0x6840, 0x6880, 0x4D80)?,
        zone(0x69C0, 0x6A40, 0x5200)?,
        zone(0x6900, 0x6940, 0x4FC0)?,
    ];
    screens[4].alternate_attributes = Some(bundle.banked(MAP_BANK, 0x4900, 0x240)?.to_vec());
    // Screens 9–14, in order. `$26:$765A` selects the coin room from the
    // door cursor's X (`$A101`): `$70` is screen 9, `$58` screen 10, `$40`
    // screen 11 and `$28` screen 12. Bell rooms are `$7C91` (13) and
    // `$78F0` (14). Attribute maps are bank `$37`; `$7D57` copies `$240`
    // bytes to VRAM1 `$9800`. Coin rooms share the OBJ set at `$761A`.
    let mini = |bg: u16, obj: u16, attributes: u16| -> Result<MapScreen, ImportError> {
        Ok(MapScreen {
            bg_palettes: palette(bg)?,
            alternate_bg_palettes: None,
            obj_palettes: [palette(obj)?, palette(obj)?],
            attributes: Some(bundle.banked(0x37, attributes, 0x240)?.to_vec()),
            alternate_attributes: None,
        })
    };
    screens.extend([
        mini(0x75DA, 0x761A, 0x7340)?,
        mini(0x759A, 0x761A, 0x7100)?,
        mini(0x755A, 0x761A, 0x6EC0)?,
        mini(0x751A, 0x761A, 0x6C80)?,
        mini(0x745A, 0x74DA, 0x6800)?,
        mini(0x749A, 0x74DA, 0x6A40)?,
    ]);
    for screen in &screens {
        for obj in &screen.obj_palettes {
            CgbPalettes::from_bytes(&screen.bg_palettes, obj)
                .map_err(|e| bad("palettes", e.to_string()))?;
        }
    }
    Ok(MapGraphics {
        tilemaps: OverworldTilemaps::from_bytes(&tilemaps)
            .ok_or_else(|| bad("maps", "wrong length".into()))?,
        manifest: serde_json::from_value(manifest).map_err(|e| bad("patches", e.to_string()))?,
        tiles: OverworldTiles::from_bytes(&tiles)
            .ok_or_else(|| bad("tiles", "wrong length".into()))?,
        zone_animation_tiles: extract::build_zone_animation_tiles(&rom)
            .map_err(|e| bad("animation", e))?,
        hub_attributes: bundle.banked(MAP_BANK, 0x6542, 0x100)?.to_vec(),
        screens,
        bell_tiles: BellRoomTiles::from_bytes(bundle.banked(0x19, 0x5000, 0x1000)?)
            .ok_or_else(|| bad("bell tiles", "wrong length".into()))?,
        room_tiles: RoomCommonTiles::from_bytes(bundle.banked(0x06, 0x4000, 0x0800)?)
            .ok_or_else(|| bad("room tiles", "wrong length".into()))?,
        sprites: MapSprites {
            hub_decor: metasprites(bundle, 0x0F, 0x4C4D, 17)?,
            zone_decor: metasprites(bundle, 0x18, 0x62E6, 188)?,
            room_metasprites: RoomSpriteTable::new(
                0x51B1,
                bundle.banked(0x18, 0x51B1, 0x593D - 0x51B1)?,
            ),
            status_bar: {
                let bytes = bundle.banked(0x0F, 0x7251, 36)?;
                std::array::from_fn(|i| {
                    let c = &bytes[i * 4..i * 4 + 4];
                    (c[0], c[1], c[2], c[3])
                })
            },
        },
    })
}

/// A pointer table of `(dy, dx, tile, attr)` metasprites, each ended by a
/// `$FF` tile byte (`$3D01D` and `$509B` test the tile, not `dy`).
fn metasprites(
    bundle: &Bundle,
    bank: u8,
    table: u16,
    count: u16,
) -> Result<Vec<Vec<(i8, i8, u8, u8)>>, ImportError> {
    (0..count)
        .map(|i| {
            let at = bundle.banked(bank, table + i * 2, 2)?;
            let mut address = u16::from_le_bytes([at[0], at[1]]);
            let mut cells = Vec::new();
            loop {
                let c = bundle.banked(bank, address, 4)?;
                if c[2] == 0xFF {
                    return Ok(cells);
                }
                if cells.len() == 64 {
                    return Err(ImportError::InvalidBundle(format!(
                        "unterminated metasprite {i} at ${bank:02X}:${table:04X}"
                    )));
                }
                cells.push((c[0] as i8, c[1] as i8, c[2], c[3]));
                address = address.wrapping_add(4);
            }
        })
        .collect()
}

/// `$0361`: the layout `$42F0[index]` names in `bank`, run-length decoded
/// (`$0386`) to `$3000` bytes. A run byte has bit 7 set and is followed by a
/// count (0 = 256) of extra copies.
fn layout(bundle: &Bundle, bank: u8, index: u8) -> Result<Vec<u8>, ImportError> {
    let at = bundle.banked(bank, 0x42F0 + u16::from(index) * 2, 2)?;
    let mut address = u16::from_le_bytes([at[0], at[1]]);
    let mut next = || -> Result<u8, ImportError> {
        let byte = bundle.banked(bank, address, 1)?[0];
        address = address.wrapping_add(1);
        Ok(byte)
    };
    let mut out = Vec::with_capacity(LAYOUT_SIZE);
    while out.len() < LAYOUT_SIZE {
        let byte = next()?;
        out.push(byte & 0x7F);
        if byte & 0x80 != 0 {
            let count = match next()? {
                0 => 256,
                n => usize::from(n),
            };
            let room = LAYOUT_SIZE - out.len();
            out.extend(std::iter::repeat_n(byte & 0x7F, count.min(room)));
        }
    }
    Ok(out)
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
