//! Separate workspaces for levels and the protected borders of fixed scenes.

use super::{EditorApp, Workspace};
use eframe::egui::{self, Ui};
use sml2_core::level::{LevelLayout, LAYOUT_SIZE, LAYOUT_WIDTH};
use sml2_core::world::World;
use sml2_runtime::scene_scenery::SceneTarget;

impl EditorApp {
    pub(super) fn target_picker(&mut self, ui: &mut Ui) {
        let old = (self.editor.level, self.editor.scene);
        let mut selected = old;
        let label = self
            .editor
            .scene
            .map(|scene| scene.label())
            .unwrap_or_else(|| {
                self.levels
                    .iter()
                    .find(|(id, _)| *id == self.editor.level)
                    .map(|(_, label)| label.clone())
                    .unwrap_or_else(|| "Level".into())
            });
        egui::ComboBox::from_id_salt("scenery-target")
            .width(190.0)
            .selected_text(label)
            .show_ui(ui, |ui| {
                ui.strong("Levels");
                for (id, label) in &self.levels {
                    ui.selectable_value(&mut selected, (*id, None), label);
                }
                if let Some(game) = &self.game {
                    ui.separator();
                    ui.strong("Scene surroundings");
                    for scene in game.session.scenery_scenes() {
                        ui.selectable_value(
                            &mut selected,
                            (self.editor.level, Some(scene)),
                            scene.label(),
                        );
                    }
                }
            });
        if selected != old {
            if let Some(scene) = selected.1 {
                self.select_editor_scene(scene);
            } else {
                self.select_editor_level(selected.0);
            }
        }
    }

    pub(super) fn select_editor_scene(&mut self, scene: SceneTarget) {
        if self.editor.scene == Some(scene) {
            return;
        }
        self.finish_editor_gesture();
        if let Some(work) = self.editor.work.take() {
            if let Some(old) = self.editor.scene {
                self.editor.scenes.insert(old, work);
            } else {
                self.editor.levels.insert(self.editor.level, work);
            }
        }
        self.editor.scene = Some(scene);
        self.editor.work = self.editor.scenes.remove(&scene);
        self.load_editor_scene_projection();
        if self.editor.work.is_none() {
            self.load_editor_scene();
        }
        self.editor.atlas = None;
        self.editor.canvas_cache.clear();
        self.editor.hover = None;
    }

    fn load_editor_scene_projection(&mut self) -> bool {
        let Some(scene) = self.editor.scene else {
            return false;
        };
        let Some(game) = &mut self.game else {
            return false;
        };
        if let Err(error) = game.session.stage_scenery_scene(scene) {
            self.status = Some(error);
            return false;
        }
        true
    }

    pub(super) fn load_editor_scene(&mut self) {
        if !self.load_editor_scene_projection() {
            return;
        }
        let mut layout = LevelLayout {
            cells: vec![u8::MAX; LAYOUT_SIZE],
            world: World::ROM,
        };
        for y in 0..9 {
            for x in 0..10 {
                layout.cells[y * LAYOUT_WIDTH + x] = (y * 10 + x) as u8;
            }
        }
        let mut work = Workspace::new(layout, 90, egui::pos2(80.0, 72.0));
        work.scene_border = true;
        match self.read_scenery(self.editor.level, 90, false) {
            Ok(scenery) => work.scenery = scenery,
            Err(error) => work.load_error = Some(error),
        }
        self.editor.work = Some(work);
    }
}
