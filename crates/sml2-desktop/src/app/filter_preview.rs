//! The Display page's filter preview: one opening-level frame with Mario, an
//! enemy and the HUD, so BG and both OBJ palettes all show, recolored per
//! filter inside a DMG-style screen bezel.

use super::theme;
use crate::settings::Filter;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Rect, Sense, TextureHandle, TextureOptions, Ui,
    Vec2,
};
use sml2_core::video::PalettePixel;
use sml2_runtime::{content::SessionAssets, FrameInput, Session};

/// Ticks into the opening level: past the fade-in, with an enemy on screen.
const SAMPLE_TICKS: u32 = 120;

pub(super) struct FilterPreview {
    pixels: Vec<PalettePixel>,
    header: [u8; 0x18],
    texture: Option<(Filter, TextureHandle)>,
}

impl FilterPreview {
    /// Run a scratch session to the sample frame. The assets come back
    /// unchanged for the real game; sessions never mutate them.
    pub(super) fn new(assets: SessionAssets) -> (SessionAssets, Result<Self, String>) {
        let header = assets.cartridge_header;
        let mut session = Session::new(assets);
        let pixels = session.debug_warp_level(0).map(|()| {
            for _ in 0..SAMPLE_TICKS {
                session.advance(FrameInput::default());
            }
            session.render_palette_frame((160, 144))
        });
        let preview = pixels.map(|pixels| Self {
            pixels,
            header,
            texture: None,
        });
        (session.assets, preview)
    }

    fn texture(&mut self, ctx: &egui::Context, filter: Filter) -> &TextureHandle {
        if self.texture.as_ref().is_none_or(|(f, _)| *f != filter) {
            let image = super::colorized(&self.pixels, filter, &self.header);
            self.texture = Some((
                filter,
                ctx.load_texture("sml2-filter-preview", image, TextureOptions::NEAREST),
            ));
        }
        &self.texture.as_ref().expect("texture just set").1
    }

    /// The sample at 2× inside a slate bezel with the power LED.
    pub(super) fn show(&mut self, ui: &mut Ui, filter: Filter, caption: &str) {
        let screen = Vec2::new(320.0, 288.0);
        let size = screen + Vec2::new(64.0, 58.0);
        let (outer, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(
            outer,
            CornerRadius {
                nw: 8,
                ne: 8,
                sw: 8,
                se: 36,
            },
            theme::BEZEL,
        );
        // The two stripes across the top of the DMG lens.
        for (offset, color) in [(10.0, theme::MAGENTA_DARK), (14.0, theme::NAVY)] {
            let y = outer.min.y + offset;
            painter.line_segment(
                [
                    egui::pos2(outer.min.x + 12.0, y),
                    egui::pos2(outer.max.x - 12.0, y),
                ],
                egui::Stroke::new(2.0_f32, color),
            );
        }
        let image = Rect::from_min_size(
            egui::pos2(outer.center().x - screen.x / 2.0 + 8.0, outer.min.y + 26.0),
            screen,
        );
        let led = egui::pos2(outer.min.x + 18.0, image.min.y + 60.0);
        painter.circle_filled(led, 4.0, theme::LED);
        painter.text(
            led + Vec2::new(0.0, 12.0),
            Align2::CENTER_TOP,
            "POWER",
            FontId::proportional(8.0),
            theme::SHELL,
        );
        let id = self.texture(ui.ctx(), filter).id();
        let painter = ui.painter();
        painter.rect_filled(image.expand(2.0), 2, Color32::BLACK);
        painter.image(
            id,
            image,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        painter.text(
            egui::pos2(image.center().x, image.max.y + 15.0),
            Align2::CENTER_CENTER,
            caption,
            FontId::proportional(12.0),
            theme::SHELL,
        );
    }
}
