use crate::input::Bindings;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Zoom {
    #[default]
    Auto,
    Integer(u8),
}

/// Display colour filter. The simulation always produces the four DMG shades
/// (`$FF`, `$AA`, `$55`, `$00`); a filter maps them to RGB at presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Filter {
    #[default]
    Gray,
    Dmg,
    Pocket,
    Sgb,
    Amber,
}

impl Filter {
    pub const ALL: [Filter; 5] = [
        Filter::Gray,
        Filter::Dmg,
        Filter::Pocket,
        Filter::Sgb,
        Filter::Amber,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Filter::Gray => "Grayscale",
            Filter::Dmg => "DMG green",
            Filter::Pocket => "Pocket",
            Filter::Sgb => "SGB color",
            Filter::Amber => "Amber",
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        let index = Self::ALL.iter().position(|f| *f == self).unwrap_or(0) as i32;
        Self::ALL[(index + delta).rem_euclid(Self::ALL.len() as i32) as usize]
    }

    /// The SGB palette is the cream/cyan/brown title reference; Pocket is a
    /// cooler grey ramp than the raw DMG shades.
    pub fn rgb(self, gray: u8) -> [u8; 3] {
        let shade = match gray {
            255 => 0,
            170 => 1,
            85 => 2,
            0 => 3,
            v if v >= 213 => 0,
            v if v >= 128 => 1,
            v if v >= 43 => 2,
            _ => 3,
        };
        let palette: [[u8; 3]; 4] = match self {
            Filter::Gray => [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]],
            Filter::Dmg => [[155, 188, 15], [139, 172, 15], [48, 98, 48], [15, 56, 15]],
            Filter::Pocket => [[255, 255, 255], [192, 192, 192], [96, 96, 96], [0, 0, 0]],
            Filter::Sgb => [[248, 240, 200], [88, 216, 232], [168, 120, 48], [56, 56, 56]],
            Filter::Amber => [[255, 235, 180], [230, 170, 80], [140, 80, 20], [30, 15, 0]],
        };
        palette[shade]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    /// Window size in logical UI points; game scaling uses physical pixels.
    pub window_size: [f32; 2],
    pub fullscreen: bool,
    pub zoom: Zoom,
    pub filter: Filter,
    pub volume: f32,
    pub muted: bool,
    pub dead_zone: f32,
    pub preferred_controller: Option<String>,
    pub bindings: Bindings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            window_size: [800.0, 720.0],
            fullscreen: false,
            zoom: Zoom::Auto,
            filter: Filter::Gray,
            volume: 0.7,
            muted: false,
            dead_zone: 0.3,
            preferred_controller: None,
            bindings: Bindings::default(),
        }
    }
}

impl Settings {
    pub fn validate(&mut self) -> Result<(), String> {
        if self.version != SETTINGS_VERSION {
            return Err(format!("Settings version {} is unsupported", self.version));
        }
        for (value, minimum) in self.window_size.iter_mut().zip([480.0, 400.0]) {
            if !value.is_finite() {
                return Err("Invalid window size".into());
            }
            *value = value.clamp(minimum, 7680.0);
        }
        if let Zoom::Integer(scale) = &mut self.zoom {
            *scale = (*scale).clamp(1, 8);
        }
        if !self.volume.is_finite() || !self.dead_zone.is_finite() {
            return Err("Invalid audio or controller setting".into());
        }
        self.volume = self.volume.clamp(0.0, 1.0);
        self.dead_zone = self.dead_zone.clamp(0.1, 0.8);
        self.bindings.validate()?;
        Ok(())
    }

    /// Invalid settings are reported to the UI, never silently overwritten.
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
        };
        let mut settings: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let mut settings = self.clone();
        settings.validate()?;
        let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
        atomic_write(path, &bytes).map_err(|e| e.to_string())
    }
}

/// Same-directory replacement keeps a partial write from replacing a valid file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub struct AppPaths {
    pub settings: PathBuf,
    pub save: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self, String> {
        let dirs = directories::ProjectDirs::from("", "", "sml2-rs")
            .ok_or("Cannot find the user configuration directory")?;
        Ok(Self {
            settings: dirs.config_dir().join("settings.json"),
            save: dirs.data_local_dir().join("saves/default.sav"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_round_trip_and_reject_future_versions_without_replacing_them() {
        let path = std::env::temp_dir().join(format!("sml2_settings_{}.json", std::process::id()));
        let mut settings = Settings {
            zoom: Zoom::Integer(3),
            filter: Filter::Sgb,
            ..Default::default()
        };
        settings.bindings.0[4] = vec![crate::input::Button::key("K")];
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), settings);
        let future = br#"{"version":999}"#;
        fs::write(&path, future).unwrap();
        assert!(Settings::load(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), future);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn filters_cycle_and_map_the_four_dmg_shades() {
        assert_eq!(Filter::Gray.cycle(1), Filter::Dmg);
        assert_eq!(Filter::Gray.cycle(-1), Filter::Amber);
        assert_eq!(Filter::Sgb.cycle(5), Filter::Sgb);
        assert_eq!(Filter::Sgb.rgb(255), [248, 240, 200]);
        assert_eq!(Filter::Sgb.rgb(170), [88, 216, 232]);
        assert_eq!(Filter::Sgb.rgb(85), [168, 120, 48]);
        assert_eq!(Filter::Sgb.rgb(0), [56, 56, 56]);
        assert_eq!(Filter::Dmg.rgb(85), [48, 98, 48]);
    }
}
