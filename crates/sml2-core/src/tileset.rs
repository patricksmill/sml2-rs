//! `$25AF` — the `$6x` scene opcode's tileset swap, and the reason a boss arena
//! does not look like the level it sits in.
//!
//! Every arena intro script starts with `$6x`, whose own nibble is
//! `sLevelGfxIndex` (the operand-length note in [`crate::scene`]): the follow-up
//! reload of the *same* level then runs `$25AF`, which overwrites three VRAM
//! regions and swaps the metatile definitions:
//!
//! ```text
//! bank 7  $7800 -> $8E80   0x50 bytes   (fixed source, no index)
//! bank    $2699[idx*4]     -> $9200   0x600 bytes  from bank $268B[idx*2]
//! bank    $2699[idx*4 + 2] -> $8B00   0x400 bytes  from bank $268B[idx*2 + 1]
//! $26B5[idx] -> sMetatileMap           the Level_TileMap_NN the index names
//! ```
//!
//! Both VRAM regions live inside the level's `$8800-$97FF` tile block, which the
//! port holds as `levelNN.bgblock.bin` (`$8800` = offset 0): `$8E80` = `0x680`,
//! `$9200` = `0xA00`, `$8B00` = `0x300`. The OBJ tile space `$8000-$8FFF` is
//! the *same* VRAM, so the port's `levelNN.objblock.bin` high half
//! (`$8800-$8FFF` = its offset `0x800`) is `bgblock[0..0x800]` and has to be
//! repatched with it.
//!
//! Two quirks are reproduced literally:
//!
//! - The OBJ copy is a literal `0x400` bytes — 64 tiles, `0x20` past the
//!   `0x380`-byte enemy region it nominally fills. Because it spans
//!   `$8B00-$8EFF` it also lands on **all** of the `$8E80` block the bank-7 copy
//!   just wrote, so that copy is dead in effect (kept, and pinned by a test,
//!   because it is what the ROM does).
//! - The metatile *definitions* swap, not the layout: the arena is a screen of
//!   the same level, and its cells were authored against these definitions. The
//!   collision classes are not part of the swap.

use serde::{Deserialize, Serialize};

use crate::level::MetatileTable;

/// Schema id written into `extracted/scenes/tilesets.json`.
pub const TILESET_SCHEMA: &str = "sml2-tilesets/1";

/// BG tile region: VRAM `$9200` inside the `$8800`-based block.
pub const BG_TILES_OFFSET: usize = 0xA00;
/// The tileset's BG tile byte count (`$25AF`'s `ld bc,$0600`).
pub const BG_TILES_LEN: usize = 0x600;
/// OBJ/enemy tile region: VRAM `$8B00` inside the `$8800`-based block.
pub const OBJ_TILES_OFFSET: usize = 0x300;
/// The OBJ copy's literal byte count (`ld bc,$0400`) — 64 tiles, `0x20` past
/// the `0x380`-byte region the level load puts there.
pub const OBJ_TILES_LEN: usize = 0x400;
/// The bank-7 `$7800` block: VRAM `$8E80` inside the `$8800`-based block.
pub const EXTRA_OFFSET: usize = 0x680;
/// that copy's byte count (`ld bc,$0050`).
pub const EXTRA_LEN: usize = 0x50;
/// The OBJ tile space's high half (`$8800-$8FFF`) inside `objblock.bin`.
pub const OBJ_BLOCK_HIGH: usize = 0x800;

/// One `sLevelGfxIndex` entry: where its three copies come from and what its
/// metatile map resolves to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilesetEntry {
    /// The `$26B5` / `$268B` / `$2699` index, i.e. `sLevelGfxIndex`.
    pub index: u8,
    /// `$268B[idx * 2]` — the bank the BG tiles are copied from.
    pub bg_bank: u8,
    /// `$2699[idx * 4]` — that bank's CPU source address.
    pub bg_addr: u16,
    /// `$268B[idx * 2 + 1]` — the bank the OBJ/enemy tiles come from.
    pub obj_bank: u8,
    /// `$2699[idx * 4 + 2]`.
    pub obj_addr: u16,
    /// `$26B5[idx * 2]` resolved through the disassembly's labels: the
    /// `Level_TileMap_NN` metatile table the swap installs.
    pub map_index: u8,
    pub map_label: String,
    /// 0x600 bytes -> VRAM `$9200`.
    pub bg_tiles: Vec<u8>,
    /// 0x400 bytes -> VRAM `$8B00`.
    pub obj_tiles: Vec<u8>,
    /// 512 bytes: the `Level_TileMap_NN` definitions, in the same layout as
    /// [`MetatileTable`]'s own source bytes.
    pub metatiles: Vec<u8>,
}

/// The whole `$25AF` table (seven entries plus the index-independent bank-7
/// block), extracted once for the ROM.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tilesets {
    pub schema: String,
    pub notes: Vec<String>,
    /// Bank 7 `$7800`, 0x50 bytes, copied to `$8E80` on **every** swap.
    pub extra_8e80: Vec<u8>,
    pub entries: Vec<TilesetEntry>,
}

impl Tilesets {
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("tilesets.json: {e}"))
    }

    pub fn entry(&self, index: u8) -> Option<&TilesetEntry> {
        self.entries.iter().find(|e| e.index == index)
    }

    /// `$25AF`: patch the level's live tile data for a gfx index.
    ///
    /// `bg` is the `$8800-$97FF` tile block (`levelNN.bgblock.bin`), `objblock`
    /// the `$8000-$8FFF` OBJ block, `metatiles` the level's metatile table. All
    /// three are mutated in the ROM's own order; returns `false` (and changes
    /// nothing) for an index the table does not carry.
    pub fn apply(
        &self,
        index: u8,
        bg: &mut [u8],
        objblock: &mut [u8],
        metatiles: &mut MetatileTable,
    ) -> bool {
        if bg.len() < EXTRA_OFFSET + EXTRA_LEN
            || bg.len() < BG_TILES_OFFSET + BG_TILES_LEN
            || bg.len() < OBJ_TILES_OFFSET + OBJ_TILES_LEN
            || self.extra_8e80.len() != EXTRA_LEN
        {
            return false;
        }
        let Some(e) = self.entry(index) else {
            return false;
        };
        if e.bg_tiles.len() != BG_TILES_LEN || e.obj_tiles.len() != OBJ_TILES_LEN {
            return false;
        }
        let Ok(table) = MetatileTable::from_bytes(&e.metatiles) else {
            return false;
        };
        // The ROM's order: bank 7's $50 bytes at $8E80, the tileset's $600 at
        // $9200, then the $400-byte OBJ copy at $8B00 — whose length overruns
        // the enemy region on purpose.
        bg[EXTRA_OFFSET..EXTRA_OFFSET + EXTRA_LEN].copy_from_slice(&self.extra_8e80);
        bg[BG_TILES_OFFSET..BG_TILES_OFFSET + BG_TILES_LEN].copy_from_slice(&e.bg_tiles);
        bg[OBJ_TILES_OFFSET..OBJ_TILES_OFFSET + OBJ_TILES_LEN].copy_from_slice(&e.obj_tiles);
        // $8000-$8FFF shares VRAM with $8800-$8FFF: the OBJ block's high half is
        // the same bytes the BG block carries at its base.
        if objblock.len() >= OBJ_BLOCK_HIGH + 0x800 {
            objblock[OBJ_BLOCK_HIGH..OBJ_BLOCK_HIGH + 0x800].copy_from_slice(&bg[..0x800]);
        }
        *metatiles = table;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_of(byte: u8) -> Vec<u8> {
        (0..512).map(|i| byte.wrapping_add(i as u8)).collect()
    }

    /// The whole `$25AF` paste, on synthetic data so every region and its order
    /// is observable: bank 7's `$50` at `$8E80`, the tileset's `$600` at
    /// `$9200`, the literal `$400` at `$8B00` — whose 0x20-byte overrun lands on
    /// the `$8E80` block's first two tiles, which only happens in this order —
    /// the OBJ block's high half, and the metatile table.
    #[test]
    fn the_swap_pastes_every_region_in_the_roms_order() {
        let ts = Tilesets {
            schema: TILESET_SCHEMA.to_string(),
            notes: Vec::new(),
            extra_8e80: vec![0xAA; EXTRA_LEN],
            entries: vec![TilesetEntry {
                index: 1,
                bg_bank: 0x1B,
                bg_addr: 0x5800,
                obj_bank: 0x1B,
                obj_addr: 0x6C00,
                map_index: 27,
                map_label: "Level_TileMap_27".to_string(),
                bg_tiles: vec![0xBB; BG_TILES_LEN],
                obj_tiles: vec![0xCC; OBJ_TILES_LEN],
                metatiles: table_of(0x40),
            }],
        };
        let mut bg = vec![0x11u8; 0x1000];
        let mut objblock = vec![0x22u8; 0x1000];
        let mut metatiles = MetatileTable::from_bytes(&vec![0x33; 512]).unwrap();
        assert!(ts.apply(1, &mut bg, &mut objblock, &mut metatiles));

        assert_eq!(&bg[0xA00..0x1000], &[0xBB; BG_TILES_LEN][..], "$9200 <- BG");
        assert_eq!(
            &bg[0x300..0x700],
            &[0xCC; OBJ_TILES_LEN][..],
            "$8B00 <- OBJ, a literal $400 bytes"
        );
        // The bank-7 copy at $8E80 (0x680) writes first and the OBJ copy that
        // follows spans $8B00-$8EFF, so every one of its bytes is overwritten
        // again: the ROM's `ld bc,$0050 / ld hl,$7800` paste is dead in effect.
        // Kept because it is what the ROM does, and pinned so a future reorder
        // cannot quietly make it live.
        assert_eq!(
            &bg[0x680..0x6D0],
            &[0xCC; EXTRA_LEN][..],
            "the $8E80 copy is overwritten by the OBJ paste's overrun"
        );
        assert_eq!(&bg[0x000..0x300], &[0x11; 0x300][..], "untouched");
        assert_eq!(
            &objblock[OBJ_BLOCK_HIGH..OBJ_BLOCK_HIGH + 0x800],
            &bg[..0x800],
            "the OBJ block's high half shares the BG block's first $800"
        );
        assert_eq!(objblock[..OBJ_BLOCK_HIGH], [0x22; 0x800], "Mario's half stays");
        assert_eq!(
            metatiles.get(0).unwrap().tiles,
            [0x40, 0x41, 0x42, 0x43],
            "the metatile definitions swap, not the layout"
        );
    }

    /// An index the table does not carry, and short/missing inputs, must change
    /// nothing rather than paste a partial block.
    #[test]
    fn a_missing_index_or_a_short_table_changes_nothing() {
        let mut ts = Tilesets {
            schema: TILESET_SCHEMA.to_string(),
            notes: Vec::new(),
            extra_8e80: vec![0xAA; EXTRA_LEN],
            entries: vec![TilesetEntry {
                index: 1,
                bg_bank: 0x1B,
                bg_addr: 0x5800,
                obj_bank: 0x1B,
                obj_addr: 0x6C00,
                map_index: 27,
                map_label: "Level_TileMap_27".to_string(),
                bg_tiles: vec![0xBB; BG_TILES_LEN],
                obj_tiles: vec![0xCC; OBJ_TILES_LEN],
                metatiles: table_of(0x40),
            }],
        };
        let mut bg = vec![0x11u8; 0x1000];
        let mut objblock = vec![0x22u8; 0x1000];
        let mut metatiles = MetatileTable::from_bytes(&vec![0x33; 512]).unwrap();
        assert!(!ts.apply(3, &mut bg, &mut objblock, &mut metatiles), "no index 3");
        assert_eq!(bg, vec![0x11u8; 0x1000]);

        // A truncated block is refused as well: a half-pasted arena would look
        // like a graphics bug, not like a missing table.
        ts.entries[0].bg_tiles.truncate(BG_TILES_LEN - 16);
        assert!(!ts.apply(1, &mut bg, &mut objblock, &mut metatiles));
        assert_eq!(bg, vec![0x11u8; 0x1000]);
    }
}
