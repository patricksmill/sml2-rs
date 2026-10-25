use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Widescreen {
    #[default]
    Off,
    Wide16x9,
    Wide21x9,
    WindowSize,
}

impl Widescreen {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Wide16x9 => "16:9",
            Self::Wide21x9 => "21:9",
            Self::WindowSize => "Window size",
        }
    }
    pub fn next(self) -> Self {
        self.cycle(1)
    }

    pub fn cycle(self, direction: i32) -> Self {
        const ALL: [Widescreen; 4] = [
            Widescreen::Off,
            Widescreen::Wide16x9,
            Widescreen::Wide21x9,
            Widescreen::WindowSize,
        ];
        let index = ALL.iter().position(|wide| *wide == self).unwrap_or(0) as i32;
        ALL[(index + direction).rem_euclid(ALL.len() as i32) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CameraZoom {
    #[default]
    Native,
    Zoom125,
    Zoom150,
    Zoom200,
}

impl CameraZoom {
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "100%",
            Self::Zoom125 => "125%",
            Self::Zoom150 => "150%",
            Self::Zoom200 => "200%",
        }
    }
    pub fn next(self) -> Self {
        self.cycle(1)
    }

    pub fn cycle(self, direction: i32) -> Self {
        const ALL: [CameraZoom; 4] = [
            CameraZoom::Native,
            CameraZoom::Zoom125,
            CameraZoom::Zoom150,
            CameraZoom::Zoom200,
        ];
        let index = ALL.iter().position(|zoom| *zoom == self).unwrap_or(0) as i32;
        ALL[(index + direction).rem_euclid(ALL.len() as i32) as usize]
    }
}

/// The game view in game pixels.
pub fn view_size(widescreen: Widescreen, zoom: CameraZoom, window: [u32; 2]) -> (usize, usize) {
    let height = match zoom {
        CameraZoom::Native => 144,
        CameraZoom::Zoom125 => 180,
        CameraZoom::Zoom150 => 216,
        CameraZoom::Zoom200 => 288,
    };
    let width = match widescreen {
        Widescreen::Off => height * 10 / 9,
        Widescreen::Wide16x9 => height * 16 / 9,
        Widescreen::Wide21x9 => height * 21 / 9,
        Widescreen::WindowSize if !window.contains(&0) => {
            let (w, h) = (window[0] as usize, window[1] as usize);
            let native_width = height * 10 / 9;
            if w * height >= h * native_width {
                return (height * w / h, height);
            }
            // Tall windows reveal more vertically instead of cropping the
            // native playfield. Camera zoom still sets its minimum extent.
            return (native_width, native_width * h / w);
        }
        Widescreen::WindowSize => height * 10 / 9,
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
        let view_size = |wide, zoom| super::view_size(wide, zoom, [800, 720]);
        assert_eq!(view_size(Off, Native), (160, 144));
        assert_eq!(view_size(Off, Zoom125), (200, 180));
        assert_eq!(view_size(Off, Zoom150), (240, 216));
        assert_eq!(view_size(Wide16x9, Native), (256, 144));
        assert_eq!(view_size(Wide16x9, Zoom125), (320, 180));
        assert_eq!(view_size(Wide16x9, Zoom150), (384, 216));
        assert_eq!(view_size(Off, Zoom200), (320, 288));
        assert_eq!(view_size(Wide16x9, Zoom200), (512, 288));
        assert_eq!(view_size(Wide21x9, Native), (336, 144));
        assert_eq!(view_size(Wide21x9, Zoom125), (420, 180));
        assert_eq!(view_size(Wide21x9, Zoom150), (504, 216));
        assert_eq!(view_size(Wide21x9, Zoom200), (672, 288));
    }

    #[test]
    fn window_size_follows_the_aspect_and_preserves_the_native_playfield() {
        use CameraZoom::*;
        use Widescreen::WindowSize;
        for (window, zoom, expected) in [
            ([1920, 1080], Native, (256, 144)),
            ([2520, 1080], Zoom150, (504, 216)),
            ([1280, 1024], Zoom200, (360, 288)),
            ([480, 960], Native, (160, 320)),
            ([480, 960], Zoom200, (320, 640)),
            ([0, 720], Native, (160, 144)),
            ([800, 0], Zoom150, (240, 216)),
        ] {
            assert_eq!(view_size(WindowSize, zoom, window), expected);
        }
    }

    #[test]
    fn widescreen_cycles_all_choices_in_both_directions() {
        use Widescreen::*;
        for (current, next) in [
            (Off, Wide16x9),
            (Wide16x9, Wide21x9),
            (Wide21x9, WindowSize),
            (WindowSize, Off),
        ] {
            assert_eq!(current.next(), next);
            assert_eq!(next.cycle(-1), current);
        }
    }

    #[test]
    fn zoom_cycles_through_200_percent_back_to_native() {
        use CameraZoom::*;
        assert_eq!(Zoom150.next(), Zoom200);
        assert_eq!(Zoom200.next(), Native);
        assert_eq!(Zoom200.label(), "200%");
    }
}
