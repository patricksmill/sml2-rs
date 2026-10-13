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
