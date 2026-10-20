//! The overworld's node tables, and the save record the hand-off writes.
//!
//! ## What a node is
//!
//! The overworld is 130 nodes (`$00`-`$81`) and a node's data is a set of
//! *parallel arrays* indexed by node id — there is no packed record. Two of them
//! live in bank 15 and are extracted together as one 260-byte blob
//! (`extracted/map/nodes.bin`, level index first, then progress slot) because
//! every consumer indexes both by the same node:
//!
//! * `$4218[node]` — the level index the map stores into `sCurLevel` when the
//!   node is entered. `$FE` (and `$FF` at nodes `$6B`/`$6C`) means the node has
//!   no level: it is a pure path point. 32 of the 130 nodes carry a level.
//! * `$650E[node]` — the node's *progress slot*: the offset the hand-off's
//!   `$3EEF0` adds to `sLevelProgress` (`$A848`) to reach that node's
//!   cleared/secret flags, and the index into the key-mask table at `$6591`.
//!   34 of the 130 nodes carry one, and the six castle slots (`$05`/`$03`/
//!   `$02`/`$01`/`$04`) sit at nodes `$20`-`$24`.
//! * `$6591[slot]` — the key/direction mask. `$3E1D7` (the map's entry gate)
//!   reads it by *progress slot* and tests it against `hKeysHeld & $F0`, so a
//!   node only accepts the directions its mask offers. Bank 15's `$3C33B` indexes
//!   the same table by *level index* instead; that discrepancy is recorded here,
//!   not resolved — the second site only feeds the zone screens' arrow drawing.
//!   The disassembly labels nothing between `$650E` and `$67BE`, so this table
//!   has no declared length: it sits immediately after the slot table (`$650E` +
//!   130 = `$6591`) and the port takes the 42 bytes the ROM's own slot range
//!   (`$00`-`$29`) addresses.
//!
//! Node 0 — the level this port's captures all play, level 1 — is `0` in both,
//! which is exactly the case the `$14` hand-off's tail keys on.
//!
//! ## The save record
//!
//! `$2934` writes 0x50 bytes to `sSaveSlot0` (`$A000`) + `sSaveFileIndex` * `$50`:
//! the 64-byte image at `$A840` (`sMapNode`, the cursor, `sLevelProgress`, the
//! golden-coin flags, the map's gates), then `sCoinLow`, `sCoinHigh`,
//! `sLevelTally`, `sLives`, `sMoneyBagCount`, and finally the running sum of all
//! 69 bytes as a one-byte checksum. The five signature bytes at +$46 are *not*
//! written by it — they persist from the fresh-file init, and `$28EC` validates
//! a slot by checking that sum and then those bytes (`$12 $34 $56 $78 $78`).
//! [`PlayerSim::save_record`] builds the image; [`save_record_valid`] checks it.

/// `$3EEA4`: the per-node camera/cursor record — the map camera a node centres on
/// and where the cursor sits on it.
///
/// The record is `$5EE6 + node*8` in bank 24 (flat `$61EE6`): X hi, X lo, Y hi,
/// Y lo, then `sMapCursorX` and `sMapCursorY`. The ROM latches the two 16-bit
/// values into the map camera *staging* (`$FFBA:B9`, `$FFB8:B7`), which the pan
/// helpers move and `$3ED4C` latches into `sMapCamX/Y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MapNodeRecord {
    pub cam_x: u16,
    pub cam_y: u16,
    pub cursor_x: u8,
    pub cursor_y: u8,
}

/// The node records, as `sml2-extract` writes them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapNodeRecords {
    blob: Vec<u8>,
}

impl MapNodeRecords {
    /// Records run to node `$7F`, like the walk table: the bytes 6-7 that are
    /// `$00` in every real record are non-zero from node `$80` on.
    pub const NODES: usize = 128;
    /// Eight bytes per record.
    pub const LEN: usize = Self::NODES * 8;

    /// `None` unless the blob is exactly the extracted size.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == Self::LEN).then(|| Self {
            blob: bytes.to_vec(),
        })
    }

    /// No records loaded: [`Self::record`] answers `None`, so a node change leaves
    /// the camera and cursor where they are instead of seeding them from nothing.
    pub fn is_empty(&self) -> bool {
        self.blob.is_empty()
    }

    /// The node's record, or `None` for a node past the table with no data loaded.
    pub fn record(&self, node: u8) -> Option<MapNodeRecord> {
        if node as usize >= Self::NODES {
            return None;
        }
        let at = node as usize * 8;
        let b = self.blob.get(at..at + 6)?;
        Some(MapNodeRecord {
            cam_x: (b[0] as u16) << 8 | b[1] as u16,
            cam_y: (b[2] as u16) << 8 | b[3] as u16,
            cursor_x: b[4],
            cursor_y: b[5],
        })
    }
}

/// Offsets into `sLevelProgress` (`$A848`) of everything else the map keeps in
/// the same 64-byte block as the per-level flags: the zone *paths* and the
/// gate bytes the cursor helpers test.
///
/// These are not level progress at all — they are map state that happens to live
/// inside the save image's first 64 bytes, which is why they survive a save/load.
/// `$3C2B8`/`$3C2E7`/`$3C2FE` (`bit 7` of the Macro gate, `bit 0` of Macro path 0,
/// `bit 7` of Macro path 2) and the per-screen handlers are their readers.
pub mod progress_offset {
    /// `$A84A` Tree Zone screen A path 1 (bit 0 set on secret clear, node 37).
    pub const TREE_A_PATH_1: usize = 2;
    /// `$A84D` Tree Zone screen A gold coin (bit 7 = collected).
    pub const GOLDEN_COIN_1: usize = 5;
    /// `$A84F` Pumpkin Zone screen A path 1 (bit 0 set on node $2C secret clear).
    pub const PUMPKIN_PATH_1: usize = 7;
    /// `$A850` Pumpkin Zone screen A path 2 (bit 0 set on node $2F secret clear).
    pub const PUMPKIN_PATH_2: usize = 8;
    /// `$A851` Pumpkin Zone screen A gold coin (bit 7 = collected).
    pub const GOLDEN_COIN_2: usize = 9;
    /// `$A853` Macro Zone path 0 (bit 0 gates the `$3C2E7` cursor move).
    pub const MACRO_PATH_0: usize = 11;
    /// `$A855` Macro Zone path 2 (bit 7 gates the `$3C2B8` A-move).
    pub const MACRO_PATH_2: usize = 13;
    /// `$A858` Space Zone path 0 (bit 0 set on secret clear, node 67).
    pub const SPACE_PATH_0: usize = 16;
    /// `$A85E` Tree Zone screen B path 1 (bit 0 set on secret clear).
    pub const TREE_B_PATH_1: usize = 22;
    /// `$A865` Mario Zone gold coin (bit 7 = collected).
    pub const GOLDEN_COIN_6: usize = 29;
    /// `$A86B` Macro gate — set when node `$6A` completes.
    pub const GATE_MACRO: usize = 35;
    /// `$A86D` Mario-Zone gate — set when node `$66` completes.
    pub const GATE_MARIO: usize = 37;
    /// `$A86E` Pumpkin-Zone gate B — set when node `$78` completes.
    pub const GATE_PUMPKIN_B: usize = 38;
    /// `$A86F` Pumpkin-Zone gate C — set when node `$7A` completes.
    pub const GATE_PUMPKIN_C: usize = 39;
    /// `$A871` Space Zone gate — read-only census.
    pub const GATE_SPACE: usize = 41;
}

/// The two parallel node tables, as `sml2-extract` writes them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapNodes {
    blob: Vec<u8>,
}

impl MapNodes {
    /// Node ids run `$00`-`$81`.
    pub const NODES: usize = 130;
    /// The key-mask table's length: the slot range the ROM's own table uses
    /// runs `$00`-`$29`. See the extractor and the module docs for why this is
    /// inferred rather than labelled.
    pub const MASKS: usize = 42;
    /// The extracted blob: the level table, the progress-slot table (both 130
    /// bytes, node-indexed), then the key masks (42 bytes, slot-indexed).
    pub const BLOB_LEN: usize = 2 * Self::NODES + Self::MASKS;

    /// `None` unless the blob is exactly the extracted size.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == Self::BLOB_LEN).then(|| Self {
            blob: bytes.to_vec(),
        })
    }

    /// No table loaded: [`Self::level_for`] and [`Self::progress_slot`] answer
    /// `None`, so a hand-off cannot mark progress it cannot index.
    pub fn is_empty(&self) -> bool {
        self.blob.is_empty()
    }

    /// `$4218[node]` — the level index, or `None` for a node without one
    /// (`$FE`/`$FF`).
    pub fn level_for(&self, node: u8) -> Option<u8> {
        let v = *self.blob.get(node as usize)?;
        (v < 0xFE).then_some(v)
    }

    /// `$4218[node]` **raw**. The map's entry gate (`$3E1D7`) tests the byte for
    /// `$FE` alone and lets `$FF` through, so [`Self::level_for`] — which folds
    /// both into `None` — cannot express it.
    pub fn level_byte(&self, node: u8) -> Option<u8> {
        self.blob.get(node as usize).copied()
    }

    /// `$650E[node]` — the `sLevelProgress` offset (and `$6591` index), or
    /// `None` when the node has neither.
    pub fn progress_slot(&self, node: u8) -> Option<u8> {
        let v = *self.blob.get(Self::NODES + node as usize)?;
        (v < 0xFE).then_some(v)
    }

    /// `$6591[slot]` — the key/direction mask `$3E1D7` ANDs with
    /// `hKeysHeld & $F0`. `None` for a slot outside the table, or when no blob
    /// is loaded; the ROM reads past the table instead, which the port refuses
    /// to reproduce (see [`crate::player_sim::PlayerSim::map_can_enter`]).
    pub fn key_mask(&self, slot: u8) -> Option<u8> {
        self.blob.get(2 * Self::NODES + slot as usize).copied()
    }
}

/// Where the overworld's walk graph starts in the ROM. The cursor code does
/// `ld hl, $563A` (main.asm:18936) with bank 24 banked in, and `$563A` is a
/// bank-24 *window* address — so the table is flat `$6163A`.
pub const MAP_WALK_ROM: u32 = 0x6163A;

/// The four directions a node's walk entry holds, in the ROM's own order: the
/// entry is `MAP_WALK_ROM + node*8 + dir*2` and `$3C52B` tests the held byte's
/// bit 1 (Left) first, then bit 2 (Up), bit 3 (Down), and falls back to Right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WalkDir {
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
}

/// The overworld's walk graph, as `sml2-extract` writes it: the 128-node pointer
/// table followed by every stream its pointers reach, so a pointer resolves by
/// *offset from the blob's start*.
///
/// The graph is 128 nodes, not the 130 the other node tables carry: node `$80`
/// and `$81` read past the end of this table (their "pointers" are the data that
/// follows it), so [`Self::NODES`] stops at `$7F`.
///
/// A stream is a list of one-byte moves ended by `$00`, followed by the id of the
/// node the walk arrives at. Each move byte's low nibble is the direction
/// (`$01` R, `$02` L, `$04` U, `$08` D — and combinations, which the cursor
/// applies in the ROM's own order), bit 4 halves the step's frame budget, and
/// bits 5-7 are read only by the cursor's OAM/pose code.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapWalk {
    blob: Vec<u8>,
}

impl MapWalk {
    /// Node ids the table covers — see the type docs for why it is not 130.
    pub const NODES: usize = 128;
    /// The pointer table's length: 8 bytes (four 16-bit window pointers) per node.
    pub const TABLE_LEN: usize = Self::NODES * 8;
    /// No path in this direction.
    pub const NO_PATH: u16 = 0xFFFF;

    /// `None` unless the blob is big enough for the table; the streams after it
    /// are read on demand.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        (bytes.len() >= Self::TABLE_LEN).then(|| Self {
            blob: bytes.to_vec(),
        })
    }

    /// No graph loaded: [`Self::pointer`] and [`Self::byte_at`] answer `None`, so
    /// no direction starts a walk.
    pub fn is_empty(&self) -> bool {
        self.blob.is_empty()
    }

    /// The node's raw `dir` pointer — a bank-24 window address, or
    /// [`Self::NO_PATH`]. `None` for a node past the table or with no graph.
    pub fn pointer(&self, node: u8, dir: WalkDir) -> Option<u16> {
        if node as usize >= Self::NODES {
            return None;
        }
        let at = node as usize * 8 + dir as usize * 2;
        let lo = *self.blob.get(at)?;
        let hi = *self.blob.get(at + 1)?;
        Some(lo as u16 | (hi as u16) << 8)
    }

    /// `$3E8F`: the byte a bank-24 window pointer addresses. The port resolves it
    /// against the blob rather than a bank, so a pointer outside the extracted
    /// span answers `None` instead of reading whatever the ROM would find there.
    pub fn byte_at(&self, ptr: u16) -> Option<u8> {
        // The table's own window address. `MAP_WALK_ROM` is bank 24's flat
        // address, and a window address resolves as `flat + ($4000 - bank*$4000)`.
        let base = MAP_WALK_ROM - (24 * 0x4000 - 0x4000);
        let off = (ptr as u32).checked_sub(base)?;
        self.blob.get(off as usize).copied()
    }
}

/// `$3E1D7` — whether the held direction opens a level at this node. The caller
/// is `$3C4D3` (`call $3E1D7 / or a / jp z, $3C5DA`), so the ROM's `0` means the
/// node's own entry action runs instead of the cursor's walking path.
///
/// The ROM's four tests, in the order it applies them: `sMapScreenId != 0`;
/// `$4218[node] != $FE` (a pure path node — `$FF` is *not* excluded here);
/// `$6591[slot] & (hKeysHeld & $F0) != 0`, i.e. the held direction must be one
/// this node offers; and `sLevelProgress[slot]` bit 7 **clear**.
///
/// Two places where the port refuses to imitate the ROM: a node whose progress
/// slot is `$FE`/`$FF` would make `$6591` read past its table, and such nodes are
/// `$FE` in `$4218` as well, so the second test normally returns first — the port
/// returns false rather than modelling the out-of-bounds read. And with no table
/// loaded everything is false.
pub fn map_enterable(
    screen_id: u8,
    node: u8,
    nodes: &MapNodes,
    level_progress: &[u8],
    keys_held: u8,
) -> bool {
    if screen_id == 0 {
        return false;
    }
    if nodes.level_byte(node) == Some(0xFE) {
        return false;
    }
    let Some(slot) = nodes.progress_slot(node) else {
        return false;
    };
    if nodes.key_mask(slot).unwrap_or(0) & (keys_held & 0xF0) == 0 {
        return false;
    }
    level_progress.get(slot as usize).copied().unwrap_or(0) & 0x80 == 0
}

/// The four bytes the fresh-file init (`$30226`) and the blank-slot writer
/// (`$2913`) both put after the checksum, which `$28EC` checks before it will
/// accept a slot. `$2934` itself never writes them — and never writes the byte
/// after them either, so a validator that also required `+$4A` would be stricter
/// than the ROM.
pub const SAVE_SIGNATURE: [u8; 4] = [0x12, 0x34, 0x56, 0x78];

/// `$28EC`'s validation of one 0x50-byte save record: the checksum at +$45 must
/// be the sum of the 69 bytes before it, and the four signature bytes at +$46
/// must be the constant the fresh-file init writes.
pub fn save_record_valid(record: &[u8]) -> bool {
    if record.len() < 0x4B {
        return false;
    }
    let sum = record[..0x45].iter().fold(0u8, |c, b| c.wrapping_add(*b));
    sum == record[0x45] && record[0x46..0x4A] == SAVE_SIGNATURE
}

/// Signed tile index mapping for Game Boy LCDC bit 4 = 0 background addressing ($8800-$97FF).
///
/// Tile indices 0..127 read from $9000 (VRAM tile index 256..383).
/// Tile indices 128..255 read from $8800 (VRAM tile index 128..255).
pub const fn signed_bg_tile_index(tile: u8) -> usize {
    if tile < 128 {
        256 + tile as usize
    } else {
        tile as usize
    }
}

/// The extracted overworld background tilemaps (`extracted/map/tilemaps.bin`).
///
/// Contains the 64x64 hub tilemap (4,096 bytes) stitched from the four 32x32
/// quadrants, followed by the 14 screen tilemaps (each 32x32 = 1,024 bytes, 576
/// bytes visible): the twelve zone screens plus the two bell-bonus rooms
/// (screens 13/14). Total size: 18,432 bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverworldTilemaps {
    blob: Vec<u8>,
}

impl OverworldTilemaps {
    pub const HUB_LEN: usize = 4096;
    pub const SCREEN_LEN: usize = 1024;
    pub const SCREEN_COUNT: usize = 14;
    pub const TOTAL_LEN: usize = Self::HUB_LEN + Self::SCREEN_COUNT * Self::SCREEN_LEN;

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == Self::TOTAL_LEN).then(|| Self {
            blob: bytes.to_vec(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.blob.is_empty()
    }

    /// Read a tile byte from the 64x64 hub tilemap.
    pub fn hub_tile(&self, x: usize, y: usize) -> u8 {
        if x >= 64 || y >= 64 || self.blob.len() < Self::HUB_LEN {
            return 0;
        }
        self.blob[y * 64 + x]
    }

    /// Read a tile byte from a screen tilemap (1..=12).
    pub fn screen_tile(&self, screen_id: u8, x: usize, y: usize) -> u8 {
        if screen_id == 0 {
            return self.hub_tile(x, y);
        }
        if screen_id > Self::SCREEN_COUNT as u8 || x >= 32 || y >= 32 {
            return 0;
        }
        let off = Self::HUB_LEN + (screen_id as usize - 1) * Self::SCREEN_LEN;
        self.blob.get(off + y * 32 + x).copied().unwrap_or(0)
    }

    /// Access the raw tilemap for a screen.
    pub fn screen_tilemap(&self, screen_id: u8) -> Option<&[u8]> {
        if screen_id == 0 {
            self.blob.get(..Self::HUB_LEN)
        } else if screen_id <= Self::SCREEN_COUNT as u8 {
            let off = Self::HUB_LEN + (screen_id as usize - 1) * Self::SCREEN_LEN;
            self.blob.get(off..off + Self::SCREEN_LEN)
        } else {
            None
        }
    }
}

/// Composed VRAM background and sprite tile blocks for overworld screens (0..=12).
///
/// Each screen contains 384 tiles (6,144 bytes in 2bpp format):
/// - Tiles 0..31: 32 common overworld/cursor/icon tiles
/// - Tiles 32..383: 352 zone-specific background tiles
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverworldTiles {
    bytes: Vec<u8>,
}

impl OverworldTiles {
    pub const TILES_PER_SCREEN: usize = 384;
    pub const BYTES_PER_SCREEN: usize = Self::TILES_PER_SCREEN * 16; // 6144
    pub const SCREEN_COUNT: usize = 13;
    pub const TOTAL_LEN: usize = Self::SCREEN_COUNT * Self::BYTES_PER_SCREEN; // 79872

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == Self::TOTAL_LEN).then(|| Self {
            bytes: bytes.to_vec(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The screen's raw 384-tile 2bpp block, for the wave strips' byte writes.
    pub fn screen_raw(&self, screen_id: u8) -> &[u8] {
        let sid = (screen_id as usize).min(Self::SCREEN_COUNT - 1);
        let start = sid * Self::BYTES_PER_SCREEN;
        self.bytes
            .get(start..start + Self::BYTES_PER_SCREEN)
            .unwrap_or(&[])
    }

    /// Decode the 384 2bpp tiles for a given screen into 8x8 pixel color arrays (0..=3).
    pub fn screen_vram_tiles(&self, screen_id: u8) -> Vec<[u8; 64]> {
        let sid = (screen_id as usize).min(Self::SCREEN_COUNT - 1);
        let start = sid * Self::BYTES_PER_SCREEN;
        let screen_bytes = match self.bytes.get(start..start + Self::BYTES_PER_SCREEN) {
            Some(b) => b,
            None => return vec![[0u8; 64]; Self::TILES_PER_SCREEN],
        };
        let mut tiles = Vec::with_capacity(Self::TILES_PER_SCREEN);
        for chunk in screen_bytes.as_chunks::<16>().0 {
            let mut tile_16 = [0u8; 16];
            tile_16.copy_from_slice(chunk);
            tiles.push(crate::video::tile_colors(&tile_16));
        }
        tiles
    }
}

/// The bell-bonus rooms' background tiles (`extracted/map/bell_room_tiles.bin`).
///
/// `$3F4B1`/`$3F5A4` copy bank 25 `$5000` (256 tiles, 4 KiB) to VRAM
/// `$8800-$97FF`, so the block occupies tile numbers 128..383 of the
/// `$8000-$9FFF` map the overworld renderer walks. Both rooms share it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BellRoomTiles {
    bytes: Vec<u8>,
}

/// `$3F58`: bank 6 `$4000` -> `$8000`, the 128 tiles both bonus rooms'
/// sprites (`$6109B`) index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomCommonTiles {
    bytes: Vec<u8>,
}

impl RoomCommonTiles {
    pub const LEN: usize = 0x0800;
    pub const TILES: usize = 128;

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == Self::LEN).then(|| Self {
            bytes: bytes.to_vec(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn vram_tiles(&self) -> Vec<[u8; 64]> {
        let mut tiles = Vec::with_capacity(Self::TILES);
        for chunk in self.bytes.as_chunks::<16>().0 {
            let mut tile = [0u8; 16];
            tile.copy_from_slice(chunk);
            tiles.push(crate::video::tile_colors(&tile));
        }
        tiles
    }
}

/// `$305B6`: the castle reveal's `$8000` tiles (0x1800) and `$9800` tilemap.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CastleRevealTiles {
    tiles: Vec<u8>,
    tilemap: Vec<u8>,
}

impl CastleRevealTiles {
    pub const TILES_LEN: usize = 0x1800;
    pub const TILEMAP_LEN: usize = 0x300;
    /// The five four-byte runs `$45B6` writes tile `$74` into, in `$9800` space.
    pub const COIN_RUNS: [usize; 5] = [0x148, 0x168, 0x188, 0x1A8, 0x1C8];

    pub fn from_bytes(tiles: &[u8], tilemap: &[u8]) -> Option<Self> {
        (tiles.len() == Self::TILES_LEN && tilemap.len() == Self::TILEMAP_LEN).then(|| Self {
            tiles: tiles.to_vec(),
            tilemap: tilemap.to_vec(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn vram_tiles(&self) -> Vec<[u8; 64]> {
        let mut out = Vec::with_capacity(384);
        for chunk in self.tiles.as_chunks::<16>().0 {
            let mut tile = [0u8; 16];
            tile.copy_from_slice(chunk);
            out.push(crate::video::tile_colors(&tile));
        }
        out
    }
}

/// Draw the reveal's 20x18 tiles; all six coins set the five `$74` runs.
pub fn render_castle_reveal(
    reveal: &CastleRevealTiles,
    all_six_coins: bool,
    bgp: u8,
    obp1: u8,
    // `$FFC4`/`$FFC5`/`$FFC6`: the walk-in's Mario — `$FFC6` is a `$5297`
    // sprite id (`$BF + (anim >> 4)`), not a tile number.
    mario: Option<(u8, u8, u8)>,
    metas: &crate::room_sprites::SpriteMetas,
    vw: usize,
    vh: usize,
) -> Vec<u8> {
    let shades = crate::video::bgp_shades(bgp);
    let mut img = vec![shades[0]; vw * vh];
    if reveal.is_empty() {
        return img;
    }
    let vram = reveal.vram_tiles();
    let mut tilemap = reveal.tilemap.clone();
    if all_six_coins {
        for run in CastleRevealTiles::COIN_RUNS {
            for at in run..run + 4 {
                if let Some(slot) = tilemap.get_mut(at) {
                    *slot = 0x74;
                }
            }
        }
    }
    let cols = vw / 8;
    let rows = vh / 8;
    for ty in 0..rows {
        for tx in 0..cols {
            let Some(&byte) = tilemap.get(ty * 32 + tx) else {
                continue;
            };
            let index = signed_bg_tile_index(byte);
            let Some(tile) = vram.get(index) else {
                continue;
            };
            for py in 0..8 {
                for px in 0..8 {
                    img[(ty * 8 + py) * vw + tx * 8 + px] = shades[tile[py * 8 + px] as usize];
                }
            }
        }
    }
    if let Some((x, y, id)) = mario {
        let obj_shades = crate::video::bgp_shades(obp1);
        for (dx, dy, tile, attr) in metas.expand(id, x, y) {
            draw_obj_tile(
                &mut img,
                &vram,
                &obj_shades,
                dx as i32 - 8,
                dy as i32 - 16,
                tile,
                attr,
                vw,
                vh,
            );
        }
    }
    img
}

impl BellRoomTiles {
    pub const LEN: usize = 0x1000;
    /// Tile number the block starts at (`$8800`).
    pub const FIRST_TILE: usize = 128;

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == Self::LEN).then(|| Self {
            bytes: bytes.to_vec(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The 384 tiles of `$8000-$9FFF`, with the bell block at `$8800`.
    pub fn vram_tiles(&self) -> Vec<[u8; 64]> {
        let mut tiles = vec![[0u8; 64]; Self::FIRST_TILE];
        for chunk in self.bytes.as_chunks::<16>().0 {
            let mut tile = [0u8; 16];
            tile.copy_from_slice(chunk);
            tiles.push(crate::video::tile_colors(&tile));
        }
        tiles.resize(384, [0u8; 64]);
        tiles
    }
}

/// One tile displacement in a route patch stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PatchTile {
    pub dx: u8,
    pub dy: u8,
    pub tile: u8,
}

/// One patch stream in bank 24 ($623EA).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PatchStream {
    pub id: usize,
    pub ptr: String,
    pub tiles: Vec<PatchTile>,
}

/// One tile in a cursor metasprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct CursorTile {
    pub dx: u8,
    pub dy: u8,
    pub tile: u8,
    pub attr: u8,
}

/// One of the 16 cursor metasprite poses ($4AD7).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct CursorPoseDef {
    pub pose: u8,
    pub direction: String,
    pub step: u8,
    pub tiles: [CursorTile; 4],
}

/// Screen rendering metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ScreenMeta {
    pub id: u8,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub tilemap_offset: usize,
    pub tilemap_len: usize,
    pub gfx: String,
    pub bg_palette: u8,
}

/// Dynamic patch placement for route unlocks.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PatchPlacement {
    pub x: u8,
    pub y: u8,
    pub patch_ids: Vec<usize>,
}

/// Rule mapping `sLevelProgress` flag to dynamic patch placement.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct DynamicUnlock {
    pub name: String,
    pub screen_id: u8,
    pub progress_offset: usize,
    pub mask: u8,
    pub patches: Vec<PatchPlacement>,
}

/// Full extracted manifest for overworld route patches and cursor metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct OverworldManifest {
    pub screens: Vec<ScreenMeta>,
    pub patches: Vec<PatchStream>,
    pub cursor_poses: Vec<CursorPoseDef>,
    pub dynamic_unlocks: Vec<DynamicUnlock>,
}

impl OverworldManifest {
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Retrieve the cursor metasprite definition for a pose (0..15).
    pub fn cursor_pose(&self, pose: u8) -> Option<&CursorPoseDef> {
        self.cursor_poses.get((pose & 0x0F) as usize)
    }
}

/// Cursor OAM metasprites that bank 15 keeps outside the pose table, byte for
/// byte: the camera-scroll arrows at `$4B77` (two blink frames, tiles $14-$17),
/// the hidden block at `$4B87`, the screen-edge arrows at `$4B8F` (right) and
/// `$4BAF` (left), and the walk stump at `$4BCF`/`$4BD7`/`$4BDF`.
pub const SCROLL_ARROWS: [[(u8, u8); 4]; 2] = [
    [(0x14, 0x00), (0x15, 0x20), (0x16, 0x00), (0x17, 0x20)],
    [(0x15, 0x00), (0x14, 0x20), (0x17, 0x00), (0x16, 0x20)],
];
pub const HIDDEN_CELLS: [(u8, u8); 4] = [(0x7F, 0x00); 4];
pub const EDGE_RIGHT: [[(u8, u8); 4]; 4] = [
    [(0x10, 0x00), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
    [(0x11, 0x00), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
    [(0x12, 0x00), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
    [(0x13, 0x00), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
];
pub const EDGE_LEFT: [[(u8, u8); 4]; 4] = [
    [(0x10, 0x20), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
    [(0x11, 0x20), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
    [(0x12, 0x20), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
    [(0x13, 0x20), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
];
pub const WALK_STUMP: [[(u8, u8); 4]; 3] = [
    [(0x18, 0x20), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
    [(0x19, 0x20), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
    [(0x1A, 0x20), (0x7F, 0x00), (0x7F, 0x00), (0x7F, 0x00)],
];
/// `$498C`: the four-entry order the stump cycles through (`00 08 00 10`).
pub const STUMP_ORDER: [usize; 4] = [0, 1, 0, 2];

/// Which metasprite the map cursor draws this frame, chosen exactly like the
/// bank-15 chain at `$3C901`:
///
/// - `$3C901`: high nibble of `sMapCursorMove` equal to 8 (move bit 7) is the
///   camera scroll, drawn as the blinking `$4B77` arrows instead of the cursor.
/// - `$3C92A`: bit 2 of that nibble (`$40`) hides the cursor (`$4B87`, all `$7F`).
/// - `$3C92E`/`$3C933`: nibble `$0A`, or bit 1 clear, keeps the pose table entry.
/// - otherwise, while a walk stream runs, move bit 1 picks `$4B8F` and bit 0
///   `$4BAF` (four frames each, indexed by `walk_frame & $0C`); with neither the
///   stump at `$4BCF` cycles through `$498C` on `walk_frame & $18`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorSprite {
    Pose(u8),
    ScrollArrows(usize),
    Hidden,
    EdgeRight(usize),
    EdgeLeft(usize),
    Stump(usize),
}

pub fn cursor_sprite(
    cursor_move: u8,
    cursor_walking: u8,
    walk_frame: u8,
    cursor_pose: u8,
) -> CursorSprite {
    let hi = (cursor_move & 0xF0) >> 4;
    if hi == 0x08 {
        return CursorSprite::ScrollArrows(((walk_frame & 0x08) >> 3) as usize);
    }
    if hi & 0x04 != 0 {
        return CursorSprite::Hidden;
    }
    if hi != 0x0A && hi & 0x02 != 0 {
        if cursor_walking != 0 {
            if cursor_move & 0x02 != 0 {
                return CursorSprite::EdgeRight(((walk_frame & 0x0C) >> 2) as usize);
            }
            if cursor_move & 0x01 != 0 {
                return CursorSprite::EdgeLeft(((walk_frame & 0x0C) >> 2) as usize);
            }
        }
        return CursorSprite::Stump(STUMP_ORDER[((walk_frame & 0x18) >> 3) as usize]);
    }
    CursorSprite::Pose(cursor_pose & 0x0F)
}

/// `$4C4D`: the seventeen decor metasprites, `(dy, dx, tile, attr)` cells
/// terminated by a `$FF` tile (not a `$FF` dy - `$3D01D` tests the tile byte).
pub const DECOR_METASPRITES: [&[(i8, i8, u8, u8)]; 17] = [
    &[], // $ 4c6f - 0 cells
    &[
        (0, 8, 0x54, 0x00),
        (0, 16, 0x55, 0x00),
        (0, 24, 0x54, 0x20),
        (8, 0, 0x56, 0x00),
        (8, 8, 0x57, 0x00),
        (8, 16, 0x58, 0x00),
        (8, 24, 0x6D, 0x00),
        (8, 32, 0x7A, 0x00),
        (16, 0, 0x59, 0x00),
        (16, 8, 0x5A, 0x00),
        (16, 16, 0x5B, 0x00),
        (16, 24, 0x5A, 0x20),
        (16, 32, 0x7B, 0x00),
        (24, 0, 0x5C, 0x00),
        (24, 8, 0x5D, 0x00),
        (24, 16, 0x6C, 0x00),
        (24, 24, 0x5D, 0x20),
        (24, 32, 0x5C, 0x20),
    ], // $ 4c73 - 18 cells
    &[
        (0, 0, 0x60, 0x00),
        (0, 8, 0x61, 0x00),
        (8, 0, 0x62, 0x00),
        (8, 8, 0x63, 0x00),
    ], // $ 4cbf - 4 cells
    &[
        (0, 0, 0x64, 0x00),
        (0, 8, 0x65, 0x00),
        (8, 0, 0x66, 0x00),
        (8, 8, 0x67, 0x00),
    ], // $ 4cd3 - 4 cells
    &[
        (0, 0, 0x68, 0x00),
        (0, 8, 0x69, 0x00),
        (8, 0, 0x6A, 0x00),
        (8, 8, 0x6B, 0x00),
    ], // $ 4ce7 - 4 cells
    &[
        (0, 0, 0x3A, 0x00),
        (0, 8, 0x3B, 0x00),
        (0, 16, 0x3C, 0x00),
        (8, 0, 0x3D, 0x00),
        (8, 8, 0x3E, 0x00),
        (8, 16, 0x3F, 0x00),
        (16, 0, 0x40, 0x00),
        (16, 8, 0x41, 0x00),
        (16, 16, 0x42, 0x00),
    ], // $ 4cfb - 9 cells
    &[
        (0, 0, 0x45, 0x00),
        (0, 8, 0x3B, 0x00),
        (0, 16, 0x3C, 0x00),
        (8, 0, 0x46, 0x00),
        (8, 8, 0x47, 0x00),
        (8, 16, 0x3F, 0x00),
        (16, 0, 0x48, 0x00),
        (16, 8, 0x49, 0x00),
        (16, 16, 0x42, 0x00),
    ], // $ 4d23 - 9 cells
    &[
        (0, 0, 0x4A, 0x00),
        (0, 8, 0x3B, 0x00),
        (0, 16, 0x3C, 0x00),
        (8, 0, 0x4B, 0x00),
        (8, 8, 0x4C, 0x00),
        (8, 16, 0x3F, 0x00),
        (16, 0, 0x4D, 0x00),
        (16, 8, 0x4E, 0x00),
        (16, 16, 0x42, 0x00),
    ], // $ 4d4b - 9 cells
    &[
        (0, 0, 0x4F, 0x00),
        (0, 8, 0x3B, 0x00),
        (0, 16, 0x3C, 0x00),
        (8, 0, 0x50, 0x00),
        (8, 8, 0x51, 0x00),
        (8, 16, 0x3F, 0x00),
        (16, 0, 0x52, 0x00),
        (16, 8, 0x53, 0x00),
        (16, 16, 0x42, 0x00),
    ], // $ 4d73 - 9 cells
    &[
        (0, 0, 0x20, 0x00),
        (0, 8, 0x21, 0x00),
        (0, 16, 0x22, 0x00),
        (0, 24, 0x23, 0x00),
        (8, 0, 0x24, 0x00),
        (8, 8, 0x25, 0x00),
        (8, 16, 0x26, 0x00),
        (8, 24, 0x27, 0x00),
    ], // $ 4d9b - 8 cells
    &[
        (0, 0, 0x36, 0x00),
        (0, 8, 0x36, 0x20),
        (0, 16, 0x36, 0x00),
        (0, 24, 0x36, 0x20),
    ], // $ 4dbf - 4 cells
    &[(0, 0, 0x43, 0x00), (8, 0, 0x44, 0x00)], // $ 4dd3 - 2 cells
    &[
        (0, 0, 0x7C, 0x00),
        (0, 8, 0x7C, 0x20),
        (8, 0, 0x7D, 0x00),
        (8, 8, 0x7D, 0x20),
    ], // $ 4ddf - 4 cells
    &[
        (0, 0, 0x37, 0x00),
        (0, 8, 0x38, 0x00),
        (8, 0, 0x37, 0x40),
        (8, 8, 0x38, 0x40),
    ], // $ 4df3 - 4 cells
    &[
        (0, 0, 0x38, 0x20),
        (0, 8, 0x37, 0x20),
        (8, 0, 0x38, 0x60),
        (8, 8, 0x37, 0x60),
    ], // $ 4e07 - 4 cells
    &[
        (0, 0, 0x7D, 0x40),
        (0, 8, 0x7D, 0x60),
        (8, 0, 0x7C, 0x40),
        (8, 8, 0x7C, 0x60),
    ], // $ 4e1b - 4 cells
    &[(0, 0, 0x00, 0x00)], // $ 4e2f - 1 cells
];

/// `$4FA8`: the X/Y pairs the first sparkle slot (`sMapSprite16`) picks from,
/// indexed by `rDIV & $0E`; `$4FB8` is the same for the second (`$3CEC1`).
pub const SPARKLE_SLOT_16: [u8; 16] = [
    0xD8, 0xAA, 0xDC, 0xB0, 0xE0, 0xAC, 0xDC, 0xAA, 0xD8, 0xAA, 0xD5, 0xB2, 0xD6, 0xB4, 0xD8, 0xB1,
];
pub const SPARKLE_SLOT_17: [u8; 16] = [
    0x02, 0xB0, 0x00, 0xB2, 0x04, 0xAC, 0x03, 0xAE, 0x01, 0xB0, 0x00, 0xB2, 0x02, 0xAA, 0x00, 0xB4,
];

/// `$4E37`: the six `sGoldenCoinN` bytes `$3CE43` reads for the coin icons, as
/// offsets into `level_progress` (`$A848`; each address minus `$A848 + 0`).
/// Slot 4 + n is live with id 2 when that byte has bit 7 set.
pub const HUB_COIN_FLAGS: [usize; 6] = [0x09, 0x05, 0x0E, 0x11, 0x17, 0x1D];

/// The hub's `sMapSprite0Page` list (`$4BFD`, copied by `$3CBE7`): `(A, X, Y, ID)`.
/// `A`'s high nibble is the graphics bank offset and its low nibble tags the
/// camera compare; `ID == $FF` is an empty slot, otherwise `ID` indexes
/// [`DECOR_METASPRITES`].
pub const HUB_DECOR_SLOTS: [(u8, u8, u8, u8); 18] = [
    (0x00, 0x4C, 0x4F, 0x01),
    (0x10, 0x78, 0x36, 0xFF),
    (0x10, 0xA8, 0xF0, 0x09),
    (0x01, 0xC0, 0x28, 0x09),
    (0x10, 0x7F, 0x55, 0xFF),
    (0x01, 0x5A, 0x00, 0xFF),
    (0x00, 0xD2, 0x60, 0xFF),
    (0x10, 0x28, 0x38, 0xFF),
    (0x11, 0xB0, 0x5C, 0xFF),
    (0x10, 0x8E, 0xC8, 0xFF),
    (0x11, 0x98, 0x78, 0xFF),
    (0x11, 0x90, 0x78, 0xFF),
    (0x11, 0x88, 0x78, 0xFF),
    (0x11, 0x80, 0x78, 0xFF),
    (0x11, 0x88, 0x70, 0x05),
    (0x11, 0x78, 0x78, 0xFF),
    (0x00, 0xD8, 0xA8, 0xFF),
    (0x10, 0x00, 0xB0, 0xFF),
];

/// `$3D044`'s 16-bit camera cull. The ROM builds both sides in "bank-tagged"
/// space - the camera's high byte OR'd with `$06`, the slot's own nibble OR'd
/// with `$06` - and keeps the slot only when `cam - $30 < slot < cam + $D0` on
/// both axes. Anything outside that 208-pixel window is flagged
/// `sMapSpriteOffscreen` and skipped.
fn decor_offscreen(slot_a: u8, slot_x: u8, slot_y: u8, cam_x: u16, cam_y: u16) -> bool {
    // X allows `-$30 .. +$D0`, Y allows `-$38 .. +$C8` (`$3D044`'s `$FFD0`/`$00D0`
    // and `$FFC8`/`$00C8` biases); the two axes are not symmetric.
    let inside = |cam: u16, value: u8, nibble: u8, lo: u16, hi: u16| {
        let cam_side = (((0x06 | (cam >> 8) as u8) as u16) << 8) | (cam as u8) as u16;
        let slot_side = (((0x06 | nibble) as u16) << 8) | value as u16;
        slot_side > cam_side.wrapping_sub(lo) && slot_side < cam_side.wrapping_add(hi)
    };
    !(inside(cam_x, slot_x, slot_a >> 4, 0x30, 0xD0)
        && inside(cam_y, slot_y, slot_a & 0x0F, 0x38, 0xC8))
}

/// One expanded decor slot: the OAM bytes `$3D001` would stage for it, or `None`
/// when the camera cull drops it.
///
/// `$3D044` ends with `x = slot.x - cam.x + 8`, `y = slot.y - cam.y + 16` (both
/// 8-bit), and `$3D01D` adds each cell's `(dy, dx)`; the OAM values are therefore
/// `screen + (8, 16)`. Measured on the console at `cam = ($BB, $0142)`, slot 14
/// (`$11 $88 $70 $05`) staged `y=62 x=213 tile=$3A attr=$00`, which is this
/// arithmetic exactly.
pub fn decor_oam(slot: (u8, u8, u8, u8), cam_x: u16, cam_y: u16) -> Option<Vec<(u8, u8, u8, u8)>> {
    let (a, x, y, id) = slot;
    if id == 0xFF || id as usize >= DECOR_METASPRITES.len() {
        return None;
    }
    if decor_offscreen(a, x, y, cam_x, cam_y) {
        return None;
    }
    let oam_x = x.wrapping_sub(cam_x as u8).wrapping_add(8);
    let oam_y = y.wrapping_sub(cam_y as u8).wrapping_add(16);
    Some(
        DECOR_METASPRITES[id as usize]
            .iter()
            .map(|&(dy, dx, tile, attr)| {
                (
                    oam_y.wrapping_add(dy as u8),
                    oam_x.wrapping_add(dx as u8),
                    tile,
                    attr,
                )
            })
            .collect(),
    )
}

/// The eight tiles the hub's wave strips write into, in strip order: strip `n`
/// owns `WAVE_TILES[2n]` (`$88E0`-style base) and `WAVE_TILES[2n+1]` (`$88F0`).
pub const WAVE_TILES: [u8; 8] = [0x8E, 0x8F, 0x9E, 0x9F, 0xCE, 0xCF, 0xDE, 0xDF];

/// `$3E7BE` + `$3E853`: the hub's shoreline shimmer.
///
/// `$3E7BE` seeds four 32-byte strips from the tile bytes at `$88E0`, `$89E0`,
/// `$8CE0` and `$8DE0` (so strip `n` byte `e` is tile `2n`'s byte `e` and byte
/// `e + $10` is tile `2n+1`'s byte `e`). Each frame `$3E853` rotates one 16-bit
/// window per strip by a bit and writes the two bytes back into the two tiles,
/// 0x10 apart. An even phase rotates all four strips **left** at
/// `(phase & 7)*2`, an odd phase rotates them **right**; either way it then
/// repeats strips 2 and 3 at `((phase & 7)*2) | 1`, so a phase writes eight
/// bytes (four strips at `e`, two strips at `e|1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapWave {
    /// `sMapRasterPhase` (`$A689`): incremented before the rotation is chosen.
    pub phase: u8,
    /// `sMapWaveStrip0..3` (`$A700`/`$A720`/`$A740`/`$A760`).
    pub strips: [[u8; 32]; 4],
    /// The eight affected tiles' raw 2bpp bytes, keyed by [`WAVE_TILES`], so the
    /// renderer can re-decode after a write.
    pub raw: [[u8; 16]; 8],
}

impl MapWave {
    /// `$3E7BE`: seed the strips from a screen's raw 384-tile block.
    pub fn seed(raw_block: &[u8]) -> Option<Self> {
        let mut raw = [[0u8; 16]; 8];
        for (i, tile) in WAVE_TILES.iter().enumerate() {
            let off = *tile as usize * 16;
            raw[i].copy_from_slice(raw_block.get(off..off + 16)?);
        }
        let mut strips = [[0u8; 32]; 4];
        for (n, strip) in strips.iter_mut().enumerate() {
            strip[..16].copy_from_slice(&raw[2 * n]);
            strip[16..].copy_from_slice(&raw[2 * n + 1]);
        }
        Some(Self {
            phase: 0,
            strips,
            raw,
        })
    }

    /// One `$3E853` call.
    pub fn tick(&mut self) {
        self.phase = self.phase.wrapping_add(1);
        let e = ((self.phase & 7) as usize) * 2;
        let left = self.phase & 1 == 0;
        for strip in 0..4 {
            self.pass(strip, e, left);
        }
        // Both paths end with `set 0, e` and repeat strips 2 and 3 at the odd
        // byte offset, so every phase writes eight bytes.
        for strip in 2..4 {
            self.pass(strip, e | 1, left);
        }
    }

    /// One strip's rotate-and-write. `$3E853`'s left variant is
    /// `sla a / rl d / rra / rlca`, the right variant `srl a / rr d / rla / rrca`,
    /// where `a` is the strip's `e + $10` byte and `d` its `e` byte.
    fn pass(&mut self, strip_index: usize, e: usize, left: bool) {
        let e = e & 0x0F;
        let (d1, a1) = (
            self.strips[strip_index][e],
            self.strips[strip_index][e + 0x10],
        );
        let (d, a) = if left {
            let a = a1 << 1;
            let carry = a1 >> 7;
            let d = (d1 << 1) | carry;
            let carry = d1 >> 7;
            // `rra`: the carry comes from `rl d`, i.e. bit 7 of the old d.
            let a = (a >> 1) | (carry << 7);
            (d, a.rotate_left(1))
        } else {
            let carry = a1 & 1;
            let a = a1 >> 1;
            let d = (d1 >> 1) | (carry << 7);
            // `rla`: the carry comes from `rr d`, i.e. bit 0 of the old d.
            let a = (a << 1) | (d1 & 1);
            (d, a.rotate_right(1))
        };
        // `ld [hl], a` -> the $..F0 tile, then `ld [hl], d` -> the $..E0 tile.
        self.raw[2 * strip_index + 1][e] = a;
        self.raw[2 * strip_index][e] = d;
        self.strips[strip_index][e] = d;
        self.strips[strip_index][e + 0x10] = a;
    }

    /// Re-decode the affected tiles into a screen VRAM block.
    pub fn apply(&self, vram: &mut [[u8; 64]]) {
        for (i, tile) in WAVE_TILES.iter().enumerate() {
            if let Some(slot) = vram.get_mut(*tile as usize) {
                *slot = crate::video::tile_colors(&self.raw[i]);
            }
        }
    }
}

/// Blit one 8x8 OBJ tile out of the map's VRAM block, honouring the attr flips
/// (bit 5 = X, bit 6 = Y). Colour 0 is transparent, as on the console.
#[allow(clippy::too_many_arguments)]
fn draw_obj_tile(
    img: &mut [u8],
    tiles_vram: &[[u8; 64]],
    obj_shades: &[u8; 4],
    x0: i32,
    y0: i32,
    tile: u8,
    attr: u8,
    vw: usize,
    vh: usize,
) {
    let Some(tile_data) = tiles_vram.get(tile as usize) else {
        return;
    };
    for dy in 0..8 {
        let py = y0 + dy;
        if py < 0 || py >= vh as i32 {
            continue;
        }
        let src_y = if attr & 0x40 != 0 { 7 - dy } else { dy };
        for dx in 0..8 {
            let px = x0 + dx;
            if px < 0 || px >= vw as i32 {
                continue;
            }
            let src_x = if attr & 0x20 != 0 { 7 - dx } else { dx };
            let col = tile_data[src_y as usize * 8 + src_x as usize];
            if col != 0 {
                img[py as usize * vw + px as usize] = obj_shades[col as usize];
            }
        }
    }
}

/// Expand a screen's decor slots into the OAM cells the renderer takes.
pub fn hub_decor_cells(
    slots: &[(u8, u8, u8, u8)],
    cam_x: u16,
    cam_y: u16,
) -> Vec<(u8, u8, u8, u8)> {
    let mut out = Vec::new();
    for slot in slots {
        if let Some(cells) = decor_oam(*slot, cam_x, cam_y) {
            out.extend(cells);
        }
    }
    out
}

/// `$7251`: the nine OAM entries `$3F203` copies into `sStatusBarOAM` (OAM 4-12)
/// before patching the five digit tiles. `y = 152` is screen row 136, the last
/// one; `x` is OAM space, so the bar starts at screen x = 80.
pub const STATUS_BAR_TEMPLATE: [(u8, u8, u8, u8); 9] = [
    (152, 88, 0x7C, 0x00),  // Mario head
    (152, 96, 0x6F, 0x00),  // "x"
    (152, 104, 0x70, 0x00), // lives tens
    (152, 112, 0x70, 0x00), // lives ones
    (152, 128, 0x7D, 0x00), // coin
    (152, 136, 0x6F, 0x00), // "x"
    (152, 144, 0x70, 0x00), // coins hundreds
    (152, 152, 0x70, 0x00), // coins tens
    (152, 160, 0x70, 0x00), // coins ones (screen x 152, half off the edge)
];

/// The overworld status bar's counters. `$3F203` builds it from `$7251` and then
/// patches five tiles to `nibble | $70`.
#[derive(Clone, Copy, Debug, Default)]
pub struct StatusBar {
    pub lives: u8,
    pub coins_low: u8,
    pub coins_high: u8,
}

impl StatusBar {
    /// `$3F203`'s digit patches: `sLives` nibbles into entries 6 and 7,
    /// `sCoinHigh`'s low nibble into entry 10, `sCoinLow`'s nibbles into 11 and 12.
    pub fn cells(&self) -> [(u8, u8, u8, u8); 9] {
        let mut cells = STATUS_BAR_TEMPLATE;
        cells[0].2 = 0x7C;
        cells[2].2 = (self.lives >> 4) | 0x70;
        cells[3].2 = (self.lives & 0x0F) | 0x70;
        cells[4].2 = 0x7D;
        cells[6].2 = (self.coins_high & 0x0F) | 0x70;
        cells[7].2 = (self.coins_low >> 4) | 0x70;
        cells[8].2 = (self.coins_low & 0x0F) | 0x70;
        cells
    }
}

/// The `$3F203` backdrop: the bottom row's right half, `(x0, y0, x1, y1)`. It
/// starts where the bar's first sprite does - screen x 80 - and stops at the
/// bottom edge, so the counters sit on a flat panel rather than a full-width
/// band across the map.
pub fn status_bar_strip_rect(vw: usize, vh: usize) -> (usize, usize, usize, usize) {
    (vw / 2, vh.saturating_sub(8), vw, vh)
}

/// `$4A11`: the four OAM entries `$3C9F4` stages while the hub pans its camera
/// (B held, `$3C5B2`). Entry order and bytes are the ROM's: (y, x, tile, attr).
/// `$4A21` is the same layout with tile `$7F`. The choice is
/// `ldh a,[$FF97] / swap a / and $01`, i.e. **bit 4** of the frame counter, so
/// the arrows show for 16 frames and blink for 16 - measured on the console:
/// B+Right on the hub gave 16 frames of `$7F` then 16 of `$7E`. While panning
/// the cursor is not drawn at all.
pub const PAN_ARROWS: [(u8, u8, u8, u8); 4] = [
    (24, 80, 0x7E, 0x00),
    (140, 80, 0x7E, 0x40),
    (84, 16, 0x39, 0x00),
    (84, 152, 0x39, 0x20),
];
/// `$4A21`'s tile: the blank half of the blink.
pub const PAN_ARROW_BLANK: u8 = 0x7F;

/// Render one overworld frame into a 160x144 (or custom vw x vh) grayscale pixel buffer.
#[allow(clippy::too_many_arguments)]
pub fn render_overworld_view(
    screen_id: u8,
    cam_x: u16,
    cam_y: u16,
    cursor_x: u8,
    cursor_y: u8,
    cursor_move: u8,
    cursor_walking: u8,
    walk_frame: u8,
    cursor_pose: u8,
    // Decor OAM cells `(y, x, tile, attr)` in the order the console stages them:
    // the hub's `$3CFC8` starts at entry 13, a zone screen's `$3E277` at entry 4.
    // Either way they draw before the cursor, which owns entries 0-3.
    decor_cells: &[(u8, u8, u8, u8)],
    // `$3F203`'s status bar, present on the hub and cleared by `$3E277` on the
    // zone screens. `None` leaves the strip alone.
    status_bar: Option<&StatusBar>,
    // `sMapScrollOnlyLatch != 0`: B is panning the hub camera, which replaces
    // the cursor with the `$4A11` border arrows. `frame_counter` stands in for
    // `$FF97`: bit 4 picks `$4A11` (visible) or `$4A21` (blank), a 16-frame
    // half-period.
    camera_pan: bool,
    frame_counter: u8,
    tilemaps: &OverworldTilemaps,
    manifest: &OverworldManifest,
    level_progress: &[u8],
    tiles_vram: &[[u8; 64]],
    bg_palette: u8,
    obj_palette: u8,
    vw: usize,
    vh: usize,
) -> Vec<u8> {
    let bg_shades = crate::video::bgp_shades(bg_palette);
    let obj_shades = crate::video::bgp_shades(obj_palette);
    let mut img = vec![bg_shades[0]; vw * vh];
    let _ = &img;

    if screen_id == 0 {
        // Hub 64x64 stitched map
        for py in 0..vh {
            let wy = (cam_y as usize + py) % 512;
            let ty = wy / 8;
            let sub_y = wy % 8;
            for px in 0..vw {
                let wx = (cam_x as usize + px) % 512;
                let tx = wx / 8;
                let sub_x = wx % 8;
                let tile_byte = tilemaps.hub_tile(tx, ty);
                let vram_idx = signed_bg_tile_index(tile_byte);
                if let Some(tile_data) = tiles_vram.get(vram_idx) {
                    let col = tile_data[sub_y * 8 + sub_x];
                    img[py * vw + px] = bg_shades[col as usize];
                }
            }
        }
    } else {
        // Zone screen 32x18 tiles (cloned to apply dynamic route patches)
        let mut active_map = [0u8; 1024];
        if let Some(base_map) = tilemaps.screen_tilemap(screen_id) {
            let n = active_map.len().min(base_map.len());
            active_map[..n].copy_from_slice(&base_map[..n]);
        }
        for unlock in &manifest.dynamic_unlocks {
            if unlock.screen_id == screen_id {
                let prog = level_progress
                    .get(unlock.progress_offset)
                    .copied()
                    .unwrap_or(0);
                if prog & unlock.mask != 0 {
                    for placement in &unlock.patches {
                        for &patch_id in &placement.patch_ids {
                            if let Some(patch) = manifest.patches.get(patch_id) {
                                for tile in &patch.tiles {
                                    let tx = (placement.x as usize + tile.dx as usize) & 0x1F;
                                    let ty = (placement.y as usize + tile.dy as usize) & 0x1F;
                                    active_map[ty * 32 + tx] = tile.tile;
                                }
                            }
                        }
                    }
                }
            }
        }
        for py in 0..vh.min(144) {
            let ty = py / 8;
            let sub_y = py % 8;
            for px in 0..vw.min(256) {
                let tx = px / 8;
                let sub_x = px % 8;
                let tile_byte = active_map[ty * 32 + tx];
                let vram_idx = signed_bg_tile_index(tile_byte);
                if let Some(tile_data) = tiles_vram.get(vram_idx) {
                    let col = tile_data[sub_y * 8 + sub_x];
                    img[py * vw + px] = bg_shades[col as usize];
                }
            }
        }
    }

    for &(oy, ox, tile, attr) in decor_cells {
        if tile == 0x7F {
            continue;
        }
        draw_obj_tile(
            &mut img,
            tiles_vram,
            &obj_shades,
            ox as i32 - 8,
            oy as i32 - 16,
            tile,
            attr,
            vw,
            vh,
        );
    }

    if camera_pan {
        // `$3C9F4`: the whole cursor metasprite is replaced by the border arrows
        // (`$4A11` visible, `$4A21` all `$7F`), so no Mario while panning.
        if frame_counter & 0x10 == 0 {
            for (oy, ox, tile, attr) in PAN_ARROWS {
                draw_obj_tile(
                    &mut img,
                    tiles_vram,
                    &obj_shades,
                    ox as i32 - 8,
                    oy as i32 - 16,
                    tile,
                    attr,
                    vw,
                    vh,
                );
            }
        }
        return img;
    }

    // `$3F203` fills tile `$E1` over the bottom row so the counters have a flat
    // backdrop. It covers the right half only - screen x 80 onwards, where the
    // bar's first sprite sits - and `$E1` is colour 0, i.e. the palette's
    // lightest shade, not black.
    let (x0, y0, x1, y1) = status_bar_strip_rect(vw, vh);
    for py in y0..y1 {
        for px in x0..x1 {
            img[py * vw + px] = bg_shades[0];
        }
    }
    if let Some(bar) = status_bar {
        for (oy, ox, tile, attr) in bar.cells() {
            draw_obj_tile(
                &mut img,
                tiles_vram,
                &obj_shades,
                ox as i32 - 8,
                oy as i32 - 16,
                tile,
                attr,
                vw,
                vh,
            );
        }
    }

    // Cursor OAM (`$3C901`): four 8x8 cells from the metasprite the chain picks,
    // placed at (cursor_x, cursor_y) plus the (+4,+8) shift the walk stream adds
    // while `sMapCursorMove` has a direction and bit 5 set (`$3C9B5`).
    let (shift_x, shift_y) = if cursor_move & 0x0F != 0 && cursor_move & 0x20 != 0 {
        (4i32, 8i32)
    } else {
        (0, 0)
    };
    let cells: [(u8, u8); 4] =
        match cursor_sprite(cursor_move, cursor_walking, walk_frame, cursor_pose) {
            CursorSprite::Pose(pose) => match manifest.cursor_pose(pose) {
                Some(def) => {
                    let mut arr = HIDDEN_CELLS;
                    for (i, tile) in def.tiles.iter().take(4).enumerate() {
                        arr[i] = (tile.tile, tile.attr);
                    }
                    arr
                }
                None => HIDDEN_CELLS,
            },
            CursorSprite::ScrollArrows(frame) => SCROLL_ARROWS[frame & 1],
            CursorSprite::Hidden => HIDDEN_CELLS,
            CursorSprite::EdgeRight(frame) => EDGE_RIGHT[frame & 3],
            CursorSprite::EdgeLeft(frame) => EDGE_LEFT[frame & 3],
            CursorSprite::Stump(frame) => WALK_STUMP[frame % WALK_STUMP.len()],
        };
    let spr_x0 = cursor_x as i32 - 8 + shift_x;
    let spr_y0 = cursor_y as i32 - 16 + shift_y;
    for (i, (tile, attr)) in cells.iter().enumerate() {
        if *tile == 0x7F {
            continue; // $7F is the metasprite's blank entry
        }
        draw_obj_tile(
            &mut img,
            tiles_vram,
            &obj_shades,
            spr_x0 + (i as i32 & 1) * 8,
            spr_y0 + (i as i32 >> 1) * 8,
            *tile,
            *attr,
            vw,
            vh,
        );
    }

    img
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real blob `sml2-extract` writes (`extracted/map/nodes.bin` = bank 15
    /// `$4218` then `$650E`, 130 bytes each). Embedded so the tests need no file
    /// and still pin the ROM's own bytes.
    const BLOB_HEX: &str = "\
        00 FE FE FE 19 11 FE FE FE FE FE FE FE FE FE FE FE FE FE FE \
        FE FE FE FE FE FE 11 FE FE FE FE FE 05 03 02 01 04 FE 1D FE \
        06 07 08 09 FE FE FE FE FE FE 14 15 16 17 FE FE FE FE FE FE \
        12 13 FE FE FE FE FE FE 1C FE 0E FE 0F 10 FE FE FE FE FE FE \
        0A 0B 0C 0D FE FE FE FE FE FE FE FF FF FE FE FE FE FE FE FE \
        FE FE 1A FE FE FE 1E FE FE FE FE FE FE FE FE FE FE FE FE FE \
        1B FE 1F FE FE FE FE FE FE FE 00 FE FE FE 28 1F FE FE FE FE \
        FE FE FE FE FE FE FF FE FE FE FE FE FE FE FE FE 1F FE FE FE \
        FE FE 05 03 02 01 04 FE 24 FE 06 07 08 09 FE FE FE FE FE FE \
        0B 0C 0D 0E FE FE FE FE FE FE 10 11 FE 13 FE FE FE FE 29 FE \
        15 FE 16 17 FE FE FE FE FE FE 1A 1B 1C 1D FE FE FE FE FE FE \
        20 FE FE FE FE FE FE FE FE FE FE FE 25 FE FE FE 23 FE FE FE \
        FE FE FE FE FE FE FE FE FE FE 26 FE 27 FE FE FE FE FE FE FE \
        00 10 40 40 40 00 40 20 40 00 00 10 10 10 00 00 \
        40 00 00 10 00 80 80 00 00 00 40 40 40 00 00 10 \
        00 00 00 00 00 00 00 00 00 44";

    fn nodes() -> MapNodes {
        let bytes: Vec<u8> = BLOB_HEX
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).unwrap())
            .collect();
        assert_eq!(bytes.len(), MapNodes::BLOB_LEN, "302 bytes");
        MapNodes::from_bytes(&bytes).expect("302-byte blob")
    }

    /// Node 0 is level 1 and a plain level — the case the `$14` hand-off's tail
    /// keys on (`or a / jr nz` means the progress mark happens only for node 0).
    #[test]
    fn node_zero_is_level_one_and_a_plain_level() {
        let n = nodes();
        assert_eq!(n.level_for(0), Some(0), "$4218[0] = 0 = level 1");
        assert_eq!(n.progress_slot(0), Some(0), "$650E[0] = 0");
    }

    /// `$6591` as the ROM has it. Note slot 0's mask is `$00`: no direction opens
    /// node 0 through `$3E1D7`, which is a real property of the game, not a bug
    /// in the extraction — level 1 is entered by the story path, not by walking a
    /// zone screen's arrow.
    #[test]
    fn the_key_masks_are_the_roms_table() {
        let n = nodes();
        assert_eq!(n.key_mask(0), Some(0x00), "slot 0: no direction opens it");
        assert_eq!(n.key_mask(1), Some(0x10), "slot 1: Right");
        assert_eq!(n.key_mask(2), Some(0x40), "slot 2: Up");
        assert_eq!(n.key_mask(40), Some(0x00));
        // The ROM's slot range reaches `$29`, and `$6591[$29]` is the byte past
        // the table's last plausible mask — the port hands back exactly what the
        // ROM would read there rather than inventing a value or refusing.
        assert_eq!(n.key_mask(41), Some(0x44), "$6591[$29]");
        assert_eq!(n.key_mask(42), None, "and nothing beyond the table");
        // Every entry up to the last one the slot range addresses is a legal
        // direction mask; index 41 is the only one that is not.
        for slot in 0..=40u8 {
            assert_eq!(n.key_mask(slot).unwrap() & !0xF0, 0, "slot {slot}");
        }
    }

    /// The tables' shape, as the ROM's own data has it.
    #[test]
    fn the_real_tables_have_the_expected_shape() {
        let n = nodes();
        let with_level = (0..MapNodes::NODES as u8)
            .filter(|i| n.level_for(*i).is_some())
            .count();
        assert_eq!(with_level, 32, "one node per level");
        let with_slot = (0..MapNodes::NODES as u8)
            .filter(|i| n.progress_slot(*i).is_some())
            .count();
        assert_eq!(with_slot, 34);
        // The six castle slots sit at nodes $20-$24, in this order.
        assert_eq!(
            (0x20..=0x24)
                .map(|i| n.progress_slot(i).unwrap())
                .collect::<Vec<_>>(),
            vec![5, 3, 2, 1, 4]
        );
        // Node $1A is a castle-ish node; $1D is a pure path node. (A prior
        // research pass reported these the other way round; the ROM listing and
        // baserom agree with this.)
        assert_eq!(
            (n.level_for(0x1A), n.progress_slot(0x1A)),
            (Some(0x11), Some(0x1F))
        );
        assert_eq!((n.level_for(0x1D), n.progress_slot(0x1D)), (None, None));
        // $FE and $FF both mean "none".
        assert_eq!(n.level_for(0x6B), None, "$4218[$6B] = $FF");
    }

    /// A wrong-sized or absent blob answers `None` rather than reading garbage.
    #[test]
    fn a_short_blob_is_refused() {
        assert!(MapNodes::from_bytes(&[0u8; 259]).is_none());
        assert!(MapNodes::from_bytes(&[0u8; 261]).is_none());
        let empty = MapNodes::default();
        assert!(empty.is_empty());
        assert_eq!(empty.level_for(0), None);
        assert_eq!(empty.progress_slot(0), None);
    }

    /// `$28EC`: the checksum is the sum of the 69 bytes before it, and the four
    /// signature bytes must be the init constant. The byte *after* them is not
    /// checked — `$2934` never writes it and the ROM's validator stops at +$49,
    /// so requiring it (as this port used to) was stricter than the ROM.
    #[test]
    fn the_save_validator_checks_sum_and_signature() {
        let mut rec = [0u8; 0x50];
        rec[0x40] = 3;
        rec[0x43] = 5;
        let sum = rec[..0x45].iter().fold(0u8, |c, b| c.wrapping_add(*b));
        rec[0x45] = sum;
        rec[0x46..0x4A].copy_from_slice(&SAVE_SIGNATURE);
        assert!(save_record_valid(&rec));

        rec[0x45] = sum.wrapping_add(1);
        assert!(!save_record_valid(&rec), "a bad checksum fails");
        rec[0x45] = sum;
        rec[0x49] = 0x00;
        assert!(!save_record_valid(&rec), "a bad signature fails");
        rec[0x49] = 0x78;
        rec[0x4A] = 0xFF;
        assert!(
            save_record_valid(&rec),
            "but +$4A is not part of the check — the ROM's compare stops at +$49"
        );
        assert!(!save_record_valid(&[0u8; 8]), "a short record fails");
    }

    /// Screens 13/14 exist in `map/tilemaps.bin` and use the bank-25 `$5000`
    /// block the room loaders copy to `$8800` (tile numbers 128..383), not the
    /// twelve zone tilesets in `map/tiles.bin`.
    #[test]
    fn bell_room_screens_have_their_own_tiles() {
        let tilemaps =
            OverworldTilemaps::from_bytes(include_bytes!("../../../extracted/map/tilemaps.bin"))
                .expect("18,432-byte tilemaps with 14 screens");
        assert_eq!(tilemaps.screen_tile(13, 0, 0), 0x77, "bank 17 $7800");
        assert_eq!(tilemaps.screen_tile(14, 0, 0), 0x2E, "bank 17 $7A40");
        // The rooms are 20 tiles wide; the rest of the row is $00, which the
        // signed addressing reads from $9000 — inside the copied block.
        assert_eq!(tilemaps.screen_tile(13, 20, 0), 0x00);

        let tiles =
            BellRoomTiles::from_bytes(include_bytes!("../../../extracted/map/bell_room_tiles.bin"))
                .expect("4,096-byte bell tileset");
        let vram = tiles.vram_tiles();
        assert_eq!(vram.len(), 384);
        assert!(vram[..BellRoomTiles::FIRST_TILE]
            .iter()
            .all(|t| t.iter().all(|&c| c == 0)));
        assert!(
            vram[BellRoomTiles::FIRST_TILE].iter().any(|&c| c != 0),
            "$8800 tile 0 is the ring tile"
        );
        assert!(
            vram[signed_bg_tile_index(0)].iter().any(|&c| c != 0),
            "$9000 tile 0 is bell tile 128, not blank"
        );
    }

    #[test]
    fn test_signed_bg_tile_indexing() {
        assert_eq!(signed_bg_tile_index(0), 256, "0 -> $9000 (tile 256)");
        assert_eq!(signed_bg_tile_index(1), 257);
        assert_eq!(signed_bg_tile_index(127), 383, "127 -> $97F0 (tile 383)");
        assert_eq!(signed_bg_tile_index(128), 128, "128 -> $8800 (tile 128)");
        assert_eq!(signed_bg_tile_index(255), 255, "255 -> $8FF0 (tile 255)");
    }

    #[test]
    fn test_overworld_tilemaps_indexing_and_screen_slices() {
        let mut blob = vec![0u8; OverworldTilemaps::TOTAL_LEN];
        blob[0] = 42; // Hub (0, 0)
        blob[63] = 43; // Hub (63, 0)
        blob[63 * 64 + 63] = 44; // Hub (63, 63)
                                 // Screen 1 (Pumpkin)
        let s1_off = OverworldTilemaps::HUB_LEN;
        blob[s1_off] = 77;
        blob[s1_off + 31] = 78;

        let maps = OverworldTilemaps::from_bytes(&blob).expect("valid tilemaps blob");
        assert_eq!(maps.hub_tile(0, 0), 42);
        assert_eq!(maps.hub_tile(63, 0), 43);
        assert_eq!(maps.hub_tile(63, 63), 44);
        assert_eq!(maps.hub_tile(64, 64), 0, "out of bounds clamps to 0");

        assert_eq!(maps.screen_tile(1, 0, 0), 77);
        assert_eq!(maps.screen_tile(1, 31, 0), 78);
        assert_eq!(maps.screen_tile(1, 32, 0), 0, "out of bounds clamps to 0");

        let s1_slice = maps.screen_tilemap(1).expect("screen 1 slice");
        assert_eq!(s1_slice[0], 77);
        assert_eq!(s1_slice[31], 78);
    }

    #[test]
    fn test_overworld_manifest_route_patches_and_rendering() {
        let manifest_json = r#"{
            "screens": [
                {
                    "id": 1,
                    "name": "pumpkin",
                    "width": 32,
                    "height": 18,
                    "tilemap_offset": 4096,
                    "tilemap_len": 576,
                    "gfx": "gfx/overworld/pumpkinzone.png",
                    "bg_palette": 228
                }
            ],
            "patches": [
                {
                    "id": 0,
                    "ptr": "$6ECA",
                    "tiles": [
                        { "dx": 0, "dy": 0, "tile": 5 },
                        { "dx": 1, "dy": 0, "tile": 6 }
                    ]
                }
            ],
            "cursor_poses": [
                {
                    "pose": 0,
                    "direction": "Left",
                    "step": 0,
                    "tiles": [
                        { "dx": 0, "dy": 0, "tile": 1, "attr": 0 },
                        { "dx": 8, "dy": 0, "tile": 2, "attr": 0 },
                        { "dx": 0, "dy": 8, "tile": 3, "attr": 0 },
                        { "dx": 8, "dy": 8, "tile": 4, "attr": 0 }
                    ]
                }
            ],
            "dynamic_unlocks": [
                {
                    "name": "test_unlock",
                    "screen_id": 1,
                    "progress_offset": 2,
                    "mask": 1,
                    "patches": [
                        { "x": 0, "y": 0, "patch_ids": [0] }
                    ]
                }
            ]
        }"#;

        let manifest = OverworldManifest::from_json_str(manifest_json).expect("valid manifest");
        assert_eq!(manifest.screens.len(), 1);
        assert_eq!(manifest.patches.len(), 1);
        assert_eq!(manifest.cursor_poses.len(), 1);

        let tilemaps_blob = vec![0u8; OverworldTilemaps::TOTAL_LEN];
        let tilemaps = OverworldTilemaps::from_bytes(&tilemaps_blob).expect("valid tilemaps");

        // Prepare dummy VRAM tiles
        let mut vram = vec![[0u8; 64]; 384];
        let signed_tile_5 = signed_bg_tile_index(5);
        let signed_tile_6 = signed_bg_tile_index(6);
        vram[signed_tile_5].fill(1);
        vram[signed_tile_6].fill(2);
        vram[1].fill(3); // cursor top-left tile

        // Render before unlock: progress byte at offset 2 is 0 -> tile is 0
        let progress = [0u8; 42];
        let frame_locked = render_overworld_view(
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            &[],
            None,
            false,
            0,
            &tilemaps,
            &manifest,
            &progress,
            &vram,
            0xE4,
            0xD0,
            160,
            144,
        );
        assert_eq!(frame_locked.len(), 160 * 144);

        // Render with unlock: progress byte at offset 2 has bit 0 set
        let mut progress_unlocked = [0u8; 42];
        progress_unlocked[2] = 1;
        let frame_unlocked = render_overworld_view(
            1,
            0,
            0,
            8,
            16,
            0,
            0,
            0,
            0,
            &[],
            None,
            false,
            0, // OAM (8,16) -> screen (0,0)
            &tilemaps,
            &manifest,
            &progress_unlocked,
            &vram,
            0xE4,
            0xD0,
            160,
            144,
        );
        assert_eq!(frame_unlocked.len(), 160 * 144);
        // At (0, 0), cursor tile 1 (color 3) should be drawn on top
        let obj_shades = crate::video::bgp_shades(0xD0);
        assert_eq!(
            frame_unlocked[0], obj_shades[3],
            "cursor sprite draws over background"
        );
    }

    #[test]
    fn test_overworld_tiles_decoding() {
        let mut blob = vec![0u8; OverworldTiles::TOTAL_LEN];
        // In screen 1, tile 0 is 16 bytes: fill with 0xFF -> color 3 everywhere
        let s1_off = OverworldTiles::BYTES_PER_SCREEN;
        blob[s1_off..s1_off + 16].fill(0xFF);

        let tiles = OverworldTiles::from_bytes(&blob).expect("valid tiles blob");
        assert_eq!(tiles.bytes.len(), OverworldTiles::TOTAL_LEN);

        let vram = tiles.screen_vram_tiles(1);
        assert_eq!(vram.len(), OverworldTiles::TILES_PER_SCREEN);
        assert_eq!(
            vram[0], [3u8; 64],
            "tile 0 should be decoded to color 3 pixels"
        );
        assert_eq!(vram[1], [0u8; 64], "tile 1 should be zero");

        // Invalid blob size rejected
        assert!(OverworldTiles::from_bytes(&blob[..100]).is_none());
    }
}

#[cfg(test)]
mod cursor_tests {
    use super::*;

    /// `$3C901`: move bit 7 is the camera scroll, drawn as the two-frame
    /// `$4B77` arrow pair that blinks on `walk_frame` bit 3.
    #[test]
    fn camera_scroll_draws_the_blinking_arrows_not_the_cursor() {
        assert_eq!(
            cursor_sprite(0x80, 0, 0x00, 0),
            CursorSprite::ScrollArrows(0)
        );
        assert_eq!(
            cursor_sprite(0x80, 0, 0x08, 0),
            CursorSprite::ScrollArrows(1)
        );
        assert_eq!(
            cursor_sprite(0x80, 0, 0x10, 0),
            CursorSprite::ScrollArrows(0)
        );
        assert_eq!(SCROLL_ARROWS[0][0], (0x14, 0x00));
        assert_eq!(SCROLL_ARROWS[1][3], (0x16, 0x20));
        // The comparison is the whole nibble ($3C901's `cp $08`), so bit 7 with
        // bit 5 set is NOT the scroll case: $A0 keeps the pose metasprite
        // (`cp $0A`), while $82 still reaches the arrows.
        assert_eq!(cursor_sprite(0xA2, 1, 0x04, 3), CursorSprite::Pose(3));
        assert_eq!(
            cursor_sprite(0x82, 1, 0x04, 3),
            CursorSprite::ScrollArrows(0)
        );
    }

    /// `$3C92A`: bit 6 of the high nibble blanks the cursor ($4B87).
    #[test]
    fn move_bit_6_hides_the_cursor() {
        assert_eq!(cursor_sprite(0x40, 0, 0, 12), CursorSprite::Hidden);
        assert_eq!(cursor_sprite(0x44, 1, 0x0C, 1), CursorSprite::Hidden);
        assert_eq!(HIDDEN_CELLS[0].0, 0x7F);
    }

    /// `$3C959`/`$3C933`: while a walk stream runs, move bits 1/0 pick the
    /// four-frame edge arrows; without them the stump cycles `$498C` = 0,8,0,16.
    #[test]
    fn walk_stream_picks_the_edge_arrows_and_the_stump() {
        assert_eq!(cursor_sprite(0x22, 1, 0x00, 0), CursorSprite::EdgeRight(0));
        assert_eq!(cursor_sprite(0x22, 1, 0x04, 0), CursorSprite::EdgeRight(1));
        assert_eq!(cursor_sprite(0x22, 1, 0x08, 0), CursorSprite::EdgeRight(2));
        assert_eq!(cursor_sprite(0x22, 1, 0x0C, 0), CursorSprite::EdgeRight(3));
        assert_eq!(cursor_sprite(0x21, 1, 0x0C, 0), CursorSprite::EdgeLeft(3));
        assert_eq!(EDGE_LEFT[3][0], (0x13, 0x20));
        // walking == 0 goes straight to the stump, and so does walking with
        // neither direction bit.
        assert_eq!(cursor_sprite(0x20, 0, 0x00, 0), CursorSprite::Stump(0));
        assert_eq!(cursor_sprite(0x20, 1, 0x08, 0), CursorSprite::Stump(1));
        assert_eq!(cursor_sprite(0x20, 1, 0x10, 0), CursorSprite::Stump(0));
        assert_eq!(cursor_sprite(0x20, 1, 0x18, 0), CursorSprite::Stump(2));
        assert_eq!(WALK_STUMP[2][0], (0x1A, 0x20));
    }

    /// `$3C92E`/`$3C933`: the nibble-$0A case and a clear bit 1 both keep the
    /// pose metasprite the $4AD7 table picked.
    #[test]
    fn pose_table_cases_keep_the_cursor_metasprite() {
        assert_eq!(cursor_sprite(0x00, 0, 0, 12), CursorSprite::Pose(12));
        assert_eq!(cursor_sprite(0x0A, 1, 0, 3), CursorSprite::Pose(3));
        assert_eq!(cursor_sprite(0xA0, 1, 0x0C, 5), CursorSprite::Pose(5));
        assert_eq!(cursor_sprite(0x01, 0, 0, 0x1C), CursorSprite::Pose(0x0C));
    }
}

#[cfg(test)]
mod cursor_render_tests {
    use super::*;

    /// A 2x2 pose metasprite (tiles 0,1,4,5) and the scroll arrows (tiles
    /// $14-$17) are drawn from the same VRAM block; give each tile a distinct
    /// shade so the test can tell which one reached the screen.
    fn assets() -> (OverworldTilemaps, OverworldManifest, Vec<[u8; 64]>) {
        let mut vram = vec![[0u8; 64]; 384];
        vram[0x00] = [3u8; 64]; // pose top-left
        vram[0x01] = [3u8; 64];
        vram[0x04] = [3u8; 64];
        vram[0x05] = [3u8; 64];
        vram[0x14] = [1u8; 64]; // scroll arrow, colour 1
        vram[0x15] = [1u8; 64];
        vram[0x16] = [1u8; 64];
        vram[0x17] = [1u8; 64];
        let manifest = OverworldManifest::from_json_str(
            r#"{"cursor_poses":[{"direction":"Left","pose":0,"step":0,"tiles":[
                {"attr":0,"dx":0,"dy":0,"tile":0},{"attr":0,"dx":8,"dy":0,"tile":1},
                {"attr":0,"dx":0,"dy":8,"tile":4},{"attr":0,"dx":8,"dy":8,"tile":5}]}],
                "dynamic_unlocks":[],"patches":[],"screens":[]}"#,
        )
        .expect("manifest");
        (OverworldTilemaps::default(), manifest, vram)
    }

    fn draw(move_: u8, walking: u8, walk_frame: u8) -> Vec<u8> {
        draw_pan(move_, walking, walk_frame, false, 0)
    }

    fn draw_pan(move_: u8, walking: u8, walk_frame: u8, pan: bool, frame_counter: u8) -> Vec<u8> {
        let (tilemaps, manifest, mut vram) = assets();
        vram[0x7E] = [2u8; 64]; // $4A11's arrow tiles
        vram[0x39] = [2u8; 64];
        render_overworld_view(
            1,
            0,
            0,
            8,
            16,
            move_,
            walking,
            walk_frame,
            0,
            &[],
            None,
            pan,
            frame_counter,
            &tilemaps,
            &manifest,
            &[],
            &vram,
            0xE4,
            0xD0,
            160,
            144,
        )
    }

    /// The blink is `$FF97` bit 4 (`swap a / and $01`), so the arrows run for
    /// sixteen frames and blank for sixteen: measured on the console, B+Right
    /// on the hub gave frames 30..45 of `$7E` and 18..29 / 46..47 of `$7F`.
    #[test]
    fn the_pan_arrows_blink_every_sixteen_frames() {
        let obj = crate::video::bgp_shades(0xD0);
        let runs: Vec<bool> = (0..64u16)
            .map(|f| draw_pan(0x00, 0, 0, true, f as u8)[8 * 160 + 72] == obj[2])
            .collect();
        let mut lengths: Vec<(bool, usize)> = Vec::new();
        for &v in &runs {
            match lengths.last_mut() {
                Some((cur, len)) if *cur == v => *len += 1,
                _ => lengths.push((v, 1)),
            }
        }
        assert_eq!(lengths[0], (true, 16), "visible half: {lengths:?}");
        assert_eq!(lengths[1], (false, 16), "blank half: {lengths:?}");
        assert_eq!(lengths[2], (true, 16), "and it repeats: {lengths:?}");
    }

    /// `$3C9F4`: while the hub pans (B held) OAM 0-3 becomes the `$4A11` border
    /// arrows - top (72,8), bottom (72,124), left (8,68), right (144,68) - and
    /// `$4A21` blanks them on `$FF97` bit 7. The cursor is not drawn at all.
    #[test]
    fn panning_draws_the_border_arrows_and_hides_the_cursor() {
        let obj = crate::video::bgp_shades(0xD0);
        let bg = crate::video::bgp_shades(0xE4);
        let panning = draw_pan(0x00, 0, 0, true, 0x00);
        // Top arrow: OAM y 24, x 80 -> screen (72, 8).
        assert_eq!(panning[8 * 160 + 72], obj[2], "top arrow");
        assert_eq!(panning[68 * 160 + 8], obj[2], "left arrow");
        assert_eq!(panning[68 * 160 + 144], obj[2], "right arrow");
        assert_eq!(panning[124 * 160 + 72], obj[2], "bottom arrow");
        // The cursor's pose tiles sit at (0,0) for OAM (8,16); panning must not
        // draw them, and the blank half of the blink draws nothing.
        assert_eq!(panning[0], bg[0], "no cursor while panning");
        let blank = draw_pan(0x00, 0, 0, true, 0x10);
        assert_eq!(blank[8 * 160 + 72], bg[0], "$4A21 blank frame");
        assert_eq!(blank[68 * 160 + 8], bg[0]);
        // Without panning the arrows are absent and the cursor is back.
        let idle = draw_pan(0x00, 0, 0, false, 0x00);
        assert_eq!(idle[8 * 160 + 72], bg[0], "no arrows when not panning");
        assert_eq!(idle[0], obj[3], "cursor pose draws");
    }

    #[test]
    fn the_camera_scroll_arrows_replace_the_cursor_metasprite() {
        let obj = crate::video::bgp_shades(0xD0);
        let bg = crate::video::bgp_shades(0xE4);
        let idle = draw(0x00, 0, 0);
        let scroll = draw(0x80, 0, 0);
        let hidden = draw(0x40, 0, 0);

        assert_eq!(idle[0], obj[3], "idle draws the pose tiles");
        assert_eq!(scroll[0], obj[1], "camera scroll draws the $4B77 arrow");
        assert_eq!(hidden[0], bg[0], "bit 6 blanks the cursor entirely");
        // The second blink frame is a different arrow tile, so the pair blinks.
        assert_eq!(draw(0x80, 0, 0x08)[0], obj[1], "frame 1 still an arrow");
        assert_eq!(draw(0x80, 0, 0x08)[8], obj[1], "second cell of the pair");
        assert_ne!(scroll[0], idle[0]);
    }
}

#[cfg(test)]
mod wave_tests {
    use super::*;

    fn bytes(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).expect("hex"))
            .collect()
    }

    const STRIP0: &str = "0f 00 00 00 70 00 7c 00 ff 00 0f 00 f8 00 ff 00 \
                          ff 00 0f 00 7e 00 7c 00 01 00 fc 00 7f 00 f7 00";
    const STRIP1: &str = "fc 00 f1 00 1f 00 1b 00 f1 00 ff 00 1f 00 e0 00 \
                          07 00 ff 00 c0 00 8c 00 e7 00 8f 00 fc 00 ff 00";
    const STRIP2: &str = "00 1f 00 00 00 70 54 28 f3 0c 00 0f 80 78 f1 0e \
                          00 ff 09 06 02 7c 70 0c 01 00 9c 60 00 7f f0 07";
    const STRIP3: &str = "00 fc 31 c0 13 0c 02 19 70 81 60 9f 15 0a e0 00 \
                          00 07 28 d7 c0 00 8c 00 00 e7 80 0f 04 f8 9f 60";
    const AFTER0: &str = "0f 00 00 00 70 00 7c 00 ff 00 0f 00 f0 00 ff 00 \
                          ff 00 0f 00 7e 00 7c 00 01 00 fc 00 ff 00 fb 00";
    const AFTER1: &str = "fc 00 f1 00 1f 00 1b 00 f1 00 ff 00 3f 00 f0 00 \
                          07 00 ff 00 c0 00 8c 00 e7 00 8f 00 f8 00 7f 00";
    const AFTER2: &str = "00 1f 00 00 00 70 54 28 f3 0c 00 0f 00 f0 78 87 \
                          00 ff 09 06 02 7c 70 0c 01 00 9c 60 01 fe f8 03";
    const AFTER3: &str = "00 fc 31 c0 13 0c 02 19 70 81 60 9f 2a 15 f0 00 \
                          00 07 28 d7 c0 00 8c 00 00 e7 80 0f 08 f0 4f 30";
    /// Tile `$8E` after the two frames, i.e. the `$88E0` half of strip 0.
    const AFTER_TILE_8E: &str = "0f 00 00 00 70 00 7c 00 ff 00 0f 00 f0 00 ff 00";

    /// Measured on PyBoy (hub, `sMapRasterPhase = $DD`): two console frames later
    /// ($DE then $DF) the four `sMapWaveStrip` buffers hold exactly this. The
    /// pair covers both passes - `$DE` is even and rotates left at `e = 12`,
    /// `$DF` is odd and rotates right at `e = 14` and again at `e = 15` for
    /// strips 2 and 3.
    #[test]
    fn wave_rotation_matches_the_console() {
        let mut wave = MapWave {
            phase: 0xDD,
            strips: [[0u8; 32]; 4],
            raw: [[0u8; 16]; 8],
        };
        for (n, hex) in [STRIP0, STRIP1, STRIP2, STRIP3].iter().enumerate() {
            wave.strips[n].copy_from_slice(&bytes(hex));
        }
        // Tile $8E is strip 0's first half, so seeding the raw block from the
        // strips keeps the two views consistent for `apply`.
        wave.raw[0].copy_from_slice(&bytes(STRIP0)[..16]);

        wave.tick();
        wave.tick();

        assert_eq!(wave.phase, 0xDF);
        for (n, hex) in [AFTER0, AFTER1, AFTER2, AFTER3].iter().enumerate() {
            assert_eq!(wave.strips[n].to_vec(), bytes(hex), "strip {n}");
        }
        assert_eq!(wave.raw[0].to_vec(), bytes(AFTER_TILE_8E), "tile $8E bytes");
        // Every phase writes eight bytes: four strips at `e`, strips 2/3 at `e|1`.
        assert_eq!(wave.strips[0][0x0F], bytes(AFTER0)[0x0F]);
        assert_eq!(wave.strips[2][0x0F], bytes(AFTER2)[0x0F]);
    }

    /// `$3E7BE`: each strip is the sixteen bytes of its even tile followed by the
    /// sixteen of its odd tile.
    #[test]
    fn seeding_copies_the_tile_pairs() {
        let mut block = vec![0u8; 384 * 16];
        for (i, tile) in WAVE_TILES.iter().enumerate() {
            for b in 0..16 {
                block[*tile as usize * 16 + b] = (i as u8) * 16 + b as u8;
            }
        }
        let wave = MapWave::seed(&block).expect("384-tile block");
        for n in 0..4 {
            assert_eq!(
                wave.strips[n][..16],
                wave.raw[2 * n][..],
                "strip {n} even tile"
            );
            assert_eq!(
                wave.strips[n][16..],
                wave.raw[2 * n + 1][..],
                "strip {n} odd tile"
            );
        }
        assert_eq!(wave.raw[0][0], 0x00);
        assert_eq!(wave.raw[7][15], 7 * 16 + 15);
        // `apply` writes the rotated bytes back through the tile decoder.
        let mut vram = vec![[0u8; 64]; 384];
        wave.apply(&mut vram);
        assert_eq!(vram[0x8E], crate::video::tile_colors(&wave.raw[0]));
    }
}

#[cfg(test)]
mod decor_tests {
    use super::*;

    /// `$4C4D` + the metasprite cells behind it, byte for byte from bank 15.
    /// `$3D01D` terminates on a `$FF` *tile*, not a `$FF` dy, so no parsed cell
    /// may carry one.
    #[test]
    fn metasprite_data_matches_the_rom() {
        assert_eq!(DECOR_METASPRITES.len(), 17);
        assert_eq!(DECOR_METASPRITES[0], &[], "ID 0 is the empty metasprite");
        assert_eq!(DECOR_METASPRITES[1].len(), 18, "ID 1, $4C73");
        assert_eq!(DECOR_METASPRITES[5].len(), 9, "ID 5, $4CFB");
        assert_eq!(DECOR_METASPRITES[16].len(), 1, "ID 16, $4E2F");
        assert_eq!(
            &DECOR_METASPRITES[5][..3],
            &[(0, 0, 0x3A, 0x00), (0, 8, 0x3B, 0x00), (0, 16, 0x3C, 0x00)][..],
            "ID 5's first row"
        );
        assert!(
            DECOR_METASPRITES
                .iter()
                .flat_map(|m| m.iter())
                .all(|c| c.2 != 0xFF),
            "the $FF tile is the terminator, never a cell"
        );
    }

    /// The 72 bytes `$3CBE7` copies out of `$4BFD` onto the hub.
    #[test]
    fn hub_slot_list_matches_the_rom() {
        let live: Vec<(u8, u8, u8, u8)> = HUB_DECOR_SLOTS
            .iter()
            .copied()
            .filter(|s| s.3 != 0xFF)
            .collect();
        assert_eq!(
            live,
            vec![
                (0x00, 0x4C, 0x4F, 0x01),
                (0x10, 0xA8, 0xF0, 0x09),
                (0x01, 0xC0, 0x28, 0x09),
                (0x11, 0x88, 0x70, 0x05),
            ],
            "four live slots, the rest $FF"
        );
        assert_eq!(HUB_DECOR_SLOTS.len(), 18);
    }

    /// Measured on the console: at `cam = ($BB, $0142)` the hub staged OAM entry
    /// 13 as `y=62 x=213 tile=$3A attr=$00`, which is slot 14's first cell with
    /// `$3D044`'s `x = slot.x - cam.x + 8` / `y = slot.y - cam.y + 16`.
    #[test]
    fn slot_14_stages_the_cell_the_console_staged() {
        let cells = decor_oam((0x11, 0x88, 0x70, 0x05), 0x00BB, 0x0142).expect("in window");
        assert_eq!(cells.len(), 9);
        assert_eq!(cells[0], (62, 213, 0x3A, 0x00));
    }

    /// `$3D044`'s window is `cam - $30 .. cam + $D0` on both axes, in the
    /// bank-tagged 16-bit space the ROM compares in. Transcribed from the
    /// listing (the hub's live list is updater-driven, so this has no console
    /// sample of its own yet).
    #[test]
    fn the_camera_window_is_twenty_four_plus_two_oh_eight() {
        // Slot 0 of the hub at the idle camera is behind the lower bound.
        assert!(decor_oam(HUB_DECOR_SLOTS[0], 0x0081, 0x0142).is_none());
        // The same slot well inside the view is kept.
        assert!(decor_oam((0x00, 0x60, 0x60, 0x01), 0x0040, 0x0040).is_some());
        // Empty slots never expand.
        assert!(decor_oam((0x00, 0x60, 0x60, 0xFF), 0x0040, 0x0040).is_none());
        // An out-of-range ID is refused rather than indexing past the table.
        assert!(decor_oam((0x00, 0x60, 0x60, 0x20), 0x0040, 0x0040).is_none());
    }
}

#[cfg(test)]
mod decor_idle_probe {
    use super::*;

    /// Documents why the static hub list is not wired into the renderer: at the
    /// hub's idle camera the console stages **no** decor entries (PyBoy OAM 13+
    /// is empty), yet the static list plus `$3D044` keeps slot 3. Something
    /// rewrites the list before the expander runs, so the live values have to
    /// come from the per-screen updaters.
    #[test]
    fn the_static_hub_list_disagrees_with_the_idle_console() {
        let kept: Vec<usize> = HUB_DECOR_SLOTS
            .iter()
            .enumerate()
            .filter(|(_, s)| decor_oam(**s, 0x0081, 0x0142).is_some())
            .map(|(i, _)| i)
            .collect();
        assert!(
            !kept.is_empty(),
            "if this ever becomes empty the static list could be rendered as-is"
        );
    }
}

#[cfg(test)]
mod decor_render_tests {
    use super::*;

    fn scene() -> (OverworldTilemaps, OverworldManifest, Vec<[u8; 64]>) {
        let mut vram = vec![[0u8; 64]; 384];
        for t in 0x20..=0x27 {
            vram[t] = [2u8; 64]; // ID 9's tiles, all colour 2
        }
        let manifest = OverworldManifest::from_json_str(
            r#"{"cursor_poses":[],"dynamic_unlocks":[],"patches":[],"screens":[]}"#,
        )
        .expect("manifest");
        (OverworldTilemaps::default(), manifest, vram)
    }

    fn draw(slots: &[(u8, u8, u8, u8)], cam_y: u16) -> Vec<u8> {
        let (tilemaps, manifest, vram) = scene();
        let cells = hub_decor_cells(slots, 0x0081, cam_y);
        render_overworld_view(
            0,
            0x0081,
            cam_y,
            8,
            16,
            0,
            0,
            0,
            0,
            &cells,
            None,
            false,
            0,
            &tilemaps,
            &manifest,
            &[],
            &vram,
            0xE4,
            0xD0,
            160,
            144,
        )
    }

    /// Measured on PyBoy at the hub's idle camera: the expansion is exactly these
    /// eight cells (slot 3, ID 9), all staged just above the top edge at y=246/254.
    #[test]
    fn the_idle_hub_expands_the_eight_cells_the_console_stages() {
        let mut all = Vec::new();
        for slot in HUB_DECOR_SLOTS {
            if let Some(cells) = decor_oam(slot, 0x0081, 0x0142) {
                all.extend(cells);
            }
        }
        assert_eq!(
            all,
            vec![
                (246, 71, 0x20, 0x00),
                (246, 79, 0x21, 0x00),
                (246, 87, 0x22, 0x00),
                (246, 95, 0x23, 0x00),
                (254, 71, 0x24, 0x00),
                (254, 79, 0x25, 0x00),
                (254, 87, 0x26, 0x00),
                (254, 95, 0x27, 0x00),
            ]
        );
    }

    /// The same slot with the camera raised so the sprite is on screen: the
    /// decor pass paints it, and it is painted before the cursor.
    #[test]
    fn decor_paints_when_the_camera_brings_it_on_screen() {
        let obj = crate::video::bgp_shades(0xD0);
        let with = draw(&HUB_DECOR_SLOTS, 0x0100);
        let without = draw(&[], 0x0100);
        assert_ne!(with, without, "the slot must paint something");
        // Slot 3 at cam (0x81, 0x100): screen (63, 40).
        assert_eq!(with[40 * 160 + 63], obj[2]);
        assert_ne!(without[40 * 160 + 63], obj[2]);
    }
}
/// `$5A21`: slot 0's id by `sMapAnimPhase` bits 3-5 - `$A6B7`, which is the
/// *third* byte of the 3-byte zone slot, i.e. its id.
pub const PUMPKIN_ID_Y: [u8; 8] = [0x0F, 0x0F, 0x0F, 0x10, 0x11, 0x11, 0x11, 0x11];

/// `$5A29`: slot 1's id by `sMapAnimPhase` bits 2-3.
pub const PUMPKIN_ID_X: [u8; 4] = [0x12, 0x13, 0x14, 0x15];

/// `$5A16`: `sMapSprite2Page` - slot 2's id - is `22 + this[index]`, plus 3 unless
/// the step value has bit 7 set.
pub const PUMPKIN_PAGE: [u8; 16] = [
    0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x02, 0x01, 0x03, 0x0F, 0x0F, 0x0F, 0x10, 0x11,
];

/// `$6227`: the Pumpkin step table (`sMapAnimIndexA` walks it 0..$4F).
pub const PUMPKIN_STEPS: [u8; 80] = [
    0x19, 0x19, 0x19, 0x19, 0x19, 0x19, 0x10, 0x10, 0x19, 0x10, 0x10, 0x10, 0x11, 0x10, 0x11, 0x11,
    0x11, 0x01, 0x11, 0x01, 0x81, 0x91, 0x81, 0x91, 0x91, 0x91, 0x90, 0x91, 0x90, 0x90, 0x90, 0x99,
    0x90, 0x90, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x90, 0x90,
    0x99, 0x90, 0x90, 0x90, 0x91, 0x90, 0x91, 0x91, 0x91, 0x81, 0x91, 0x81, 0x01, 0x11, 0x01, 0x11,
    0x11, 0x11, 0x10, 0x11, 0x10, 0x10, 0x10, 0x19, 0x10, 0x10, 0x19, 0x19, 0x19, 0x19, 0x19, 0x19,
];

#[cfg(test)]
mod status_bar_tests {
    use super::*;

    /// The console's OAM 4-12 on the hub with the default five lives and an empty
    /// purse: (152,88,$7C) (152,96,$6F) (152,104,$70) (152,112,$75) (152,128,$7D)
    /// (152,136,$6F) (152,144,$70) (152,152,$70) (152,160,$70).
    #[test]
    fn status_bar_matches_the_console_row() {
        let bar = StatusBar {
            lives: 0x05,
            coins_low: 0x00,
            coins_high: 0x00,
        };
        assert_eq!(
            bar.cells(),
            [
                (152, 88, 0x7C, 0x00),
                (152, 96, 0x6F, 0x00),
                (152, 104, 0x70, 0x00),
                (152, 112, 0x75, 0x00),
                (152, 128, 0x7D, 0x00),
                (152, 136, 0x6F, 0x00),
                (152, 144, 0x70, 0x00),
                (152, 152, 0x70, 0x00),
                (152, 160, 0x70, 0x00),
            ]
        );
    }

    /// `$3F203` patches one nibble per tile, OR'd with `$70`: `sLives` high into
    /// entry 6 and low into 7, `sCoinHigh`'s low nibble into 10, `sCoinLow`'s
    /// nibbles into 11 and 12.
    #[test]
    fn status_bar_digits_are_nibbles_or_70() {
        let bar = StatusBar {
            lives: 0x07,
            coins_low: 0x43,
            coins_high: 0x01,
        };
        let c = bar.cells();
        assert_eq!((c[2].2, c[3].2), (0x70, 0x77), "lives 07");
        assert_eq!((c[6].2, c[7].2, c[8].2), (0x71, 0x74, 0x73), "coins 143");
    }

    /// The strip `$3F203` writes ($E1 over two rows at `$9C00`) is the bottom row,
    /// and the counters draw over it.
    #[test]
    fn the_strip_and_counters_render() {
        let (tilemaps, manifest, vram) = {
            // The bar's tiles ($6F, $70-$77, $7C, $7D) get a non-transparent
            // pattern so the counters are distinguishable from the strip.
            let mut v = vec![[0u8; 64]; 256];
            for t in [
                0x6Fu8, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x7C, 0x7D,
            ] {
                v[t as usize] = [3u8; 64]; // decoded 2bpp: colour 3
            }
            (
                OverworldTilemaps::default(),
                OverworldManifest::default(),
                v,
            )
        };
        let draw = |bar: Option<&StatusBar>| {
            render_overworld_view(
                0,
                0,
                0,
                8,
                16,
                0,
                0,
                0,
                0,
                &[],
                bar,
                false,
                0,
                &tilemaps,
                &manifest,
                &[],
                &vram,
                0xE4,
                0xD0,
                160,
                144,
            )
        };
        let plain = draw(None);
        let with_bar = draw(Some(&StatusBar {
            lives: 0x07,
            coins_low: 0x43,
            coins_high: 0x00,
        }));
        // The panel is the bottom row's right half - from the bar's first sprite
        // at x 80 to the edge - not a band across the whole map.
        assert_eq!(status_bar_strip_rect(160, 144), (80, 136, 160, 144));
        let shades = crate::video::bgp_shades(0xE4);
        for x in [80usize, 100, 140, 159] {
            assert_eq!(plain[136 * 160 + x], shades[0], "strip at x {x}");
        }
        let band: Vec<usize> = (136 * 160..144 * 160)
            .filter(|i| plain[*i] != with_bar[*i])
            .collect();
        assert!(!band.is_empty(), "the counters draw in the strip's row");
    }
}

#[cfg(test)]
mod reveal_render_tests {
    use super::*;

    fn reveal() -> CastleRevealTiles {
        let tiles = std::fs::read("../../extracted/map/castle_reveal_tiles.bin")
            .or_else(|_| std::fs::read("extracted/map/castle_reveal_tiles.bin"))
            .expect("tiles");
        let map = std::fs::read("../../extracted/map/castle_reveal_tilemap.bin")
            .or_else(|_| std::fs::read("extracted/map/castle_reveal_tilemap.bin"))
            .expect("tilemap");
        CastleRevealTiles::from_bytes(&tiles, &map).expect("reveal")
    }

    #[test]
    fn the_six_coins_patch_the_five_runs() {
        let reveal = reveal();
        let metas = crate::room_sprites::SpriteMetas::default();
        let plain = render_castle_reveal(&reveal, false, 0xE4, 0xD0, None, &metas, 160, 144);
        let patched = render_castle_reveal(&reveal, true, 0xE4, 0xD0, None, &metas, 160, 144);
        assert_ne!(plain, patched, "tile $74 lands in the five runs");
        assert!(plain.iter().any(|&p| p != plain[0]), "the screen is drawn");
        // The runs sit at rows 10-14, so the first row is untouched.
        assert_eq!(&plain[..160], &patched[..160]);
    }
}
