//! Level data structures for Super Mario Land 2: headers, 16x16 metatiles and
//! the 256x48 level layout, plus the `LevelManifest` schema emitted by the
//! `sml2-extract` tool.
//!
//! ## Formats (verified against the ROM + the RGBDS v1.0.4 disassembly)
//!
//! * **Level header** — 20 bytes per level (`levels/levelheaders.asm`, 32
//!   entries). Selected by `sCurLevel` ($A269): `Level_Header_Pointers[sCurLevel]`
//!   is copied to WRAM `$A800-$A813` by `$5579`. Field order:
//!   Y lo/hi, X lo/hi, cam Y lo/hi, cam X lo/hi, unknown pair, level number,
//!   tileset-map pointer lo/hi, level data bank, music, BG palette,
//!   OBJ palette 0, OBJ palette 1, (layout index | scene state), time limit.
//! * **16x16 metatile** — four 8x8 VRAM tile numbers ordered
//!   `[top-left, top-right, bottom-left, bottom-right]`. Proof: the VBlank
//!   consumer `ScrollLevelMap` (home.asm $0AFB) writes the four staged tile
//!   bytes to `dest`, `dest+1`, `dest+$20`, `dest+$21`.
//! * **Metatile table** ("tileset mapping") — 128 metatiles x 4 bytes = 512
//!   bytes (`tilemaps/mappings/NN.bin`, bank $08 `Level_TileMap_NN`), loaded
//!   to WRAM `$A600-$A7FF` at level load; lookup is `table[id*4 + quadrant]`.
//!   Tile numbers index background VRAM as seen through LCDC bit 4 = 0:
//!   `< 128` reads `$9000 + t*16`, `>= 128` reads `$8800 + (t-128)*16`.
//! * **Level layout** — 256 columns x 48 rows, one metatile id per 16x16
//!   cell, row-major with a 256-byte row stride. `Read_Level_Data` (home.asm
//!   $0386) RLE-decompresses exactly `$3000` bytes into `$B000-$DFFF`; the
//!   block pipelines address it as `$B000 + row*$100 + col`.
//! * **World geometry** — one cell = 16x16 px. The 160x144 screen shows the
//!   world centred on the camera: `screen_top_left = (camera_x - 80,
//!   camera_y - 72)`. Live-verified: an offline render of (layout + metatile
//!   table + VRAM tiles) matched PyBoy screenshots at 94-97% exact pixels
//!   across a walk, the remainder being sprites.

use serde::{Deserialize, Serialize};

use crate::collision;

/// Level layout width in metatile cells.
pub const LAYOUT_WIDTH: usize = 256;
/// Level layout height in metatile cells.
pub const LAYOUT_HEIGHT: usize = 48;
/// Full decompressed layout size in bytes (`256 * 48 == $3000`).
pub const LAYOUT_SIZE: usize = LAYOUT_WIDTH * LAYOUT_HEIGHT;
/// Metatiles per metatile table (`512 / 4`).
pub const METATILE_COUNT: usize = 128;

/// Schema id written into every `*.manifest.json`.
pub const MANIFEST_SCHEMA: &str = "sml2-level-manifest/4";
/// Schema id written into `index.json`.
pub const INDEX_SCHEMA: &str = "sml2-levels-index/4";

/// A single 16x16 metatile: four 8x8 tile numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metatile {
    /// `[top-left, top-right, bottom-left, bottom-right]` tile numbers.
    pub tiles: [u8; 4],
}

/// The 128 metatile definitions used by one level (its "tileset mapping").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetatileTable {
    pub metatiles: Vec<Metatile>,
}

impl MetatileTable {
    /// Parse a 512-byte table (`128 * 4` bytes).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != METATILE_COUNT * 4 {
            return Err(format!(
                "metatile table: expected {} bytes, got {}",
                METATILE_COUNT * 4,
                bytes.len()
            ));
        }
        let mut metatiles = Vec::with_capacity(METATILE_COUNT);
        for i in 0..METATILE_COUNT {
            let b = &bytes[i * 4..i * 4 + 4];
            metatiles.push(Metatile {
                tiles: [b[0], b[1], b[2], b[3]],
            });
        }
        Ok(Self { metatiles })
    }

    /// Definition of one metatile id (0..128).
    pub fn get(&self, id: u8) -> Option<&Metatile> {
        self.metatiles.get(id as usize)
    }
}

/// A decompressed level layout: metatile ids, row-major, matching the ROM's
/// `$B000 + y*$100 + x` addressing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelLayout {
    pub cells: Vec<u8>,
}

impl LevelLayout {
    /// Parse a fully decompressed layout (exactly `$3000` bytes).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != LAYOUT_SIZE {
            return Err(format!(
                "layout: expected {LAYOUT_SIZE} bytes, got {}",
                bytes.len()
            ));
        }
        Ok(Self {
            cells: bytes.to_vec(),
        })
    }

    /// Metatile id at cell (x, y); `None` when out of bounds.
    pub fn get(&self, x: usize, y: usize) -> Option<u8> {
        if x < LAYOUT_WIDTH && y < LAYOUT_HEIGHT {
            Some(self.cells[y * LAYOUT_WIDTH + x])
        } else {
            None
        }
    }

    /// Sorted list of metatile ids that appear anywhere in the layout.
    pub fn distinct_metatile_ids(&self) -> Vec<u8> {
        let mut seen = [false; 256];
        for &c in &self.cells {
            seen[c as usize] = true;
        }
        (0..=255u8).filter(|&i| seen[i as usize]).collect()
    }

    /// The most frequent metatile id (usually the sky/background filler).
    pub fn most_common_metatile(&self) -> MetatileCount {
        let mut counts = [0u32; 256];
        for &c in &self.cells {
            counts[c as usize] += 1;
        }
        let (id, &count) = counts.iter().enumerate().max_by_key(|&(_, &c)| c).unwrap();
        MetatileCount {
            id: id as u8,
            count,
        }
    }
}

/// A metatile id and how often it occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetatileCount {
    pub id: u8,
    pub count: u32,
}

/// Decoded level header (the 20 bytes copied to `$A800-$A813`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelHeader {
    /// Header byte +10 ("Level Number"; equals `sCurLevel`).
    pub level_id: u8,
    /// Mario spawn X in pixels (header +2/+3).
    pub mario_spawn_x: u16,
    /// Mario spawn Y in pixels (header +0/+1).
    pub mario_spawn_y: u16,
    /// Starting camera X in pixels (header +6/+7).
    pub camera_start_x: u16,
    /// Starting camera Y in pixels (header +4/+5).
    pub camera_start_y: u16,
    /// Header bytes +8/+9: a write-only latch pair in WRAM (`$A808/$A809`),
    /// kept raw (no reader found; see the WP6 handoff).
    pub unknown: [u8; 2],
    /// Bank holding the level's RLE layout data (header +13).
    pub level_data_bank: u8,
    /// Music track id (header +14).
    pub music_track: u8,
    /// `[BG palette, OBJ palette 0, OBJ palette 1]` (header +15..+17).
    pub palettes: [u8; 3],
    /// Header +18 low nibble: index of the level's 48-byte scrollbox/edge-map
    /// record (see `$03F1`: `source = [bank:$4000] + 48 * layout_index`).
    pub layout_index: u8,
    /// Header +18 high nibble: initial `sSceneState`.
    pub scene_state: u8,
    /// Header +19: time limit, displayed in-game as `raw * 100`.
    pub time_limit: u8,
}

impl LevelHeader {
    /// Decode the 20 raw header bytes.
    pub fn from_bytes(b: &[u8; 20]) -> Self {
        Self {
            level_id: b[10],
            mario_spawn_x: u16::from_le_bytes([b[2], b[3]]),
            mario_spawn_y: u16::from_le_bytes([b[0], b[1]]),
            camera_start_x: u16::from_le_bytes([b[6], b[7]]),
            camera_start_y: u16::from_le_bytes([b[4], b[5]]),
            unknown: [b[8], b[9]],
            level_data_bank: b[13],
            music_track: b[14],
            palettes: [b[15], b[16], b[17]],
            layout_index: b[18] & 0x0F,
            scene_state: b[18] & 0xF0,
            time_limit: b[19],
        }
    }
}

/// Per-level byte tables from home.asm (`LevelProperties*`, 32 entries each).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelProperties {
    /// `LevelPropertiesUNK` ($1F51) — unknown flag byte.
    pub unknown: u8,
    /// `LevelPropertiesScroll` ($1F71) — auto-scroll flag.
    pub auto_scroll: u8,
    /// `LevelPropertiesSpace` ($1F91) — Space-zone physics flag.
    pub space_physics: u8,
}

/// One object-spawn record: a 6-byte record as it lands in the `$AB06` stream
/// buffer (the `$EBBB` expander turns 3-byte packs into these).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnRecord {
    /// World X (record bytes 0/1; low byte is 8-aligned).
    pub x: u16,
    /// World Y (record bytes 3/4; low byte is 8-aligned).
    pub y: u16,
    /// Object/anim index (record byte 5; copied to slot+5, staged `$FFD5`).
    pub anim_index: u8,
    /// Record byte 2: 0 = spawns in both modes; 1 = skipped in Easy Mode
    /// (normal gate: `(flag & $0E) == 0`, easy gate: `(flag & $0F) == 0`).
    /// The game writes 2 here once the record has spawned.
    pub mode_flag: u8,
    /// Sprite id resolved from `anim_index` through the mode's anim->sprite
    /// table (see `object.rs`); what `$FFDB` becomes in-game.
    pub sprite_id: u8,
    /// Collision extents of the sprite: `[x0, x1, y0, y1]` signed offsets
    /// (slot+19..+22 in-game).
    pub hitbox: [i8; 4],
    /// The 6 expanded bytes as stored in the `$AB06` buffer.
    pub raw: [u8; 6],
}

/// One level's object-spawn stream: the pointer table lives in bank `$03`
/// (`UnknownData_0xE037`, indexed by `sCurLevel`), the packed source is
/// expanded to `$AB06` at level load by `$EBBB` and consumed by the spawn
/// scan (`$8C31`), which matches records against the camera-driven cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectStream {
    pub table_label: String,
    pub table_bank: u8,
    /// ROM offset of the packed source (3-byte packs).
    pub stream_rom: u32,
    /// Packed length in bytes, including the `$FF` terminator.
    pub packed_len: u32,
    pub records: Vec<SpawnRecord>,
}

/// Reference to the level's metatile table (bank $08).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetMapRef {
    /// 1-based table number (the `NN` in `Level_TileMap_NN`).
    pub index: u8,
    pub label: String,
    pub file: String,
}

/// Reference to the level's background tileset graphics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphicsRef {
    /// Label, e.g. `GFX_TilesetTreeZone1`.
    pub label: String,
    pub bank: u8,
    /// Extracted 2bpp file, e.g. `gfx/tilesets/treezone1.2bpp`.
    pub file: String,
}

/// Summary of the level's decompressed layout (the bytes themselves live in
/// `file`, `levelNN.decompressed.bin` next to the manifest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutSummary {
    pub file: String,
    pub width: u16,
    pub height: u16,
    pub size: u32,
    pub distinct_metatile_ids: Vec<u8>,
    pub most_common: MetatileCount,
}

/// Tile animation config for one level. The engine loads
/// `UnknownData_0x35EA[sCurLevel]`, stores `value + 1` in `sTileAnimConfig`
/// (config 0 = animation off) and then animates 4 tiles at VRAM `$9200` by
/// copying 64 bytes from bank $1D `$6680 + row*$100 + phase*$40` (phases
/// 0..3) whenever `sTileAnimTimer` reaches `speed * 2` (timer = frame count).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileAnim {
    /// Raw table value (`$FF` -> config 0 -> no animation).
    pub table_value: u8,
    pub enabled: bool,
    /// Row into the bank-$1D frame data (`$6680 + row*$100`).
    pub row: u8,
    /// Frames advance when the frame timer reaches `speed * 2` (0 = fastest).
    pub speed: u8,
}

impl TileAnim {
    /// Decode a raw `UnknownData_0x35EA` entry.
    pub fn from_table_value(v: u8) -> Self {
        let config = v.wrapping_add(1);
        Self {
            table_value: v,
            enabled: config != 0,
            row: (config.wrapping_sub(1)) & 0x0F,
            speed: (config & 0xF0) >> 4,
        }
    }
}

/// The in-level status bar: a 20-tile row rendered by the window layer at the
/// bottom of the screen. The engine copies `template` (ROM $07EA) to $9C00
/// (window map) via `$07DB` and patches digit tiles (`$80 + value`) at
/// `digit_columns` every frame (`$076B`): lives at 2-3, coins at 7-9, money at
/// 13-14, time at 17-19; the window sits at rWX=7 / rWY=136 during gameplay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HudStrip {
    pub template: Vec<u8>,
    pub window_x: u8,
    pub window_y: u8,
    /// Map columns patched with digits, in display order (lives, coins,
    /// money, time).
    pub digit_columns: Vec<u8>,
}

/// A complete structured description of one level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelManifest {
    pub schema: String,
    /// Game level number 0..31 (equals `sCurLevel`).
    pub level_id: u8,
    /// Source label, e.g. `Level_Header_01` (label NN = `level_id` + 1).
    pub header_label: String,
    /// The 20 raw header bytes as stored in the ROM (lowercase hex).
    pub header_raw_hex: String,
    pub header: LevelHeader,
    /// Label of the level's RLE layout data, e.g. `Level_01`.
    pub level_data_label: String,
    pub tileset_map: TilesetMapRef,
    pub graphics: GraphicsRef,
    pub enemy_graphics: GraphicsRef,
    pub metatiles: MetatileTable,
    /// Collision class of every metatile id, indexed by id (0..128). The
    /// ranges are global engine semantics, live-verified; see
    /// `collision.rs`.
    pub metatile_classes: Vec<collision::TileClass>,
    /// `sSpriteMapIndex` this level loads with (0 = anims 0..95 / 168 sprite
    /// mappings, 1 = anims 0..111 / 228 mappings; level id > $0A selects 1).
    pub sprite_map_index: u8,
    /// Tile animation (`$9200` block) config for this level.
    pub tile_anim: TileAnim,
    pub layout: LayoutSummary,
    pub properties: LevelProperties,
    pub spawns: ObjectStream,
}

/// Index of all levels + metatile-table usage across the ROM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelsIndex {
    pub schema: String,
    pub levels: Vec<IndexLevel>,
    pub tileset_maps: Vec<IndexTilesetMap>,
    pub notes: Vec<String>,
}

/// One level's entry in `index.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexLevel {
    pub level_id: u8,
    pub header_label: String,
    pub tileset_map_index: u8,
    pub graphics_file: String,
    pub layout_file: String,
    /// Number of object-spawn records in the level's stream.
    pub spawn_count: u32,
}

/// Usage of one bank-$08 metatile table (`Level_TileMap_NN`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexTilesetMap {
    pub index: u8,
    pub file: String,
    /// Level ids whose header references this table.
    pub used_by_levels: Vec<u8>,
    /// `sLevelGfxIndex` values selecting this table through the boss/special
    /// GFX path (`$25AF` + the `$26B5` pointer table); empty for tables 1-24.
    pub boss_gfx_indices: Vec<u8>,
}
