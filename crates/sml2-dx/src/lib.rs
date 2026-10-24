//! Local import of one audited SML2DX revision. Native gameplay never executes
//! patched cartridge code. Imported graphics belong in the user's local cache.

use sha2::{Digest, Sha256};
use std::{error::Error, fmt, ops::Range};

mod bundle;
pub mod cache;
mod fingerprint;
mod ips;
mod presentation;
pub use bundle::{extract, Bundle, BANKS};
pub use fingerprint::base_fingerprint;
pub use ips::apply_ips;
pub use presentation::{
    BossArena, BossFight, LevelGraphics, MapGraphics, MapScreen, MapSprites, PlayerGraphics,
    Presentation, Screen, VramCopy,
};

pub const REVISION: &str = "SML2DX v1.8.1 (UE v1.0)";
pub const ROM_LEN: usize = 0x100000;
/// Locally retained original ROM, used only as the input to an optional IPS patch.
pub const SOURCE_ROM: &str = "source.gb";
/// User-supplied ROM built from the official IPS patch against UE v1.0.
pub const ROM_SHA256: &str = "1dd604a91b9ee99ef381cc795da3ae28c72d04ba9de78b355df829e56ce49c46";
pub const IMPORTER_VERSION: u32 = 8;
pub const BUNDLE_SCHEMA: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    UnsupportedRevision { size: usize, sha256: String },
    InvalidRange { offset: usize, length: usize },
    InvalidPointer { bank: u8, address: u16 },
    InvalidBundle(String),
    InvalidPatch(String),
    Cancelled,
    Io(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRevision { size, sha256 } => write!(
                f,
                "Unsupported DX ROM ({size} bytes, SHA-256 {sha256}). Expected {REVISION}."
            ),
            Self::InvalidRange { offset, length } => write!(
                f,
                "DX data range ${offset:X} + ${length:X} is outside the ROM"
            ),
            Self::InvalidPointer { bank, address } => {
                write!(f, "Invalid DX ROM pointer {bank:02X}:${address:04X}")
            }
            Self::InvalidBundle(message) => write!(f, "Invalid DX presentation bundle: {message}"),
            Self::InvalidPatch(message) => write!(f, "Invalid DX IPS patch: {message}"),
            Self::Cancelled => f.write_str("DX import cancelled"),
            Self::Io(message) => write!(f, "DX import: {message}"),
        }
    }
}

impl Error for ImportError {}

impl From<std::io::Error> for ImportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Revision recognition is deliberately independent of cartridge header hints.
pub fn identify(bytes: &[u8]) -> Result<&'static str, ImportError> {
    let digest = sha256(bytes);
    if bytes.len() != ROM_LEN || digest != ROM_SHA256 {
        return Err(ImportError::UnsupportedRevision {
            size: bytes.len(),
            sha256: digest,
        });
    }
    Ok(REVISION)
}

/// Bounds-checked ROM access shared by the audited extraction tables.
pub struct Rom<'a>(&'a [u8]);

impl<'a> Rom<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, ImportError> {
        identify(bytes)?;
        Ok(Self(bytes))
    }

    pub fn read(&self, offset: usize, length: usize) -> Result<&'a [u8], ImportError> {
        let end = offset
            .checked_add(length)
            .ok_or(ImportError::InvalidRange { offset, length })?;
        self.0
            .get(offset..end)
            .ok_or(ImportError::InvalidRange { offset, length })
    }

    /// Resolve a switchable ROM-bank pointer; a read cannot spill into another bank.
    pub fn banked(&self, bank: u8, address: u16, length: usize) -> Result<&'a [u8], ImportError> {
        if bank == 0
            || bank >= 64
            || !(0x4000..0x8000).contains(&address)
            || length > 0x8000 - usize::from(address)
        {
            return Err(ImportError::InvalidPointer { bank, address });
        }
        self.read(
            usize::from(bank) * 0x4000 + usize::from(address) - 0x4000,
            length,
        )
    }

    pub fn word(&self, offset: usize) -> Result<u16, ImportError> {
        let b = self.read(offset, 2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn range(&self, range: Range<usize>) -> Result<&'a [u8], ImportError> {
        let length = range
            .end
            .checked_sub(range.start)
            .ok_or(ImportError::InvalidRange {
                offset: range.start,
                length: 0,
            })?;
        self.read(range.start, length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_by_digest_rather_than_header_or_length() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        for size in [0, 0x14f, ROM_LEN - 1, ROM_LEN, ROM_LEN + 1] {
            let mut bytes = vec![0; size];
            if size >= 0x150 {
                bytes[0x143] = 0xC0;
                bytes[0x148] = 5;
            }
            assert!(
                matches!(identify(&bytes), Err(ImportError::UnsupportedRevision { size: n, .. }) if n == size)
            );
        }
    }

    #[test]
    fn rom_reads_reject_overflow_and_cross_bank_pointers() {
        let bytes = [0x12, 0x34, 0x56];
        let rom = Rom(&bytes);
        assert_eq!(rom.word(0).unwrap(), 0x3412);
        assert!(rom.word(2).is_err());
        assert!(rom.read(usize::MAX, 2).is_err());
        assert!(rom.range(2..1).is_err());
        let bytes = vec![0; ROM_LEN];
        let rom = Rom(&bytes);
        assert_eq!(rom.banked(63, 0x7fff, 1).unwrap(), &[0]);
        for (bank, address, length) in [
            (0, 0x4000, 1),
            (64, 0x4000, 1),
            (1, 0x3fff, 1),
            (1, 0x8000, 1),
            (1, 0x7fff, 2),
        ] {
            assert!(matches!(
                rom.banked(bank, address, length),
                Err(ImportError::InvalidPointer { .. })
            ));
        }
    }
}
