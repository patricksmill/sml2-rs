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
