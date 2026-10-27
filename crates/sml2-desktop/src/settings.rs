use crate::input::Bindings;
use serde::{Deserialize, Serialize};
pub use sml2_runtime::persistence::atomic_write;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const SETTINGS_VERSION: u32 = 3;

mod filter;
pub use filter::Filter;
mod view;
pub use sml2_runtime::render::HudStyle;
pub use view::{CameraZoom, Widescreen};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VanillaInstall {
    pub revision: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Zoom {
    #[default]
    Auto,
    Integer(u8),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    /// Window size in logical UI points; game scaling uses physical pixels.
    pub window_size: [f32; 2],
    pub fullscreen: bool,
    pub zoom: Zoom,
    /// Restrict output scaling to whole physical pixels per game pixel.
    pub integer_scaling: bool,
    pub filter: Filter,
    /// Where levels draw the status bar.
    pub hud_style: HudStyle,
    pub volume: f32,
    #[serde(default = "full_volume")]
    pub music_volume: f32,
    #[serde(default = "full_volume")]
    pub effects_volume: f32,
    pub muted: bool,
    pub dead_zone: f32,
    pub preferred_controller: Option<String>,
    pub bindings: Bindings,
    pub dx_installation: Option<sml2_dx::cache::CacheKey>,
    /// Present covered screens with the installed DX graphics and colors.
    pub dx_mode: bool,
    /// Who DX Mode draws; native physics are the same for both.
    pub character: sml2_core::character::Character,
    /// Widen the game view (Mods). Changes object spawning while enlarged.
    pub widescreen: Widescreen,
    pub camera_zoom: CameraZoom,
    /// Draw levels between the last two ticks on fast displays (Mods).
    pub motion_smoothing: bool,
    /// Draw the top toolbar. F10 toggles this while it is hidden.
    #[serde(default = "toolbar_shown")]
    pub show_toolbar: bool,
    #[serde(default)]
    pub vanilla: Option<VanillaInstall>,
}

fn toolbar_shown() -> bool {
    true
}

fn full_volume() -> f32 {
    1.0
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            window_size: [800.0, 720.0],
            fullscreen: false,
            zoom: Zoom::Auto,
            integer_scaling: true,
            filter: Filter::Gray,
            hud_style: HudStyle::Classic,
            volume: 0.7,
            music_volume: 1.0,
            effects_volume: 1.0,
            muted: false,
            dead_zone: 0.3,
            preferred_controller: None,
            bindings: Bindings::default(),
            dx_installation: None,
            dx_mode: false,
            character: Default::default(),
            widescreen: Widescreen::Off,
            camera_zoom: CameraZoom::Native,
            motion_smoothing: false,
            show_toolbar: true,
            vanilla: None,
        }
    }
}

/// Output levels in `0.0..=1.0`. Music and effects weight the APU channels the
/// driver gives each this frame; the master level scales the finished mix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume {
    pub master: f32,
    pub music: f32,
    pub effects: f32,
}

impl From<f32> for Volume {
    fn from(master: f32) -> Self {
        Self {
            master,
            music: 1.0,
            effects: 1.0,
        }
    }
}

impl Settings {
    pub fn view(&self) -> (usize, usize) {
        self.view_for_window(self.window_size.map(|size| size as u32))
    }

    /// Resolve the game view against the live drawable area, in physical pixels.
    /// Fixed aspect ratios do not depend on the window dimensions.
    pub fn view_for_window(&self, window: [u32; 2]) -> (usize, usize) {
        view::view_size(self.widescreen, self.camera_zoom, window)
    }

    /// The levels the game mixes audio at; silent while muted.
    pub fn audio_volume(&self) -> Volume {
        Volume {
            master: if self.muted { 0.0 } else { self.volume },
            music: self.music_volume,
            effects: self.effects_volume,
        }
    }

    pub fn validate(&mut self) -> Result<(), String> {
        if self.version != SETTINGS_VERSION {
            return Err(format!("Settings version {} is unsupported", self.version));
        }
        if matches!(self.filter, Filter::Sgb(index) if index >= 32) {
            return Err("SGB preset must be in 1-A through 4-H".into());
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
        let levels = [self.volume, self.music_volume, self.effects_volume];
        if !levels.iter().all(|v| v.is_finite()) || !self.dead_zone.is_finite() {
            return Err("Invalid audio or controller setting".into());
        }
        for level in [
            &mut self.volume,
            &mut self.music_volume,
            &mut self.effects_volume,
        ] {
            *level = level.clamp(0.0, 1.0);
        }
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
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let version = value.get("version");
        if value.is_object()
            && (version.is_none()
                || version.and_then(serde_json::Value::as_u64) == Some(1)
                || version.and_then(serde_json::Value::as_u64) == Some(2))
        {
            value["version"] = SETTINGS_VERSION.into();
            if value.get("filter").and_then(serde_json::Value::as_str) == Some("Sgb") {
                value["filter"] = serde_json::json!({"Sgb": 0});
            }
        }
        let mut settings: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
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
    fn widescreen_modes_round_trip_without_changing_existing_settings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        for widescreen in [
            Widescreen::Off,
            Widescreen::Wide16x9,
            Widescreen::Wide21x9,
            Widescreen::WindowSize,
        ] {
            let settings = Settings {
                widescreen,
                camera_zoom: CameraZoom::Zoom150,
                window_size: [1280.0, 1024.0],
                ..Default::default()
            };
            settings.save(&path).unwrap();
            assert_eq!(Settings::load(&path).unwrap(), settings);
        }
    }

    #[test]
    fn settings_round_trip_and_reject_future_versions_without_replacing_them() {
        let path = std::env::temp_dir().join(format!("sml2_settings_{}.json", std::process::id()));
        let mut settings = Settings {
            zoom: Zoom::Integer(3),
            integer_scaling: false,
            filter: Filter::Sgb(0),
            hud_style: HudStyle::Modern,
            ..Default::default()
        };
        settings.bindings.0[4] = vec![crate::input::Button::key("K")];
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), settings);
        let future = br#"{"version":999,"filter":"Sgb"}"#;
        fs::write(&path, future).unwrap();
        assert!(Settings::load(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), future);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn version_one_sgb_migrates_without_losing_other_settings() {
        let path =
            std::env::temp_dir().join(format!("sml2_settings_v1_{}.json", std::process::id()));
        let original = Settings {
            filter: Filter::Sgb(0),
            zoom: Zoom::Integer(4),
            volume: 0.25,
            muted: true,
            preferred_controller: Some("controller".into()),
            ..Default::default()
        };
        let mut value = serde_json::to_value(&original).unwrap();
        value["version"] = 1.into();
        value["filter"] = "Sgb".into();
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), original);
        value.as_object_mut().unwrap().remove("version");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            Settings::load(&path).unwrap(),
            original,
            "versionless SGB settings also migrate"
        );
        fs::write(&path, br#"{"filter":"Gray"}"#).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), Settings::default());
        original.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), original);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn version_two_settings_restore_optional_dx_selection() {
        let mut settings = Settings::default();
        let mut value = serde_json::to_value(&settings).unwrap();
        for field in [
            "dx_installation",
            "dx_mode",
            "character",
            "widescreen",
            "camera_zoom",
            "motion_smoothing",
            "hud_style",
        ] {
            value.as_object_mut().unwrap().remove(field);
        }
        assert_eq!(serde_json::from_value::<Settings>(value).unwrap(), settings);
        settings.dx_installation = Some(sml2_dx::cache::CacheKey::new("1".repeat(64)));
        settings.dx_mode = true;
        settings.character = sml2_core::character::Character::Luigi;
        let restored: Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(restored, settings);
    }

    #[test]
    fn version_two_without_show_toolbar_loads_as_version_three_with_the_bar() {
        let path = std::env::temp_dir().join(format!(
            "sml2_settings_v2_toolbar_{}.json",
            std::process::id()
        ));
        fs::write(&path, br#"{"version":2}"#).unwrap();
        let settings = Settings::load(&path).unwrap();
        assert_eq!(settings.version, 3);
        assert!(settings.show_toolbar);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn the_hud_style_saved_as_corner_loads_as_modern() {
        let settings: Settings = serde_json::from_str(r#"{"hud_style":"Corner"}"#).unwrap();
        assert_eq!(settings.hud_style, HudStyle::Modern);
    }

    #[test]
    fn rejects_invalid_sgb_presets() {
        let mut settings = Settings {
            filter: Filter::Sgb(32),
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn a_version_three_file_without_vanilla_still_loads() {
        let path =
            std::env::temp_dir().join(format!("sml2_vanilla_settings_{}.json", std::process::id()));
        let settings = Settings::default();
        settings.save(&path).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        value.as_object_mut().unwrap().remove("vanilla");
        value.as_object_mut().unwrap().remove("integer_scaling");
        std::fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert!(loaded.vanilla.is_none());
        assert!(loaded.integer_scaling);
        assert_eq!(loaded.version, 3);
        std::fs::remove_file(path).unwrap();
    }
}
