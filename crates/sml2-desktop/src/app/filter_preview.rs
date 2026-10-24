//! The Display page's filter preview: one opening-level frame with Mario, an
//! enemy and the HUD, so BG and both OBJ palettes all show, recolored per
//! filter with a plain image and caption.

use crate::settings::Filter;
use eframe::egui::{self, TextureHandle, TextureOptions, Ui};
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
            let image = super::colorized(&self.pixels, filter, &self.header, (160, 144));
            self.texture = Some((
                filter,
                ctx.load_texture("sml2-filter-preview", image, TextureOptions::NEAREST),
            ));
        }
        &self.texture.as_ref().expect("texture just set").1
    }

    /// The sample at 2×, without a decorative console frame.
    pub(super) fn show(&mut self, ui: &mut Ui, filter: Filter, caption: &str) {
        let id = self.texture(ui.ctx(), filter).id();
        ui.vertical_centered(|ui| {
            ui.image((id, egui::vec2(320.0, 288.0)));
            ui.weak(caption);
        });
    }
}
