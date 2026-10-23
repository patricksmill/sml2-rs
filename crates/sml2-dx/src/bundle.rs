//! The persisted import: the DX ROM's bank-0 level loader tables and the raw
//! banks every level's presentation reads. See docs/sml2dx-audit.md.

use crate::{presentation::Presentation, ImportError, Rom, BUNDLE_SCHEMA, ROM_SHA256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "bundle_tests.rs"]
mod bundle_tests;

/// Banks the presentation reads. `$06`, `$07` and `$1B`-`$1F` hold the base
/// game's level, arena, transfer and animation graphics, which DX patches in
/// place; the rest are DX additions: `$20` common HUD tiles, `$21` tile
/// attributes, `$23`/`$28`/`$3A` object mappings, `$24` VRAM1 graphics and
/// HUD tables, `$26` palettes, `$2A`/`$2B` Mario/Luigi graphics and
/// `$2C`/`$2D` their mappings.
pub const BANKS: [u8; 18] = [
    0x06, 0x07, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x23, 0x24, 0x26, 0x28, 0x2A, 0x2B, 0x2C,
    0x2D, 0x3A,
];
pub const BANK_LEN: usize = 0x4000;
pub const LEVELS: usize = 32;

/// Bank-0 tables `LoadMarioGFX` ($0669) reads, identical to the base ROM's
/// except `$360A`, whose values DX changed.
const LEVEL_CONFIG: usize = 0x360A;
const ENEMY_SETS: usize = 0x362A;
const TILE_SETS: usize = 0x36AA;
const EXTRA_POINTERS: usize = 0x0763;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub schema: u32,
    pub source_sha256: String,
    /// `$360A`: per-level `sLevelGfxConfig`.
    pub level_config: Vec<u8>,
    /// `$362A`/`$36AA`: per-level `[bank, 0, lo, hi]` enemy and tileset sources.
    pub enemy_sets: Vec<u8>,
    pub tile_sets: Vec<u8>,
    /// `$0763`: four bank-7 pointers for the `$8E80` block.
    pub extra_pointers: Vec<u8>,
    #[serde(with = "hex_banks")]
    pub banks: BTreeMap<u8, Vec<u8>>,
}

impl Bundle {
    /// Shape checks, then every derived table: a bundle that loads can draw
    /// every level.
    pub fn validate(&self) -> Result<(), ImportError> {
        self.check_shape()?;
        Presentation::new(self).map(|_| ())
    }

    pub(crate) fn check_shape(&self) -> Result<(), ImportError> {
        let bad = |s: String| Err(ImportError::InvalidBundle(s));
        if self.schema != BUNDLE_SCHEMA || self.source_sha256 != ROM_SHA256 {
            return bad(
                "this DX installation came from an older importer or another ROM; re-import it"
                    .into(),
            );
        }
        for (label, data, length) in [
            ("level config", &self.level_config, LEVELS),
            ("enemy sets", &self.enemy_sets, LEVELS * 4),
            ("tile sets", &self.tile_sets, LEVELS * 4),
            ("extra pointers", &self.extra_pointers, 8),
        ] {
            if data.len() != length {
                return bad(format!("{label} needs {length} bytes, got {}", data.len()));
            }
        }
        if !self.banks.keys().copied().eq(BANKS) {
            return bad("bundle must carry exactly the presentation banks".into());
        }
        if let Some((bank, _)) = self.banks.iter().find(|(_, b)| b.len() != BANK_LEN) {
            return bad(format!("bank ${bank:02X} needs {BANK_LEN} bytes"));
        }
        Ok(())
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

    /// Bounded read inside one stored bank.
    pub fn banked(&self, bank: u8, address: u16, length: usize) -> Result<&[u8], ImportError> {
        let invalid = ImportError::InvalidPointer { bank, address };
        if !(0x4000..0x8000).contains(&address) || length > 0x8000 - usize::from(address) {
            return Err(invalid);
        }
        let start = usize::from(address) - 0x4000;
        self.banks
            .get(&bank)
            .and_then(|b| b.get(start..start + length))
            .ok_or(invalid)
    }
}

pub fn extract(bytes: &[u8]) -> Result<Bundle, ImportError> {
    let rom = Rom::new(bytes)?;
    let bundle = Bundle {
        schema: BUNDLE_SCHEMA,
        source_sha256: ROM_SHA256.into(),
        level_config: rom.read(LEVEL_CONFIG, LEVELS)?.to_vec(),
        enemy_sets: rom.read(ENEMY_SETS, LEVELS * 4)?.to_vec(),
        tile_sets: rom.read(TILE_SETS, LEVELS * 4)?.to_vec(),
        extra_pointers: rom.read(EXTRA_POINTERS, 8)?.to_vec(),
        banks: BANKS
            .iter()
            .map(|&bank| Ok((bank, rom.banked(bank, 0x4000, BANK_LEN)?.to_vec())))
            .collect::<Result<_, ImportError>>()?,
    };
    bundle.validate()?;
    Ok(bundle)
}

/// Banks as `{"1B": "hex…"}`: half the size of a JSON number array.
mod hex_banks {
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S: Serializer>(
        banks: &BTreeMap<u8, Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        banks
            .iter()
            .map(|(bank, bytes)| {
                let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                (format!("{bank:02X}"), hex)
            })
            .collect::<BTreeMap<_, _>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<u8, Vec<u8>>, D::Error> {
        let raw = BTreeMap::<String, String>::deserialize(deserializer)?;
        raw.into_iter()
            .map(|(bank, hex)| {
                if bank.len() != 2 || hex.len() % 2 != 0 {
                    return Err(D::Error::custom("malformed bank entry"));
                }
                let bank = u8::from_str_radix(&bank, 16).map_err(D::Error::custom)?;
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|i| {
                        hex.get(i..i + 2)
                            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                            .ok_or_else(|| D::Error::custom("bank bytes must be hex"))
                    })
                    .collect::<Result<_, _>>()?;
                Ok((bank, bytes))
            })
            .collect()
    }
}
