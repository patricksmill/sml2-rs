use serde::{Deserialize, Serialize};
use sml2_core::{
    hardware_palettes::{gbc_auto, rgb555, sgb_preset, HardwarePalettes},
    video::{PalettePixel, PaletteSource},
};

/// Presentation palettes consume the mapped hardware shade and its BG/OBJ source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Filter {
    #[default]
    Gray,
    Dmg,
    Pocket,
    Gbc,
    /// Zero-based SGB system preset: 1-A through 4-H.
    Sgb(u8),
    Amber,
}

pub struct DisplayPalette {
    bg: [[u8; 3]; 4],
    obj0: [[u8; 3]; 4],
    obj1: [[u8; 3]; 4],
}

impl DisplayPalette {
    pub fn rgb(&self, pixel: PalettePixel) -> [u8; 3] {
        let palette = match pixel.source {
            PaletteSource::Bg => &self.bg,
            PaletteSource::Obj0 => &self.obj0,
            PaletteSource::Obj1 => &self.obj1,
        };
        palette[usize::from(pixel.shade)]
    }
}

impl From<HardwarePalettes> for DisplayPalette {
    fn from(palettes: HardwarePalettes) -> Self {
        Self {
            bg: palettes.bg.map(rgb555),
            obj0: palettes.obj0.map(rgb555),
            obj1: palettes.obj1.map(rgb555),
        }
    }
}

impl Filter {
    pub const ALL: [Filter; 37] = {
        let mut filters = [Self::Gray; 37];
        filters[1] = Self::Dmg;
        filters[2] = Self::Pocket;
        filters[3] = Self::Gbc;
        let mut index = 0;
        while index < 32 {
            filters[index + 4] = Self::Sgb(index as u8);
            index += 1;
        }
        filters[36] = Self::Amber;
        filters
    };

    pub fn label(self) -> String {
        match self {
            Self::Gray => "Grayscale".into(),
            Self::Dmg => "DMG green".into(),
            Self::Pocket => "Pocket".into(),
            Self::Gbc => "GBC auto".into(),
            Self::Sgb(index) => format!("SGB {}-{}", index / 8 + 1, char::from(b'A' + index % 8)),
            Self::Amber => "Amber".into(),
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        let index = Self::ALL.iter().position(|f| *f == self).unwrap_or(0) as i64;
        Self::ALL[(index + i64::from(delta)).rem_euclid(Self::ALL.len() as i64) as usize]
    }

    pub fn palette(self, header: &[u8; 0x18]) -> DisplayPalette {
        let ramp = match self {
            Self::Gbc => return gbc_auto(header).into(),
            Self::Sgb(index) => {
                return sgb_preset(usize::from(index))
                    .expect("validated SGB preset")
                    .into()
            }
            Self::Gray => [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]],
            Self::Dmg => [[155, 188, 15], [139, 172, 15], [48, 98, 48], [15, 56, 15]],
            Self::Pocket => [[255, 255, 255], [192, 192, 192], [96, 96, 96], [0, 0, 0]],
            Self::Amber => [[255, 235, 180], [230, 170, 80], [140, 80, 20], [30, 15, 0]],
        };
        DisplayPalette {
            bg: ramp,
            obj0: ramp,
            obj1: ramp,
        }
    }
}

impl std::str::FromStr for Filter {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "gray" => Ok(Self::Gray),
            "dmg" => Ok(Self::Dmg),
            "pocket" => Ok(Self::Pocket),
            "gbc" => Ok(Self::Gbc),
            "sgb" => Ok(Self::Sgb(0)),
            "amber" => Ok(Self::Amber),
            _ => {
                let preset = value.strip_prefix("sgb-").unwrap_or("").as_bytes();
                if let [group @ b'1'..=b'4', shade @ b'a'..=b'h'] = preset {
                    return Ok(Self::Sgb((group - b'1') * 8 + shade - b'a'));
                }
                Err(format!("--filter must be gray, dmg, pocket, gbc, sgb, sgb-1a..sgb-4h or amber, got {value}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_has_a_label_cli_spelling_and_cycle_position() {
        let mut filter = Filter::Gray;
        for expected in Filter::ALL {
            assert_eq!(filter, expected);
            assert_eq!(filter.cycle(-1).cycle(1), filter);
            filter = filter.cycle(1);
        }
        assert_eq!(filter, Filter::Gray);
        assert_eq!("sgb".parse::<Filter>().unwrap(), Filter::Sgb(0));
        assert_eq!("gbc".parse::<Filter>().unwrap(), Filter::Gbc);
        for index in 0..32 {
            let filter = Filter::Sgb(index);
            let label = format!("SGB {}-{}", index / 8 + 1, char::from(b'A' + index % 8));
            assert_eq!(filter.label(), label);
            let cli = format!("sgb-{}{}", index / 8 + 1, char::from(b'a' + index % 8));
            assert_eq!(cli.parse::<Filter>().unwrap(), filter);
        }
        for invalid in ["sgb-0a", "sgb-5a", "sgb-1i", "sgb-1aa", "sgb-32"] {
            assert!(invalid.parse::<Filter>().is_err());
        }
    }

    #[test]
    fn legacy_ramps_preserve_all_four_mapped_shades() {
        for (filter, expected) in [
            (
                Filter::Gray,
                [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]],
            ),
            (
                Filter::Dmg,
                [[155, 188, 15], [139, 172, 15], [48, 98, 48], [15, 56, 15]],
            ),
            (
                Filter::Pocket,
                [[255, 255, 255], [192, 192, 192], [96, 96, 96], [0, 0, 0]],
            ),
            (
                Filter::Amber,
                [[255, 235, 180], [230, 170, 80], [140, 80, 20], [30, 15, 0]],
            ),
        ] {
            let palette = filter.palette(&[0; 0x18]);
            for source in [PaletteSource::Bg, PaletteSource::Obj0, PaletteSource::Obj1] {
                for shade in 0..4 {
                    assert_eq!(
                        palette.rgb(PalettePixel { source, shade }),
                        expected[shade as usize]
                    );
                }
            }
        }
    }
}
