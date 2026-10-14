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
    /// `$A84D` Tree Zone screen A gold coin (bit 7 = collected).
    pub const GOLDEN_COIN_1: usize = 5;
    /// `$A853` Macro Zone path 0 (bit 0 gates the `$3C2E7` cursor move).
    pub const MACRO_PATH_0: usize = 11;
    /// `$A855` Macro Zone path 2 (bit 7 gates the `$3C2B8` A-move).
    pub const MACRO_PATH_2: usize = 13;
    /// `$A86B` Macro gate — set when node `$6A` completes.
    pub const GATE_MACRO: usize = 35;
    /// `$A86D` Mario-Zone gate — set when node `$66` completes.
    pub const GATE_MARIO: usize = 37;
    /// `$A86E` Pumpkin-Zone gate B — set when node `$78` completes.
    pub const GATE_PUMPKIN_B: usize = 38;
    /// `$A86F` Pumpkin-Zone gate C — set when node `$7A` completes.
    pub const GATE_PUMPKIN_C: usize = 39;
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
}
