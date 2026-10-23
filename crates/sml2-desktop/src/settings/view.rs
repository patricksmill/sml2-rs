use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Widescreen {
    #[default]
    Off,
    Wide16x9,
}

impl Widescreen {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Wide16x9 => "16:9",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Wide16x9,
            Self::Wide16x9 => Self::Off,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CameraZoom {
    #[default]
    Native,
    Zoom125,
    Zoom150,
}

impl CameraZoom {
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "100%",
            Self::Zoom125 => "125%",
            Self::Zoom150 => "150%",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Native => Self::Zoom125,
            Self::Zoom125 => Self::Zoom150,
            Self::Zoom150 => Self::Native,
        }
    }
}

/// The game view in game pixels.
pub fn view_size(widescreen: Widescreen, zoom: CameraZoom) -> (usize, usize) {
    let height = match zoom {
        CameraZoom::Native => 144,
        CameraZoom::Zoom125 => 180,
        CameraZoom::Zoom150 => 216,
    };
    let width = match widescreen {
        Widescreen::Off => height * 10 / 9,
        Widescreen::Wide16x9 => height * 16 / 9,
    };
    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn view_sizes_match_the_design_table() {
        use CameraZoom::*;
        use Widescreen::*;
        assert_eq!(view_size(Off, Native), (160, 144));
        assert_eq!(view_size(Off, Zoom125), (200, 180));
        assert_eq!(view_size(Off, Zoom150), (240, 216));
        assert_eq!(view_size(Wide16x9, Native), (256, 144));
        assert_eq!(view_size(Wide16x9, Zoom125), (320, 180));
        assert_eq!(view_size(Wide16x9, Zoom150), (384, 216));
    }
}
