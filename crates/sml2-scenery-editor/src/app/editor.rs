//! Scenery authoring workspace. The live game is used only for playtesting.

use super::{theme, EditorApp, Screen};
use eframe::egui::{self, Context, PointerButton, Rect, Ui};
use sml2_runtime::content::LevelContent;
use state::{Cell, Tool, Workspace};
use std::collections::BTreeMap;

mod canvas;
mod export;
mod palette;
mod persistence;
mod state;
mod targets;
#[cfg(test)]
pub(super) mod tests;

#[derive(Default)]
pub(super) struct Editor {
    pub level: u8,
    scene: Option<sml2_runtime::scene_scenery::SceneTarget>,
    scenes: BTreeMap<sml2_runtime::scene_scenery::SceneTarget, Workspace>,
    export: Option<export::ExportDialog>,
    work: Option<Workspace>,
    levels: BTreeMap<u8, Workspace>,
    atlas: Option<palette::Atlas>,
    canvas_cache: canvas::CanvasCache,
    all_tiles: bool,
    pub playtesting: bool,
    canvas_rect: Option<Rect>,
    hover: Option<Cell>,
    gesture_button: Option<PointerButton>,
    pan_button: Option<PointerButton>,
}

impl EditorApp {
    pub(super) fn open_editor(&mut self, ctx: &Context) {
        if self.game.is_none() {
            self.load_game();
        }
        if self.game.is_none() {
            return;
        }
        if self.settings.dx_mode && self.dx_bundle.is_none() && self.dx_worker.is_none() {
            self.restore_dx();
        }
        if self.editor.work.is_none() {
            if self.editor.scene.is_some() {
                self.load_editor_scene();
            } else {
                self.load_editor_level();
            }
        }
        self.editor.playtesting = false;
        self.navigate(ctx, Screen::Editor);
    }

    /// Level `id` as the game plays it: rebuilt by its recipe while the
    /// game's Expanded levels setting is on. Like the game, a recipe that
    /// fails loads the original, and the error is returned beside it.
    pub(super) fn editor_level_content(
        &self,
        id: u8,
    ) -> Option<(LevelContent, bool, Option<String>)> {
        let assets = &self.game.as_ref()?.session.assets;
        let content = assets.level(id).ok()?;
        match assets
            .expanded_level(content)
            .filter(|_| self.settings.expanded_levels)
        {
            Some(Ok(rebuilt)) => Some((rebuilt, true, None)),
            Some(Err(error)) => Some((content.clone(), false, Some(error))),
            None => Some((content.clone(), false, None)),
        }
    }

    fn load_editor_level(&mut self) {
        let Some((content, expanded, error)) = self.editor_level_content(self.editor.level) else {
            return;
        };
        if error.is_some() {
            self.status = error;
        }
        let initial_scenery = content.scenery.clone();
        let mut work = Workspace::new(
            content.layout.clone(),
            content.manifest.metatiles.metatiles.len(),
            egui::pos2(
                f32::from(content.manifest.header.camera_start_x),
                f32::from(content.manifest.header.camera_start_y),
            ),
        );
        work.scenery = initial_scenery;
        work.expanded = expanded;
        match self.read_scenery(self.editor.level, work.count, expanded) {
            Ok(scenery) => work.scenery = scenery,
            Err(error) => work.load_error = Some(error),
        }
        self.editor.work = Some(work);
    }

    fn select_editor_level(&mut self, id: u8) {
        if id == self.editor.level && self.editor.scene.is_none() {
            return;
        }
        self.finish_editor_gesture();
        if let Some(work) = self.editor.work.take() {
            if let Some(scene) = self.editor.scene {
                self.editor.scenes.insert(scene, work);
            } else {
                self.editor.levels.insert(self.editor.level, work);
            }
        }
        self.editor.scene = None;
        self.editor.level = id;
        self.editor.work = self.editor.levels.remove(&id);
        if self.editor.work.is_none() {
            self.load_editor_level();
        }
        self.editor.atlas = None;
        self.editor.canvas_cache.clear();
        self.editor.hover = None;
    }

    pub(super) fn finish_editor_gesture(&mut self) {
        if let Some(work) = &mut self.editor.work {
            work.finish();
        }
        self.editor.gesture_button = None;
        self.editor.pan_button = None;
        self.save_scenery();
    }

    pub(super) fn cancel_editor_gesture(&mut self) -> bool {
        let cancelled = self.editor.work.as_mut().is_some_and(Workspace::cancel);
        self.editor.gesture_button = None;
        self.editor.pan_button = None;
        cancelled
    }

    pub(super) fn play_editor(&mut self, ctx: &Context) {
        self.finish_editor_gesture();
        let Some(work) = &self.editor.work else {
            return;
        };
        if work.load_error.is_some() {
            return;
        }
        let Some(game) = &mut self.game else {
            return;
        };
        if let Some(scene) = self.editor.scene {
            if let Err(error) = game
                .session
                .set_scene_scenery(scene, work.scenery.clone())
                .and_then(|()| game.session.stage_scenery_scene(scene))
            {
                self.status = Some(error);
                return;
            }
            self.paused = false;
            self.editor.playtesting = true;
            self.navigate(ctx, Screen::Game);
            return;
        }
        let id = self.editor.level;
        let Ok(mut content) = game.session.assets.level(id).cloned() else {
            return;
        };
        // A rebuilt level's scenery is in the rebuilt map's cells; the
        // session rebuilds the level at entry and takes it from there.
        if work.expanded {
            content.expanded_scenery = Some(work.scenery.clone());
        } else {
            content.scenery = work.scenery.clone();
        }
        game.session.assets.insert_level(id, content);
        game.session.expanded_levels = work.expanded;
        if let Err(error) = game.session.begin_level(id, 0) {
            self.status = Some(error);
            return;
        }
        self.paused = false;
        self.editor.playtesting = true;
        self.navigate(ctx, Screen::Game);
    }

    pub(super) fn editor_view(&mut self, ctx: &Context, ui: &mut Ui) {
        self.editor_atlas(ctx);
        ui.scope(|ui| {
            ui.spacing_mut().interact_size.y = 26.0;
            ui.spacing_mut().button_padding = egui::vec2(7.0, 4.0);
            egui::Frame::NONE
                .fill(ui.visuals().panel_fill)
                .inner_margin(8)
                .show(ui, |ui| {
                    self.editor_tools(ctx, ui);
                    self.editor.tile_tray(ui);
                });
            egui::TopBottomPanel::bottom("scenery-status")
                .frame(
                    egui::Frame::NONE
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(5),
                )
                .show_inside(ui, |ui| {
                    self.editor_status(ui);
                });
            self.editor.canvas(ui);
        });
        let saving = self.editor.work.as_ref().is_some_and(|w| {
            w.dirty && w.gesture.is_none() && w.load_error.is_none() && w.save_error.is_none()
        });
        self.save_scenery();
        if saving {
            ctx.request_repaint();
        }
    }

    fn editor_tools(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.strong("SCENERY MAKER");
            self.target_picker(ui);
            if ui.button("Play · Enter").clicked() {
                self.play_editor(ctx);
            }
            if ui.button("Export level…").clicked() {
                self.begin_export();
            }
            if ui.button("Back").clicked() {
                self.navigate(ctx, Screen::Home);
            }
        });
        let Some(work) = &mut self.editor.work else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            for (tool, label, hint) in Tool::ALL {
                if ui
                    .add(egui::Button::new(label).selected(work.tool == tool))
                    .on_hover_text(hint)
                    .clicked()
                {
                    work.finish();
                    work.tool = tool;
                }
            }
            ui.separator();
            if ui
                .add_enabled(work.can_undo(), egui::Button::new("Undo"))
                .on_hover_text("Ctrl/Cmd+Z")
                .clicked()
            {
                work.undo();
            }
            if ui
                .add_enabled(work.can_redo(), egui::Button::new("Redo"))
                .on_hover_text("Ctrl/Cmd+Shift+Z")
                .clicked()
            {
                work.redo();
            }
            ui.toggle_value(&mut work.grid, "Grid · G");
            let mut zoom = work.viewport.zoom;
            if ui.button("−").clicked() {
                zoom = zoom.saturating_sub(1).max(1);
            }
            ui.label(format!("{}×", zoom));
            if ui.button("+").clicked() {
                zoom = (zoom + 1).min(8);
            }
            if let Some(rect) = self.editor.canvas_rect {
                work.viewport.zoom_at(rect, rect.center(), zoom);
            } else {
                work.viewport.zoom = zoom;
            }
            if ui.button("Reset view").clicked() {
                work.reset_view();
            }
        });
    }

    fn editor_status(&mut self, ui: &mut Ui) {
        let Some(work) = &self.editor.work else {
            ui.label("No level loaded");
            return;
        };
        if let Some(error) = &work.load_error {
            let reload = ui
                .horizontal_wrapped(|ui| {
                    ui.colored_label(
                        theme::ERROR,
                        format!("File not loaded; editing locked: {error}"),
                    );
                    ui.button("Reload file").clicked()
                })
                .inner;
            if reload {
                self.reload_scenery();
            }
            return;
        }
        if let Some(error) = &work.save_error {
            let retry = ui
                .horizontal_wrapped(|ui| {
                    ui.colored_label(theme::ERROR, format!("Save failed: {error}"));
                    ui.button("Retry").clicked()
                })
                .inner;
            if retry {
                self.editor.work.as_mut().unwrap().save_error = None;
            }
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.strong(if work.dirty { "Saving…" } else { "Saved" });
            ui.label(format!("· {} scenery tiles", work.scenery.len()));
            if let Some(cell) = self.editor.hover {
                ui.label(format!(
                    "· {}, {} · {}",
                    cell.0,
                    cell.1,
                    if work.outside(cell) {
                        "Scenery"
                    } else {
                        "Map scenery"
                    }
                ));
            } else {
                ui.label(
                    "· Wheel: scroll · Ctrl-wheel: zoom · Space-drag: pan · Right-drag: erase",
                );
            }
        });
        if let Some(warning) = self.editor.atlas.as_ref().and_then(|a| a.warning.as_ref()) {
            ui.colored_label(theme::ERROR, warning);
        }
    }

    pub(super) fn editor_shortcuts(&mut self, ctx: &Context) {
        // Ordinary tool buttons retain egui focus too. Only popups and debug
        // text entry should take these shortcuts away from the workspace.
        if self.editor.export.is_some()
            || egui::Popup::is_any_open(ctx)
            || self.debug && ctx.wants_keyboard_input()
        {
            return;
        }
        let Some(work) = &mut self.editor.work else {
            return;
        };
        let (undo, redo, tool, grid, play, zoom_delta) = ctx.input_mut(|i| {
            let modifiers = i.modifiers;
            let mut pressed = |key| i.consume_key(modifiers, key);
            let z = modifiers.command && pressed(egui::Key::Z);
            let mut tool = None;
            let (mut grid, mut play) = (false, false);
            let mut zoom_delta = 0;
            if !modifiers.command && !modifiers.alt {
                for (key, candidate) in [
                    (egui::Key::B, Tool::Brush),
                    (egui::Key::E, Tool::Eraser),
                    (egui::Key::I, Tool::Eyedropper),
                    (egui::Key::R, Tool::Rectangle),
                ] {
                    if pressed(key) {
                        tool = Some(candidate);
                    }
                }
                grid = pressed(egui::Key::G);
                play = pressed(egui::Key::Enter);
                let plus = pressed(egui::Key::Plus) | pressed(egui::Key::Equals);
                let minus = pressed(egui::Key::Minus);
                zoom_delta = i16::from(plus) - i16::from(minus);
            }
            (
                z && !modifiers.shift,
                z && modifiers.shift,
                tool,
                grid,
                play,
                zoom_delta,
            )
        });
        if undo || redo || tool.is_some() || grid || play || zoom_delta != 0 {
            sml2_desktop::ui::release_focus(ctx);
        }
        if undo || redo || tool.is_some() {
            work.finish();
        }
        if undo {
            work.undo();
        }
        if redo {
            work.redo();
        }
        if let Some(tool) = tool {
            work.tool = tool;
        }
        if grid {
            work.grid = !work.grid;
        }
        if zoom_delta != 0 && work.gesture.is_none() {
            if let Some(rect) = self.editor.canvas_rect {
                let zoom = (i16::from(work.viewport.zoom) + zoom_delta).clamp(1, 8) as u8;
                work.viewport.zoom_at(rect, rect.center(), zoom);
            }
        }
        if play {
            self.play_editor(ctx);
        }
    }
}
