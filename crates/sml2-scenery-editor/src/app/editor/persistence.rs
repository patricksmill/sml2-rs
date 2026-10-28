//! Compatible per-level scenery files, with atomic gesture-level autosave.

use super::EditorApp;
use crate::settings::atomic_write;
use sml2_core::scenery::SceneryOverlay;
use std::{fs, path::PathBuf};

impl EditorApp {
    /// Reload only a blocked workspace, after the user repairs its file.
    pub(super) fn reload_scenery(&mut self) {
        let Some(work) = &self.editor.work else {
            return;
        };
        if work.load_error.is_none() {
            return;
        }
        let result = self.read_scenery(self.editor.level, work.count, work.expanded);
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

    /// The draft and export name of the selected target's scenery.
    pub(super) fn scenery_file(&self, level: u8, expanded: bool) -> String {
        match self.editor.scene {
            Some(scene) => format!("scenery-{}.json", scene.file_stem()),
            None if expanded => format!("scenery-{level:02}-expanded.json"),
            None => format!("scenery-{level:02}.json"),
        }
    }

    fn scenery_path(&self, level: u8, expanded: bool) -> PathBuf {
        self.options
            .paths
            .settings
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(self.scenery_file(level, expanded))
    }

    pub(super) fn read_scenery(
        &self,
        level: u8,
        count: usize,
        expanded: bool,
    ) -> Result<SceneryOverlay, String> {
        let path = self.scenery_path(level, expanded);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(self
                    .game
                    .as_ref()
                    .map(|game| {
                        if let Some(scene) = self.editor.scene {
                            game.session.scene_scenery(scene).clone()
                        } else if expanded {
                            self.editor_level_content(level)
                                .filter(|(_, rebuilt, _)| *rebuilt)
                                .map(|(content, _, _)| content.scenery)
                                .unwrap_or_default()
                        } else {
                            game.session
                                .assets
                                .level(level)
                                .map(|content| content.scenery.clone())
                                .unwrap_or_default()
                        }
                    })
                    .unwrap_or_default())
            }
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        let overlay: SceneryOverlay =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        SceneryOverlay::try_from_paints(overlay.paints(), count)
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    pub(super) fn save_scenery(&mut self) {
        let Some(expanded) = self.editor.work.as_ref().map(|work| work.expanded) else {
            return;
        };
        let path = self.scenery_path(self.editor.level, expanded);
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
