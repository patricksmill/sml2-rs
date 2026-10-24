//! Bank-24 castle and bonus-room scripts (`extracted/map/castle_scripts.bin`).
//!
//! `$3E00` only switches banks and jumps, so `$4028`/`$4512`/`$4A81` are
//! ordinary code entries. The extractor emits the contiguous regions plus the
//! three bell slot tables and the two bank-15 sprite lists; this type gives the
//! port a name-keyed view of those bytes for the room handlers.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ScriptBlobMeta {
    pub name: String,
    pub bank: u8,
    /// `$NNNN`, in the bank window.
    pub addr: String,
    pub offset: usize,
    pub len: usize,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct Manifest {
    blobs: Vec<ScriptBlobMeta>,
    total_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CastleScripts {
    bytes: Vec<u8>,
    blobs: Vec<ScriptBlobMeta>,
}

impl CastleScripts {
    pub fn from_parts(bytes: Vec<u8>, manifest_json: &str) -> Result<Self, String> {
        let manifest: Manifest =
            serde_json::from_str(manifest_json).map_err(|e| format!("castle scripts: {e}"))?;
        if manifest.total_len != bytes.len() {
            return Err(format!(
                "castle scripts: manifest says {} bytes, blob is {}",
                manifest.total_len,
                bytes.len()
            ));
        }
        for blob in &manifest.blobs {
            if blob.offset + blob.len > bytes.len() {
                return Err(format!(
                    "castle scripts: {} runs past the blob (${:05X}+{})",
                    blob.name, blob.offset, blob.len
                ));
            }
        }
        Ok(Self {
            bytes,
            blobs: manifest.blobs,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn blob(&self, name: &str) -> Option<&[u8]> {
        let meta = self.blobs.iter().find(|b| b.name == name)?;
        self.bytes.get(meta.offset..meta.offset + meta.len)
    }

    /// Replace a presentation blob while retaining its script metadata.
    pub fn replace_blob(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let meta = self
            .meta(name)
            .ok_or_else(|| format!("missing castle blob {name}"))?
            .clone();
        if bytes.len() != meta.len {
            return Err(format!("castle blob {name} has wrong length"));
        }
        self.bytes[meta.offset..meta.offset + meta.len].copy_from_slice(bytes);
        Ok(())
    }

    pub fn meta(&self, name: &str) -> Option<&ScriptBlobMeta> {
        self.blobs.iter().find(|b| b.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extracted() -> CastleScripts {
        CastleScripts::from_parts(
            include_bytes!("../../../extracted/map/castle_scripts.bin").to_vec(),
            include_str!("../../../extracted/map/castle_scripts.json"),
        )
        .expect("castle scripts asset")
    }

    #[test]
    fn extracted_blobs_pin_bank24_entries() {
        let scripts = extracted();
        let cases: &[(&str, u8, &str, usize, &[u8])] = &[
            // `$4028`: state table at +$0F, keyed by sCastleState.
            (
                "bell_room_b_script",
                24,
                "$4028",
                0x4EA,
                &[0xFA, 0xA6, 0xA7, 0x87],
            ),
            (
                "bell_room_a_script",
                24,
                "$4512",
                0x508,
                &[0xFA, 0xA6, 0xA7, 0x87],
            ),
            (
                "coin_room_script",
                24,
                "$4A81",
                0x4D9,
                &[0xFA, 0xA6, 0xA7, 0xFE],
            ),
        ];
        for (name, bank, addr, len, head) in cases {
            let meta = scripts.meta(name).expect(name);
            assert_eq!(
                (meta.bank, meta.addr.as_str(), meta.len),
                (*bank, *addr, *len)
            );
            assert_eq!(&scripts.blob(name).unwrap()[..head.len()], *head, "{name}");
        }
    }

    #[test]
    fn bell_slot_tables_run_to_their_marker() {
        let scripts = extracted();
        assert_eq!(scripts.blob("bell_slots_0").unwrap().len(), 0x15);
        assert_eq!(scripts.blob("bell_slots_1").unwrap().len(), 0x29);
        assert_eq!(scripts.blob("bell_slots_2").unwrap().len(), 0x29);
        for name in ["bell_slots_0", "bell_slots_1", "bell_slots_2"] {
            let table = scripts.blob(name).unwrap();
            assert_eq!(*table.last().unwrap(), 0xFF, "{name} terminator");
            assert!(
                table[..table.len() - 1].iter().all(|&b| b < 6),
                "{name} values"
            );
        }
    }

    #[test]
    fn sprite_lists_carry_two_terminated_record_runs() {
        let scripts = extracted();
        let a = scripts.blob("bell_room_a_sprites").unwrap();
        let b = scripts.blob("bell_room_b_sprites").unwrap();
        assert_eq!(a.len(), 0x2D); // 13 sprites + terminator + empty patches
        assert_eq!(b.len(), 0x5D); // 16 sprites + 13 patches + two terminators
        assert_eq!(&a[..3], &[0x20, 0xA0, 0x00]);
        assert_eq!(&b[..3], &[0x20, 0xA0, 0x00]);
    }
}
