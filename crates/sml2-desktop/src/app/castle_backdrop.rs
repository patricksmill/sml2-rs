//! Fill spare window space with the castle's own wall and ground, keeping the
//! native scene and its animation at their existing scale and anchor.
use super::DesktopApp;
use crate::settings::Filter;
use eframe::egui::{self, Color32, Rect, TextureHandle, TextureOptions};
use sml2_runtime::{render::motion::Smooth, session::SessionState};

#[derive(PartialEq, Eq)]
struct Key {
    view: (usize, usize),
    filter: Filter,
    scene: u8,
    bgp: u8,
    dx: bool,
    installation: Option<sml2_dx::cache::CacheKey>,
}

pub(super) struct CastleBackdrop {
    key: Key,
    texture: TextureHandle,
}

impl DesktopApp {
    pub(super) fn paint_castle_backdrop(&mut self, ui: &egui::Ui, picture: Rect, available: Rect) {
        let scene = self
            .game
            .as_ref()
            .and_then(|game| match &game.session.state {
                SessionState::Castle(seq)
                    if !matches!(
                        seq.state,
                        sml2_core::castle_sequence::STATE_CONGRATS
                            | sml2_core::castle_sequence::STATE_WAIT_START
                    ) =>
                {
                    Some((1, seq.bgp))
                }
                SessionState::GameOver(_) => Some((0, 0xE4)),
                _ => None,
            });
        let Some((scene, bgp)) = scene else {
            self.castle_backdrop = None;
            return;
        };
        let scale = self.effective_scale;
        if scale == 0 {
            return;
        }
        let ppp = ui.ctx().pixels_per_point();
        let view = self.settings.view();
        // Symmetric margins preserve the native anchor. Round outward to cover
        // odd window sizes too; the painter clips the surplus at the edges.
        let extra = |axis: f32, length: usize| {
            ((axis * ppp).ceil() as u32)
                .saturating_sub(length as u32 * scale)
                .div_ceil(2)
                .div_ceil(scale) as usize
        };
        let (extra_x, extra_y) = (
            extra(available.width(), view.0),
            extra(available.height(), view.1),
        );
        if extra_x == 0 && extra_y == 0 {
            self.castle_backdrop = None;
            return;
        }
        let expanded = (view.0 + 2 * extra_x, view.1 + 2 * extra_y);
        let key = Key {
            view: expanded,
            filter: self.settings.filter,
            scene,
            bgp,
            dx: self.settings.dx_mode && self.dx_bundle.is_some(),
            installation: self.settings.dx_installation.clone(),
        };
        if self
            .castle_backdrop
            .as_ref()
            .is_none_or(|backdrop| backdrop.key != key)
        {
            let Some(image) = self.frame_image_with_view(expanded, Smooth::OFF) else {
                return;
            };
            if let Some(backdrop) = &mut self.castle_backdrop {
                backdrop.texture.set(image, TextureOptions::NEAREST);
                backdrop.key = key;
            } else {
                self.castle_backdrop = Some(CastleBackdrop {
                    key,
                    texture: ui.ctx().load_texture(
                        "sml2-castle-backdrop",
                        image,
                        TextureOptions::NEAREST,
                    ),
                });
            }
        }
        let backdrop = self.castle_backdrop.as_ref().unwrap();
        let destination = Rect::from_min_size(
            picture.min - egui::vec2(extra_x as f32, extra_y as f32) * scale as f32 / ppp,
            egui::vec2(expanded.0 as f32, expanded.1 as f32) * scale as f32 / ppp,
        );
        ui.painter().with_clip_rect(available).image(
            backdrop.texture.id(),
            destination,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

#[cfg(test)]
mod tests;
