//! Signed world-space canvas; gameplay zoom and camera never participate.

use super::state::{outside_map, Cell, Gesture, Tool};
use super::Editor;
use crate::app::theme;
use eframe::egui::{self, Color32, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Ui};
use sml2_core::level::{LAYOUT_HEIGHT, LAYOUT_WIDTH};
use std::collections::HashMap;

impl Editor {
    pub(super) fn canvas(&mut self, ui: &mut Ui) {
        let size = egui::vec2(ui.available_width(), ui.available_height().max(1.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        self.canvas_rect = Some(rect);
        let Some(work) = &mut self.work else {
            return;
        };
        let (pos, delta, space, scroll, modifiers, focused) = ui.input(|i| {
            (
                i.pointer.hover_pos(),
                i.pointer.delta(),
                i.key_down(egui::Key::Space),
                i.raw_scroll_delta,
                i.modifiers,
                i.focused,
            )
        });
        let inside = pos.is_some_and(|p| rect.contains(p)) && response.contains_pointer();
        if !focused {
            work.cancel();
            self.gesture_button = None;
            self.pan_button = None;
        }
        if inside && self.gesture_button.is_none() {
            if self.pan_button.is_none() {
                self.pan_button = ui.input(|i| {
                    if i.pointer.button_pressed(PointerButton::Middle) {
                        Some(PointerButton::Middle)
                    } else if space && i.pointer.primary_pressed() {
                        Some(PointerButton::Primary)
                    } else {
                        None
                    }
                });
            }
            if scroll != egui::Vec2::ZERO && self.pan_button.is_none() {
                if modifiers.command || modifiers.ctrl {
                    let next = (i16::from(work.viewport.zoom) + if scroll.y > 0.0 { 1 } else { -1 })
                        .clamp(1, 8) as u8;
                    work.viewport.zoom_at(rect, pos.unwrap(), next);
                } else {
                    let delta = if modifiers.shift {
                        egui::vec2(scroll.x + scroll.y, 0.0)
                    } else {
                        scroll
                    };
                    work.viewport.pan(delta);
                }
            }
        }
        if let Some(button) = self.pan_button {
            work.viewport.pan(delta);
            if !ui.input(|i| i.pointer.button_down(button)) {
                self.pan_button = None;
            }
        }
        self.hover = if inside {
            pos.and_then(|p| work.viewport.cell(rect, p))
        } else {
            None
        };
        let button = if inside && self.pan_button.is_none() && !space {
            ui.input(|i| {
                if i.pointer.secondary_pressed() {
                    Some(PointerButton::Secondary)
                } else if i.pointer.primary_pressed() {
                    Some(PointerButton::Primary)
                } else {
                    None
                }
            })
        } else {
            None
        };
        // Pick what is actually shown, including DX's collision-compatible decoration.
        let picked = if work.tool == Tool::Eyedropper {
            self.hover.and_then(|cell| {
                if work.paint(cell).is_some() || outside_map(cell) {
                    work.sample(cell)
                } else {
                    self.atlas
                        .as_ref()?
                        .layout
                        .get(cell.1 as usize * LAYOUT_WIDTH + cell.0 as usize)
                        .copied()
                }
            })
        } else {
            None
        };
        if let (Some(button), Some(cell)) = (button, self.hover) {
            if work.tool == Tool::Eyedropper && button == PointerButton::Primary {
                if let Some(id) = picked.filter(|&id| usize::from(id) < work.count) {
                    work.metatile = id;
                }
            } else {
                work.begin(cell, button == PointerButton::Secondary);
                if work.gesture.is_some() {
                    self.gesture_button = Some(button);
                }
            }
        }
        if let Some(button) = self.gesture_button {
            work.extend(self.hover);
            if !ui.input(|i| i.pointer.button_down(button)) {
                work.finish();
                self.gesture_button = None;
            }
        }
        response.on_hover_cursor(if space || self.pan_button.is_some() {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::Crosshair
        });
        self.paint_canvas(ui, rect);
    }

    fn paint_canvas(&self, ui: &Ui, rect: Rect) {
        let Some(work) = &self.work else {
            return;
        };
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(42, 45, 56));
        let view = work.viewport;
        let cell_rect = |cell: Cell| {
            Rect::from_min_size(
                view.screen(
                    rect,
                    egui::pos2(f32::from(cell.0) * 16.0, f32::from(cell.1) * 16.0),
                ),
                egui::vec2(16.0, 16.0) * f32::from(view.zoom),
            )
        };
        let lo = view.world(rect, rect.min);
        let hi = view.world(rect, rect.max);
        let bounds = (
            ((lo.x / 16.0).floor() as i32).max(i16::MIN as i32),
            ((lo.y / 16.0).floor() as i32).max(i16::MIN as i32),
            ((hi.x / 16.0).floor() as i32).min(i16::MAX as i32),
            ((hi.y / 16.0).floor() as i32).min(i16::MAX as i32),
        );
        let paints: HashMap<_, _> = work
            .scenery
            .paints
            .iter()
            .map(|p| ((p.x, p.y), p.metatile))
            .collect();
        let draw_tile = |id, at, tint| {
            if let Some(atlas) = &self.atlas {
                if usize::from(id) < work.count {
                    painter.image(atlas.texture.id(), at, atlas.uv(id), tint);
                }
            }
        };
        for y in bounds.1..=bounds.3 {
            for x in bounds.0..=bounds.2 {
                let cell = (x as i16, y as i16);
                let at = cell_rect(cell);
                let id = paints.get(&cell).copied().or_else(|| {
                    if outside_map(cell) {
                        None
                    } else {
                        self.atlas
                            .as_ref()
                            .and_then(|a| a.layout.get(y as usize * LAYOUT_WIDTH + x as usize))
                            .copied()
                    }
                });
                if let Some(id) = id {
                    draw_tile(id, at, Color32::WHITE);
                } else if (x + y).rem_euclid(2) == 0 {
                    painter.rect_filled(at, 0.0, Color32::from_white_alpha(8));
                }
            }
        }
        let level_rect = Rect::from_min_max(
            view.screen(rect, Pos2::ZERO),
            view.screen(
                rect,
                egui::pos2((LAYOUT_WIDTH * 16) as f32, (LAYOUT_HEIGHT * 16) as f32),
            ),
        );
        painter.rect_filled(
            level_rect,
            0.0,
            Color32::from_rgba_unmultiplied(42, 45, 138, 12),
        );
        if work.grid {
            let stroke = Stroke::new(1.0_f32, Color32::from_black_alpha(55));
            for x in bounds.0..=bounds.2 + 1 {
                let sx = view.screen(rect, egui::pos2(x as f32 * 16.0, 0.0)).x;
                painter.line_segment(
                    [egui::pos2(sx, rect.top()), egui::pos2(sx, rect.bottom())],
                    stroke,
                );
            }
            for y in bounds.1..=bounds.3 + 1 {
                let sy = view.screen(rect, egui::pos2(0.0, y as f32 * 16.0)).y;
                painter.line_segment(
                    [egui::pos2(rect.left(), sy), egui::pos2(rect.right(), sy)],
                    stroke,
                );
            }
        }
        painter.rect_stroke(
            level_rect,
            0.0,
            Stroke::new(2.0_f32, theme::MAGENTA),
            StrokeKind::Inside,
        );
        if level_rect.left() > rect.left() + 140.0 {
            painter.text(
                rect.left_top() + egui::vec2(12.0, 12.0),
                egui::Align2::LEFT_TOP,
                "SCENERY · paint here",
                egui::FontId::proportional(12.0),
                theme::SHELL_LIGHT,
            );
        }
        if let Some(Gesture::Rectangle { start, end, value }) = &work.gesture {
            for y in start.1.min(end.1)..=start.1.max(end.1) {
                for x in start.0.min(end.0)..=start.0.max(end.0) {
                    draw_tile(*value, cell_rect((x, y)), Color32::from_white_alpha(140));
                }
            }
            let outline = cell_rect((start.0.min(end.0), start.1.min(end.1)))
                .union(cell_rect((start.0.max(end.0), start.1.max(end.1))));
            painter.rect_stroke(
                outline,
                0.0,
                Stroke::new(2.0_f32, theme::GOLD),
                StrokeKind::Inside,
            );
        } else if self.pan_button.is_none() {
            if let Some(cell) = self.hover {
                let at = cell_rect(cell);
                let erasing = work.tool == Tool::Eraser
                    || self.gesture_button == Some(PointerButton::Secondary);
                let color = if erasing {
                    Color32::LIGHT_RED
                } else {
                    theme::GOLD
                };
                if !erasing && work.tool != Tool::Eyedropper {
                    draw_tile(work.metatile, at, Color32::from_white_alpha(140));
                }
                if erasing {
                    painter.rect_filled(at, 0.0, color.gamma_multiply(0.2));
                }
                painter.rect_stroke(at, 0.0, Stroke::new(2.0_f32, color), StrokeKind::Inside);
            }
        }
    }
}
