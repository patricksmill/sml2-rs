//! The battery-backed save slots at `$A000/$A050/$A0A0`.
//!
//! Each record is the 80-byte image the ROM's `$2934` writes and `$28EC`
//! validates: a 64-byte progress block, coin lo/hi, BCD level tally, lives,
//! money bags, a checksum, and four signature bytes. `$28D9` validates all
//! three slots when the file select opens; an invalid slot is replaced with the
//! `$2913`/`$30226` fresh-file template.

use crate::map::{save_record_valid, SAVE_SIGNATURE};

/// Bytes per save slot (`$A050 - $A000`).
pub const SLOT_LEN: usize = 0x50;
/// The three battery-backed slots.
pub const SLOT_COUNT: usize = 3;
/// The whole slot window as the ROM lays it out.
pub const SAVE_IMAGE_LEN: usize = SLOT_COUNT * SLOT_LEN;

/// `$2913`/`$30226`: the fresh-file template.
///
/// Sixty-four zero progress bytes, coins `0/0`, tally `0`, lives `5`, money
/// bags `0`, checksum `2`, then the signature. The checksum is **not** the sum
/// of the bytes before it (that sum is `5`), so the template deliberately fails
/// `$28EC`; a slot stays \"empty\" until a real save writes a matching checksum.
/// `+$4A` is not written by the template (or by `$2934`) and is not validated.
pub fn fresh_template() -> [u8; SLOT_LEN] {
    let mut record = [0u8; SLOT_LEN];
    record[0x43] = 5;
    record[0x45] = 2;
    record[0x46..0x4A].copy_from_slice(&SAVE_SIGNATURE);
    record
}

/// The three save records, stored exactly as the ROM's battery RAM holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSlots {
    slots: [[u8; SLOT_LEN]; SLOT_COUNT],
}

impl Default for SaveSlots {
    fn default() -> Self {
        Self {
            slots: [[0u8; SLOT_LEN]; SLOT_COUNT],
        }
    }
}

impl SaveSlots {
    /// A zeroed SRAM image, before `$28D9` has run.
    pub fn new() -> Self {
        Self::default()
    }

    /// Read a persisted image. Short images are zero-padded, so a truncated or
    /// legacy file still reaches `validate_and_init` instead of panicking.
    pub fn from_image(image: &[u8]) -> Self {
        let mut slots = Self::default();
        for (slot, chunk) in slots.slots.iter_mut().zip(image.chunks(SLOT_LEN)) {
            let n = chunk.len().min(SLOT_LEN);
            slot[..n].copy_from_slice(&chunk[..n]);
        }
        slots
    }

    /// The whole `$A000-$A0EF` window, for persistence.
    pub fn image(&self) -> [u8; SAVE_IMAGE_LEN] {
        let mut image = [0u8; SAVE_IMAGE_LEN];
        for (i, slot) in self.slots.iter().enumerate() {
            image[i * SLOT_LEN..(i + 1) * SLOT_LEN].copy_from_slice(slot);
        }
        image
    }

    /// `$28D9`: validate every slot and replace the invalid ones with the
    /// fresh-file template. Returns which slots were valid before the call.
    pub fn validate_and_init(&mut self) -> [bool; SLOT_COUNT] {
        let mut valid = [false; SLOT_COUNT];
        for (i, slot) in self.slots.iter_mut().enumerate() {
            valid[i] = save_record_valid(slot);
            if !valid[i] {
                *slot = fresh_template();
            }
        }
        valid
    }

    /// `$28EC`: one slot passes its checksum and signature.
    pub fn valid(&self, index: usize) -> bool {
        self.slot(index).is_some_and(|slot| save_record_valid(slot))
    }

    /// The raw record, whether or not it validates.
    pub fn slot(&self, index: usize) -> Option<&[u8; SLOT_LEN]> {
        self.slots.get(index)
    }

    /// The record the file select copies into the live sessions (`$3020A`).
    ///
    /// The ROM validates at entry (`$28D9`) and never again on load: an empty
    /// slot has already become the fresh template, which is exactly what a new
    /// game loads. Call [`Self::validate_and_init`] first, as `$19` does.
    pub fn load_slot(&self, index: usize) -> Option<[u8; SLOT_LEN]> {
        self.slot(index).copied()
    }

    /// `$2934`: write one slot from a produced record.
    ///
    /// The record is stored as given; its checksum is the producer's job (and
    /// a bad one will simply fail the next `$28D9`, as on the console).
    pub fn write_slot(&mut self, index: usize, record: &[u8]) -> bool {
        if record.len() < SLOT_LEN {
            return false;
        }
        let Some(slot) = self.slots.get_mut(index) else {
            return false;
        };
        slot.copy_from_slice(&record[..SLOT_LEN]);
        true
    }

    /// The erase path: replace one slot with the fresh-file template.
    pub fn erase_slot(&mut self, index: usize) -> bool {
        let Some(slot) = self.slots.get_mut(index) else {
            return false;
        };
        *slot = fresh_template();
        true
    }
}

#[cfg(test)]
mod tests;
