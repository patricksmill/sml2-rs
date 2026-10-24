//! Compatible per-level scenery files, with atomic gesture-level autosave.

use super::DesktopApp;
use crate::settings::atomic_write;
use sml2_core::scenery::SceneryOverlay;
use std::{fs, path::PathBuf};

impl DesktopApp {
    /// Prepare saved scenery before the session starts, for every save pipe.
    pub(in crate::app) fn load_saved_scenery(
        &self,
        assets: &mut sml2_runtime::content::SessionAssets,
    ) -> Vec<String> {
        let mut diagnostics = Vec::new();
        for id in 0..32 {
            let Ok(content) = assets.level(id) else {
                continue;
            };
            match self.read_scenery(id, content.manifest.metatiles.metatiles.len()) {
                Ok(scenery) if !scenery.is_empty() => {
                    let mut content = content.clone();
                    content.scenery = scenery;
                    assets.insert_level(id, content);
                }
                Ok(_) => {}
                Err(error) => diagnostics.push(error),
            }
        }
        diagnostics
    }

    /// Reload only a blocked workspace, after the user repairs its file.
    pub(super) fn reload_scenery(&mut self) {
        let Some(work) = &self.editor.work else {
            return;
        };
        if work.load_error.is_none() {
            return;
        }
        let result = self.read_scenery(self.editor.level, work.count);
        let work = self.editor.work.as_mut().unwrap();
        match result {
            Ok(scenery) => {
                work.scenery = scenery;
                work.load_error = None;
                self.editor.canvas_cache.clear();
            }
            Err(error) => work.load_error = Some(error),
        }
    }

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
        SceneryOverlay::try_from_paints(overlay.paints(), count)
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
            Ok(()) => {
                work.dirty = false;
                // Normal entry and re-entry use the saved edits too, without
                // requiring the editor's Playtest action.
                if let Some(game) = &mut self.game {
                    if let Ok(mut content) = game.session.assets.level(self.editor.level).cloned() {
                        content.scenery = work.scenery.clone();
                        game.session.assets.insert_level(self.editor.level, content);
                    }
                }
            }
            Err(error) => work.save_error = Some(error),
        }
    }
}
