//! Native character identity, independent of cartridge flags and asset storage.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Character {
    #[default]
    Mario,
    Luigi,
}

impl Character {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mario => "Mario",
            Self::Luigi => "Luigi",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Mario => Self::Luigi,
            Self::Luigi => Self::Mario,
        }
    }
}

/// Bank-1 displacement data imported from the supported DX ROM, without code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovementCurves {
    pub horizontal: [u8; 256],
    pub normal: [u8; 76],
    pub swim: [u8; 76],
    pub moon: [u8; 76],
}
