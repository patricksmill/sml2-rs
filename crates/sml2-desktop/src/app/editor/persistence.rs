//! Compatible per-level scenery files, with atomic gesture-level autosave.

use super::DesktopApp;
use crate::settings::atomic_write;
use sml2_core::scenery::SceneryOverlay;
use std::{fs, path::PathBuf};

impl DesktopApp {
    fn scenery_path(&self, level: u8) -> PathBuf {
        self.options
            .paths
            .settings
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(format!("scenery-{level:02}.json"))
    }

    pub(super) fn read_scenery(&self, level: u8, count: usize) -> Result<SceneryOverlay, String> {
        let path = self.scenery_path(level);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SceneryOverlay::empty())
            }
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        let overlay: SceneryOverlay =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        SceneryOverlay::try_from_paints(overlay.paints, count)
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    pub(super) fn save_scenery(&mut self) {
        let path = self.scenery_path(self.editor.level);
        let Some(work) = &mut self.editor.work else {
            return;
        };
        if !work.dirty
            || work.gesture.is_some()
            || work.load_error.is_some()
            || work.save_error.is_some()
        {
            return;
        }
        let saved = serde_json::to_vec_pretty(&work.scenery)
            .map_err(|e| e.to_string())
            .and_then(|bytes| atomic_write(&path, &bytes).map_err(|e| e.to_string()));
        match saved {
            Ok(()) => work.dirty = false,
            Err(error) => work.save_error = Some(error),
        }
    }
}
