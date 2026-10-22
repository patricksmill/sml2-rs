//! Audited opening-level presentation data; see docs/sml2dx-audit.md.

use crate::{ImportError, Rom, BUNDLE_SCHEMA, ROM_SHA256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sml2_core::{character::Character, object::MappingEntry, video::cgb::CgbPalettes};
use std::{fs, io::Read, path::Path};

#[cfg(test)]
#[path = "bundle_tests.rs"]
mod bundle_tests;

/// Character-specific opening-theme graphics, separate from native physics.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterGraphics {
    pub normal: Vec<u8>,
    pub fire: Vec<u8>,
    pub bank1: Vec<u8>,
    pub mappings: Vec<Vec<MappingEntry>>,
}

impl CharacterGraphics {
    fn validate(&self) -> Result<(), ImportError> {
        for (label, bytes) in [
            ("normal", &self.normal),
            ("fire", &self.fire),
            ("bank 1", &self.bank1),
        ] {
            if bytes.len() != 0x800 {
                return Err(ImportError::InvalidBundle(format!(
                    "Luigi {label} needs 2048 bytes, got {}",
                    bytes.len()
                )));
            }
        }
        if self.mappings.len() != 242 || self.mappings.iter().any(|m| m.len() > 64) {
            return Err(ImportError::InvalidBundle(
                "invalid Luigi sprite mapping count or length".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub schema: u32,
    pub source_sha256: String,
    /// Both banks span VRAM $8800..$97FF.
    pub background: [Vec<u8>; 2],
    /// Mario's dynamically loaded $8000..$87FF, normal and fire forms.
    pub mario_normal: Vec<u8>,
    pub mario_fire: Vec<u8>,
    pub mario_bank1: Vec<u8>,
    pub tile_attributes: Vec<u8>,
    pub hud_tiles: Vec<u8>,
    pub hud_attributes: Vec<u8>,
    pub bg_palettes: Vec<u8>,
    pub obj_palettes: Vec<u8>,
    /// Four phases, each 64 bytes, loaded at VRAM0 $9200.
    pub animation: Vec<u8>,
    pub object_mappings: Vec<Vec<MappingEntry>>,
    pub player_mappings: Vec<Vec<MappingEntry>>,
    /// Missing in the original Mario-only schema. Re-import to add Luigi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub luigi: Option<CharacterGraphics>,
}

impl Bundle {
    pub fn validate(&self) -> Result<(), ImportError> {
        let bad = |s: &str| ImportError::InvalidBundle(s.into());
        if !matches!(self.schema, 1 | BUNDLE_SCHEMA) || self.source_sha256 != ROM_SHA256 {
            return Err(bad("unsupported schema or source revision"));
        }
        match (self.schema, &self.luigi) {
            (1, None) => {}
            (BUNDLE_SCHEMA, Some(graphics)) => graphics.validate()?,
            _ => return Err(bad("character graphics do not match bundle schema")),
        }
        for (label, data, length) in [
            ("background bank 0", &self.background[0], 0x1000),
            ("background bank 1", &self.background[1], 0x1000),
            ("Mario normal", &self.mario_normal, 0x800),
            ("Mario fire", &self.mario_fire, 0x800),
            ("Mario bank 1", &self.mario_bank1, 0x800),
            ("tile attributes", &self.tile_attributes, 256),
            ("HUD tiles", &self.hud_tiles, 20),
            ("HUD attributes", &self.hud_attributes, 20),
            ("BG palettes", &self.bg_palettes, 64),
            ("OBJ palettes", &self.obj_palettes, 64),
            ("animation", &self.animation, 256),
        ] {
            if data.len() != length {
                return Err(bad(&format!(
                    "{label} needs {length} bytes, got {}",
                    data.len()
                )));
            }
        }
        if self.object_mappings.len() != 168
            || self.player_mappings.len() != 242
            || self
                .object_mappings
                .iter()
                .chain(&self.player_mappings)
                .any(|m| m.len() > 64)
        {
            return Err(bad("invalid sprite mapping count or length"));
        }
        CgbPalettes::from_bytes(&self.bg_palettes, &self.obj_palettes)
            .map_err(|e| bad(&e.to_string()))?;
        Ok(())
    }

    /// Legacy bundles retain Mario support; Luigi requires an updated import.
    pub fn supports_character(&self, character: Character) -> bool {
        match character {
            Character::Mario => true,
            Character::Luigi => self.luigi.is_some(),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ImportError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|e| ImportError::InvalidBundle(e.to_string()))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ImportError> {
        if bytes.len() > crate::cache::MAX_PAYLOAD_BYTES {
            return Err(ImportError::InvalidBundle(
                "bundle exceeds size limit".into(),
            ));
        }
        let bundle: Self =
            serde_json::from_slice(bytes).map_err(|e| ImportError::InvalidBundle(e.to_string()))?;
        bundle.validate()?;
        Ok(bundle)
    }
}

fn mappings(
    rom: &Rom<'_>,
    bank: u8,
    table: usize,
    count: usize,
) -> Result<Vec<Vec<MappingEntry>>, ImportError> {
    (0..count)
        .map(|i| {
            let pointer = rom.word(table + i * 2)?;
            let mut result = Vec::new();
            for n in 0..=64 {
                let address = pointer
                    .checked_add(n * 4)
                    .ok_or(ImportError::InvalidPointer {
                        bank,
                        address: pointer,
                    })?;
                if rom.banked(bank, address, 1)?[0] == 0x80 {
                    return Ok(result);
                }
                if n == 64 {
                    break;
                }
                let c = rom.banked(bank, address, 4)?;
                result.push(MappingEntry {
                    y: c[0] as i8,
                    x: c[1] as i8,
                    tile: c[2],
                    attr: c[3],
                });
            }
            Err(ImportError::InvalidBundle(format!(
                "unterminated sprite mapping {i}"
            )))
        })
        .collect()
}

pub fn extract(bytes: &[u8]) -> Result<Bundle, ImportError> {
    let rom = Rom::new(bytes)?;
    let mut bank0 = vec![0; 0x1000];
    for (source, destination, length) in [
        (0x6f600, 0, 0x300),
        (0x19800, 0x300, 0x380),
        (0x1f100, 0x680, 0x380),
        (0x1c000, 0xa00, 0x600),
    ] {
        bank0[destination..destination + length].copy_from_slice(rom.read(source, length)?);
    }
    let mut bank1 = rom.read(0x81d00, 0x300)?.to_vec();
    bank1.extend_from_slice(rom.read(0x90000, 0xd00)?);
    let bundle = Bundle {
        schema: BUNDLE_SCHEMA,
        source_sha256: ROM_SHA256.into(),
        background: [bank0, bank1],
        mario_normal: rom.read(0xa8200, 0x800)?.to_vec(),
        mario_fire: rom.read(0xa9200, 0x800)?.to_vec(),
        mario_bank1: rom.read(0xa8a00, 0x800)?.to_vec(),
        luigi: Some(CharacterGraphics {
            normal: rom.read(0xac200, 0x800)?.to_vec(),
            fire: rom.read(0xad200, 0x800)?.to_vec(),
            bank1: rom.read(0xaca00, 0x800)?.to_vec(),
            mappings: mappings(&rom, 0x2d, 0xb4000, 242)?,
        }),
        tile_attributes: rom.read(0x84000, 256)?.to_vec(),
        hud_tiles: rom.read(0x93c24, 20)?.to_vec(),
        hud_attributes: rom.read(0x92040, 20)?.to_vec(),
        bg_palettes: rom.read(0x98000, 64)?.to_vec(),
        obj_palettes: rom.read(0x99000, 64)?.to_vec(),
        animation: rom.read(0x77380, 256)?.to_vec(),
        object_mappings: mappings(&rom, 0x23, 0x8c0b1, 168)?,
        player_mappings: mappings(&rom, 0x2c, 0xb0000, 242)?,
    };
    bundle.validate()?;
    Ok(bundle)
}

/// Hash the actual base bundle, including relative paths and file lengths.
/// Symlinks are rejected so cache identity cannot silently follow external data.
pub fn base_fingerprint(root: &Path) -> Result<String, ImportError> {
    fingerprint_with_limits(root, 32, 100_000, 512 * 1024 * 1024)
}

fn fingerprint_with_limits(
    root: &Path,
    max_depth: usize,
    max_files: usize,
    max_bytes: u64,
) -> Result<String, ImportError> {
    let bad = |s: &str| ImportError::InvalidBundle(s.into());
    fn collect(
        root: &Path,
        dir: &Path,
        files: &mut Vec<std::path::PathBuf>,
        depth: usize,
        max_depth: usize,
        max_files: usize,
    ) -> Result<(), ImportError> {
        if depth > max_depth {
            return Err(ImportError::InvalidBundle(
                "base asset directories exceed depth limit".into(),
            ));
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(ImportError::InvalidBundle(
                    "base assets contain a symlink".into(),
                ));
            }
            if kind.is_dir() {
                collect(root, &entry.path(), files, depth + 1, max_depth, max_files)?;
            } else if kind.is_file() {
                if files.len() >= max_files {
                    return Err(ImportError::InvalidBundle(
                        "base assets exceed file count limit".into(),
                    ));
                }
                files.push(entry.path().strip_prefix(root).unwrap().to_owned());
            } else {
                return Err(ImportError::InvalidBundle(
                    "base assets contain a nonregular entry".into(),
                ));
            }
        }
        Ok(())
    }
    if !fs::symlink_metadata(root)?.is_dir() {
        return Err(bad("base asset root must be a directory"));
    }
    if !root.join("levels/level01.manifest.json").is_file() {
        return Err(ImportError::InvalidBundle("existing extracted base assets are required; choose the extracted directory with --assets".into()));
    }
    let mut files = Vec::new();
    collect(root, root, &mut files, 0, max_depth, max_files)?;
    files.sort();
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0; 64 * 1024];
    for path in files {
        // Join components so literal backslashes in Unix filenames retain their
        // identity rather than colliding with a directory separator.
        let name = path
            .components()
            .map(|c| {
                c.as_os_str()
                    .to_str()
                    .ok_or_else(|| bad("non-UTF8 asset path"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        let absolute = root.join(&path);
        let metadata = fs::symlink_metadata(&absolute)?;
        if !metadata.is_file() {
            return Err(bad("base asset file changed type"));
        }
        let length = metadata.len();
        total = total
            .checked_add(length)
            .filter(|&n| n <= max_bytes)
            .ok_or_else(|| bad("base assets exceed byte limit"))?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update(length.to_le_bytes());
        let mut file = fs::File::open(&absolute)?;
        let mut read = 0u64;
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            read += count as u64;
            if read > length {
                return Err(bad("base asset changed during fingerprinting"));
            }
            hash.update(&buffer[..count]);
        }
        if read != length {
            return Err(bad("base asset changed during fingerprinting"));
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mappings_cannot_read_across_bank_or_loop_without_terminator() {
        let mut bytes = vec![0; crate::ROM_LEN];
        bytes[..2].copy_from_slice(&0x7fffu16.to_le_bytes());
        assert!(mappings(&Rom(&bytes), 1, 0, 1).is_err());
        bytes[..2].copy_from_slice(&0x4000u16.to_le_bytes());
        assert!(mappings(&Rom(&bytes), 1, 0, 1).is_err());
        bytes[0x4000..0x4005].copy_from_slice(&[0xf8, 4, 12, 0x2b, 0x80]);
        let m = mappings(&Rom(&bytes), 1, 0, 1).unwrap();
        assert_eq!(
            m[0],
            vec![MappingEntry {
                y: -8,
                x: 4,
                tile: 12,
                attr: 0x2b
            }]
        );
    }

    #[test]
    fn base_identity_includes_names_and_contents() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("levels")).unwrap();
        let file = dir.path().join("levels/level01.manifest.json");
        fs::write(&file, b"{}").unwrap();
        let original = base_fingerprint(dir.path()).unwrap();
        fs::write(&file, b"[]").unwrap();
        assert_ne!(base_fingerprint(dir.path()).unwrap(), original);
        fs::write(&file, b"{}").unwrap();
        fs::write(dir.path().join("extra"), b"").unwrap();
        assert_ne!(base_fingerprint(dir.path()).unwrap(), original);
    }
}
