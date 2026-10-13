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
    /// The extracted blob: the level table then the progress-slot table.
    pub const BLOB_LEN: usize = 2 * Self::NODES;

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

    /// `$650E[node]` — the `sLevelProgress` offset (and `$6591` index), or
    /// `None` when the node has neither.
    pub fn progress_slot(&self, node: u8) -> Option<u8> {
        let v = *self.blob.get(Self::NODES + node as usize)?;
        (v < 0xFE).then_some(v)
    }
}

/// The five bytes the fresh-file init writes after the checksum, which `$28EC`
/// checks before it will accept a slot. `$2934` itself never rewrites them.
pub const SAVE_SIGNATURE: [u8; 5] = [0x12, 0x34, 0x56, 0x78, 0x78];

/// `$28EC`'s validation of one 0x50-byte save record: the checksum at +$45 must
/// be the sum of the 69 bytes before it, and the five signature bytes at +$46
/// must be the constant the fresh-file init writes.
pub fn save_record_valid(record: &[u8]) -> bool {
    if record.len() < 0x4B {
        return false;
    }
    let sum = record[..0x45].iter().fold(0u8, |c, b| c.wrapping_add(*b));
    sum == record[0x45] && record[0x46..0x4B] == SAVE_SIGNATURE
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
        FE FE FE FE FE FE FE FE FE FE 26 FE 27 FE FE FE FE FE FE FE";

    fn nodes() -> MapNodes {
        let bytes: Vec<u8> = BLOB_HEX
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).unwrap())
            .collect();
        MapNodes::from_bytes(&bytes).expect("260-byte blob")
    }

    /// Node 0 is level 1 and a plain level — the case the `$14` hand-off's tail
    /// keys on (`or a / jr nz` means the progress mark happens only for node 0).
    #[test]
    fn node_zero_is_level_one_and_a_plain_level() {
        let n = nodes();
        assert_eq!(n.level_for(0), Some(0), "$4218[0] = 0 = level 1");
        assert_eq!(n.progress_slot(0), Some(0), "$650E[0] = 0");
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

    /// `$28EC`: the checksum is the sum of the 69 bytes before it, and the
    /// signature bytes must be the init constant.
    #[test]
    fn the_save_validator_checks_sum_and_signature() {
        let mut rec = [0u8; 0x50];
        rec[0x40] = 3;
        rec[0x43] = 5;
        let sum = rec[..0x45].iter().fold(0u8, |c, b| c.wrapping_add(*b));
        rec[0x45] = sum;
        rec[0x46..0x4B].copy_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x78]);
        assert!(save_record_valid(&rec));

        rec[0x45] = sum.wrapping_add(1);
        assert!(!save_record_valid(&rec), "a bad checksum fails");
        rec[0x45] = sum;
        rec[0x4A] = 0x00;
        assert!(!save_record_valid(&rec), "a bad signature fails");
        assert!(!save_record_valid(&[0u8; 8]), "a short record fails");
    }
}
