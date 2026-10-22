//! The launcher's scrolling backdrop: level 1's surface rendered once through
//! the game's own BG composer, then panned across a row of texture chunks
//! (each well under any GPU's texture limit) that wraps at the layout's end.

use crate::settings::Filter;
use eframe::egui::{self, Color32, ColorImage, Context, Rect, TextureHandle, TextureOptions, Ui};
use sml2_core::level::LAYOUT_WIDTH;
use sml2_core::video::{OutputMode, PalettePixel};
use sml2_runtime::content::SessionAssets;

/// Game Boy pixels per second.
const SPEED: f64 = 24.0;
/// The whole 256-px BG window: the start view plus the sky above it.
const HEIGHT: usize = 256;
const CHUNK: usize = 1024;

pub(super) struct Backdrop {
    width: usize,
    pixels: Vec<PalettePixel>,
    header: [u8; 0x18],
    textures: Option<(Filter, Vec<TextureHandle>)>,
}

impl Backdrop {
    /// Stitch 160-px views of level 1 (`$00`) across its whole layout, without
    /// objects or the HUD. The bottom edge matches the start view's.
    pub(super) fn new(assets: &SessionAssets) -> Result<Self, String> {
        let content = assets.level(0)?;
        let manifest = &content.manifest;
        let width = LAYOUT_WIDTH * 16;
        let cam_y = manifest.header.camera_start_y - (HEIGHT as u16 - 144) / 2;
        let mut pixels = vec![PalettePixel::from_token(0); width * HEIGHT];
        for left in (0..width).step_by(160) {
            let view = sml2_runtime::render::render_view_output(
                manifest,
                &content.layout,
                &[],
                &content.bg_base,
                (left + 80) as u16,
                cam_y,
                160,
                HEIGHT,
                None,
                None,
                None,
                OutputMode::PaletteIndices,
            )?;
            for (y, row) in view.as_chunks::<160>().0.iter().enumerate() {
                let columns = (width - left).min(160);
                for (x, &token) in row[..columns].iter().enumerate() {
                    pixels[y * width + left + x] = PalettePixel::from_token(token);
                }
            }
        }
        Ok(Self {
            width,
            pixels,
            header: assets.cartridge_header,
            textures: None,
        })
    }

    fn textures(&mut self, ctx: &Context, filter: Filter) -> &[TextureHandle] {
        if self.textures.as_ref().is_none_or(|(f, _)| *f != filter) {
            let palette = filter.palette(&self.header);
            let chunks = (0..self.width)
                .step_by(CHUNK)
                .map(|left| {
                    let columns = (self.width - left).min(CHUNK);
                    let pixels = (0..HEIGHT)
                        .flat_map(|y| &self.pixels[y * self.width + left..][..columns])
                        .map(|&p| {
                            let [r, g, b] = palette.rgb(p);
                            Color32::from_rgb(r, g, b)
                        })
                        .collect();
                    let image = ColorImage::new([columns, HEIGHT], pixels);
                    ctx.load_texture("sml2-backdrop", image, TextureOptions::NEAREST)
                })
                .collect();
            self.textures = Some((filter, chunks));
        }
        &self.textures.as_ref().expect("textures just built").1
    }

    /// Fill `rect` with the level at a whole-number scale that covers it,
    /// panning right over time and wrapping at the layout's end.
    pub(super) fn paint(&mut self, ui: &Ui, rect: Rect, filter: Filter, dim: u8) {
        let ppp = ui.ctx().pixels_per_point();
        let scale = (rect.height() * ppp / HEIGHT as f32).ceil().max(1.0) / ppp;
        let time = ui.ctx().input(|i| i.time);
        let offset = (time * SPEED).rem_euclid(self.width as f64) as f32;
        // Snap the pan to whole screen pixels so the art never shimmers.
        let offset = (offset * scale * ppp).floor() / (scale * ppp);
        let visible = egui::vec2(rect.width() / scale, rect.height() / scale);
        let top = (HEIGHT as f32 - visible.y).max(0.0);
        let (v0, v1) = (top / HEIGHT as f32, 1.0);
        let width = self.width;
        let textures = self.textures(ui.ctx(), filter).to_vec();
        // Walk the visible world columns chunk by chunk, wrapping at the end.
        let mut x = offset;
        let end = offset + visible.x;
        while x < end {
            let wrapped = x.rem_euclid(width as f32);
            let index = (wrapped as usize / CHUNK).min(textures.len() - 1);
            let chunk_left = (index * CHUNK) as f32;
            let chunk_width = (width - index * CHUNK).min(CHUNK) as f32;
            let piece = (chunk_left + chunk_width - wrapped).min(end - x);
            let screen = Rect::from_min_size(
                egui::pos2(rect.min.x + (x - offset) * scale, rect.min.y),
                egui::vec2(piece * scale, rect.height()),
            );
            let u0 = (wrapped - chunk_left) / chunk_width;
            let uv =
                Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u0 + piece / chunk_width, v1));
            ui.painter()
                .image(textures[index].id(), screen, uv, Color32::WHITE);
            x += piece;
        }
        if dim > 0 {
            ui.painter()
                .rect_filled(rect, 0, Color32::from_black_alpha(dim));
        }
    }
}
