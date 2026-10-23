//! Scenery editor: paint metatiles outside the original level picture.

use super::{overlay_rect, DesktopApp, Screen};
use crate::app::theme;
use eframe::egui::{self, Color32, ColorImage, Context, Sense, TextureOptions, Ui};
use sml2_core::level::Metatile;
use sml2_core::scenery::{SceneryOverlay, SceneryPaint};
use sml2_core::video::{bg_tile_offset, bgp_shades, tile_colors};
use sml2_runtime::render::motion::Smooth;
use sml2_runtime::render::{paintable_cell, view};
use sml2_runtime::session::SessionState;
use std::fs;
use std::path::PathBuf;

pub(super) struct Editor {
    pub level: u8,
    pub metatile: u8,
    pub cam: (u16, u16),
    pub scenery: SceneryOverlay,
    metatile_count: usize,
    palette: Vec<u8>,
    swatches: Vec<(u8, ColorImage)>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            level: 0,
            metatile: 0,
            cam: (0, 0),
            scenery: SceneryOverlay::empty(),
            metatile_count: 0,
            palette: Vec::new(),
            swatches: Vec::new(),
        }
    }
}

impl DesktopApp {
    pub(super) fn open_editor(&mut self, ctx: &Context) {
        if self.game.is_none() {
            self.load_game();
        }
        if self.game.is_none() {
            return;
        }
        self.load_editor_level();
        self.navigate(ctx, Screen::Editor);
    }

    pub(super) fn paint_cell(&mut self, x: i16, y: i16) -> Result<(), String> {
        self.editor.scenery.set(
            SceneryPaint {
                x,
                y,
                metatile: self.editor.metatile,
            },
            self.editor.metatile_count,
        )?;
        self.save_scenery();
        Ok(())
    }

    pub(super) fn erase_cell(&mut self, x: i16, y: i16) {
        self.editor.scenery.erase(x, y);
        self.save_scenery();
    }

    pub(super) fn play_editor(&mut self, ctx: &Context) {
        self.save_scenery();
        let id = self.editor.level;
        let scenery = self.editor.scenery.clone();
        let Some(game) = &mut self.game else {
            return;
        };
        let Ok(mut content) = game.session.assets.level(id).cloned() else {
            return;
        };
        content.scenery = scenery;
        game.session.assets.insert_level(id, content);
        if let Err(error) = game.session.begin_level(id, 0) {
            self.status = Some(error);
            return;
        }
        self.navigate(ctx, Screen::Game);
    }

    pub(super) fn editor_view(&mut self, ctx: &Context, ui: &mut Ui) {
        self.sync_editor();
        if let Some(image) = self.frame_image(Smooth::OFF) {
            self.texture.set(image, TextureOptions::NEAREST);
        }
        ui.horizontal(|ui| {
            self.editor_tools(ctx, ui);
            self.editor_canvas(ui);
        });
    }

    fn editor_tools(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.set_width(220.0);
            ui.label(
                "Paint past the level, where the map ends. Anywhere Mario can walk stays the real level. Right click erases.",
            );
            if !view::enlarged(self.settings.view()) {
                theme::error(
                    ui,
                    "Turn on widescreen or camera zoom so the view reaches past the level.",
                );
            }
            let mut level = self.editor.level;
            egui::ComboBox::from_id_salt("scenery-level")
                .selected_text(
                    self.levels
                        .iter()
                        .find(|(id, _)| *id == level)
                        .map(|(_, label)| label.as_str())
                        .unwrap_or("level"),
                )
                .show_ui(ui, |ui| {
                    for (id, label) in &self.levels {
                        ui.selectable_value(&mut level, *id, label);
                    }
                });
            if level != self.editor.level {
                self.editor.level = level;
                self.load_editor_level();
            }
            ui.horizontal(|ui| {
                if ui.button("Camera left").clicked() {
                    self.editor.cam.0 = self.editor.cam.0.saturating_sub(16);
                }
                if ui.button("Camera right").clicked() {
                    self.editor.cam.0 = self.editor.cam.0.saturating_add(16);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Camera up").clicked() {
                    self.editor.cam.1 = self.editor.cam.1.saturating_sub(16);
                }
                if ui.button("Camera down").clicked() {
                    self.editor.cam.1 = self.editor.cam.1.saturating_add(16);
                }
            });
            ui.label(format!("{} paints", self.editor.scenery.paints.len()));
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (id, image) in &self.editor.swatches {
                            let texture = ctx.load_texture(
                                format!("scenery-swatch-{id}"),
                                image.clone(),
                                TextureOptions::NEAREST,
                            );
                            let selected = *id == self.editor.metatile;
                            if ui
                                .add(egui::Button::image(&texture).selected(selected))
                                .on_hover_text(format!("metatile {id}"))
                                .clicked()
                            {
                                self.editor.metatile = *id;
                            }
                        }
                    });
                });
            if ui.button("Play this level").clicked() {
                self.play_editor(ctx);
            }
            if ui.button("Back").clicked() {
                self.navigate(ctx, Screen::Home);
            }
        });
    }

    fn editor_canvas(&mut self, ui: &mut Ui) {
        let view = self.settings.view();
        let available = ui.available_size();
        let scale = (available.x / view.0 as f32)
            .min(available.y / view.1 as f32)
            .floor()
            .max(1.0_f32);
        let size = egui::vec2(view.0 as f32 * scale, view.1 as f32 * scale);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        ui.painter().image(
            self.texture.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0_f32, 1.0_f32)),
            Color32::WHITE,
        );
        let (mx, my) = view::level_margin(view);
        ui.painter().rect_stroke(
            overlay_rect(rect, view, (mx, my, 160, 144)),
            0.0_f32,
            egui::Stroke::new(2.0_f32, theme::MAGENTA),
            egui::StrokeKind::Inside,
        );
        if response.clicked() || response.secondary_clicked() || response.dragged() {
            let Some(pos) = response.interact_pointer_pos() else {
                return;
            };
            let px = ((pos.x - rect.min.x) / scale) as i32;
            let py = ((pos.y - rect.min.y) / scale) as i32;
            let erase = response.secondary_clicked() || ui.input(|i| i.pointer.secondary_down());
            match paintable_cell(view, px, py, self.editor.cam, 0) {
                Some((x, y)) if erase => self.erase_cell(x, y),
                Some((x, y)) => {
                    if let Err(error) = self.paint_cell(x, y) {
                        self.status = Some(error);
                    }
                }
                None if response.clicked() || response.secondary_clicked() => {
                    self.status = Some("That spot is part of the level.".into());
                }
                None => {}
            }
        }
    }

    fn load_editor_level(&mut self) {
        let id = self.editor.level;
        let Some(game) = &self.game else {
            return;
        };
        let Ok(content) = game.session.assets.level(id) else {
            return;
        };
        let cam = (
            content.manifest.header.camera_start_x,
            content.manifest.header.camera_start_y,
        );
        let count = content.manifest.metatiles.metatiles.len();
        let palette = content.layout.distinct_metatile_ids();
        let bgp = content.manifest.header.palettes[0];
        let swatches = palette
            .iter()
            .filter_map(|id| {
                content
                    .manifest
                    .metatiles
                    .get(*id)
                    .map(|metatile| (*id, swatch(&content.bg_base, bgp, metatile)))
            })
            .collect();
        self.editor.cam = cam;
        self.editor.metatile_count = count;
        self.editor.palette = palette;
        self.editor.swatches = swatches;
        if !self.editor.palette.contains(&self.editor.metatile) {
            self.editor.metatile = self.editor.palette.first().copied().unwrap_or(0);
        }
        self.editor.scenery = self.read_scenery(count);
    }

    fn sync_editor(&mut self) {
        let id = self.editor.level;
        let cam = self.editor.cam;
        let scenery = self.editor.scenery.clone();
        let Some(game) = &mut self.game else {
            return;
        };
        let current = matches!(
            &game.session.state,
            SessionState::InLevel(ctx) if ctx.manifest.level_id == id
        );
        if !current && game.session.begin_level(id, 0).is_err() {
            return;
        }
        game.session.preview_scenery(cam, scenery);
    }

    fn scenery_path(&self) -> PathBuf {
        self.options
            .paths
            .settings
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(format!("scenery-{:02}.json", self.editor.level))
    }

    fn read_scenery(&self, metatile_count: usize) -> SceneryOverlay {
        let Ok(text) = fs::read_to_string(self.scenery_path()) else {
            return SceneryOverlay::empty();
        };
        let Ok(overlay) = serde_json::from_str::<SceneryOverlay>(&text) else {
            return SceneryOverlay::empty();
        };
        SceneryOverlay::try_from_paints(overlay.paints, metatile_count)
            .unwrap_or_else(|_| SceneryOverlay::empty())
    }

    fn save_scenery(&mut self) {
        let path = self.scenery_path();
        if let Some(parent) = path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                self.status = Some(format!("Could not save scenery: {error}"));
                return;
            }
        }
        match serde_json::to_string_pretty(&self.editor.scenery) {
            Ok(text) => {
                if let Err(error) = fs::write(&path, text) {
                    self.status = Some(format!("Could not save scenery: {error}"));
                }
            }
            Err(error) => self.status = Some(format!("Could not save scenery: {error}")),
        }
    }
}

fn swatch(bg: &[u8], bgp: u8, metatile: &Metatile) -> ColorImage {
    let shades = bgp_shades(bgp);
    let mut pixels = vec![Color32::from_gray(shades[0]); 16 * 16];
    for (quad, &tile) in metatile.tiles.iter().enumerate() {
        let (ox, oy) = ((quad % 2) * 8, (quad / 2) * 8);
        let offset = bg_tile_offset(tile);
        let Some(bytes) = bg.get(offset..offset + 16) else {
            continue;
        };
        let Ok(bytes) = <[u8; 16]>::try_from(bytes) else {
            continue;
        };
        let colors = tile_colors(&bytes);
        for y in 0..8 {
            for x in 0..8 {
                pixels[(oy + y) * 16 + ox + x] =
                    Color32::from_gray(shades[colors[y * 8 + x] as usize]);
            }
        }
    }
    ColorImage::new([16, 16], pixels)
}
