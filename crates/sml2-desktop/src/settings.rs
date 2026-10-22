use crate::input::Bindings;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

pub const SETTINGS_VERSION: u32 = 2;

mod filter;
pub use filter::Filter;

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
    pub filter: Filter,
    pub volume: f32,
    pub muted: bool,
    pub dead_zone: f32,
    pub preferred_controller: Option<String>,
    pub bindings: Bindings,
    pub dx_installation: Option<sml2_dx::cache::CacheKey>,
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
            dx_installation: None,
        }
    }
}

impl Settings {
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
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let version = value.get("version");
        if value.is_object()
            && (version.is_none() || version.and_then(serde_json::Value::as_u64) == Some(1))
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
            filter: Filter::Sgb(0),
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
        value.as_object_mut().unwrap().remove("dx_installation");
        assert_eq!(
            serde_json::from_value::<Settings>(value)
                .unwrap()
                .dx_installation,
            None
        );
        settings.dx_installation = Some(sml2_dx::cache::CacheKey::new("1".repeat(64)));
        let restored: Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(restored, settings);
    }

    #[test]
    fn rejects_invalid_sgb_presets() {
        let mut settings = Settings {
            filter: Filter::Sgb(32),
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }
}
