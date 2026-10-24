//! Presentation helpers shared by the game and optional authoring app.

use crate::settings::Filter;
use eframe::egui::{self, Color32, ColorImage};
use sml2_core::video::PalettePixel;

pub mod screenshot;
pub mod theme;

/// Keep the winning BG/OBJ source until its mapped shade reaches presentation.
pub fn colorized(
    frame: &[PalettePixel],
    filter: Filter,
    header: &[u8; 0x18],
    view: (usize, usize),
) -> ColorImage {
    let palette = filter.palette(header);
    let pixels = frame
        .iter()
        .map(|&pixel| {
            let [r, g, b] = palette.rgb(pixel);
            Color32::from_rgb(r, g, b)
        })
        .collect();
    ColorImage::new([view.0, view.1], pixels)
}

/// Release keyboard focus before handling a host shortcut.
pub fn release_focus(ctx: &egui::Context) {
    ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
}
