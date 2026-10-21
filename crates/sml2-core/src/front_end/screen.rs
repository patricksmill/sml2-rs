//! Static background rasterizer for the front-end screens.
//!
//! The title and file select both use the `$8000` tile-data mode (LCDC bit 4 =
//! 0): a tile number `t` reads `vram[t*16..]`, and the 32x32 map at `$9800`
//! covers a 256x256-pixel plane that wraps. No window, no sprites here — the
//! file select's OAM pass is added by its own renderer.

use crate::video::{tile_colors, OutputMode, PaletteSource};

/// The `$8000-$97FF` background window.
pub const VRAM_LEN: usize = 0x1800;
/// The 32x32 background map.
pub const TILEMAP_LEN: usize = 0x400;

/// LCDC bit 4: which tile-data base the background uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileData {
    /// `$8000 + tile * 16` (LCDC bit 4 = 0).
    Unsigned,
    /// `$9000 + tile * 16` for tile 0..127, `$8000 + tile * 16` above
    /// (LCDC bit 4 = 1, the signed `$8800` method).
    Signed,
}

/// Render a `view`-sized frame from the background map.
#[allow(clippy::too_many_arguments)]
pub fn render_bg(
    vram: &[u8],
    tilemap: &[u8],
    bgp: u8,
    scroll: (u8, u8),
    view: (usize, usize),
) -> Result<Vec<u8>, String> {
    render_bg_mode(vram, tilemap, bgp, scroll, view, TileData::Unsigned)
}

/// Render with an explicit tile-data base.
#[allow(clippy::too_many_arguments)]
pub fn render_bg_mode(
    vram: &[u8],
    tilemap: &[u8],
    bgp: u8,
    scroll: (u8, u8),
    view: (usize, usize),
    tile_data: TileData,
) -> Result<Vec<u8>, String> {
    render_bg_output(
        vram,
        tilemap,
        bgp,
        scroll,
        view,
        tile_data,
        OutputMode::Grayscale,
    )
}

/// Render background pixels in the requested output representation.
#[allow(clippy::too_many_arguments)]
pub fn render_bg_output(
    vram: &[u8],
    tilemap: &[u8],
    bgp: u8,
    scroll: (u8, u8),
    view: (usize, usize),
    tile_data: TileData,
    output: OutputMode,
) -> Result<Vec<u8>, String> {
    if vram.len() < VRAM_LEN {
        return Err(format!(
            "background VRAM is {} bytes, expected {VRAM_LEN}",
            vram.len()
        ));
    }
    if tilemap.len() < TILEMAP_LEN {
        return Err(format!(
            "background map is {} bytes, expected {TILEMAP_LEN}",
            tilemap.len()
        ));
    }
    let (vw, vh) = view;
    let shades = output.palette(bgp, PaletteSource::Bg);
    let (scroll_x, scroll_y) = (scroll.0 as usize, scroll.1 as usize);
    let mut out = vec![shades[0]; vw * vh];
    for y in 0..vh {
        let py = (y + scroll_y) & 0xFF;
        let row = (py >> 3) & 31;
        let line = py & 7;
        for x in 0..vw {
            let px = (x + scroll_x) & 0xFF;
            let col = (px >> 3) & 31;
            let tile = tilemap[row * 32 + col];
            let offset = match tile_data {
                TileData::Unsigned => tile as usize * 16,
                TileData::Signed if tile < 0x80 => 0x1000 + tile as usize * 16,
                TileData::Signed => tile as usize * 16,
            };
            let bytes: &[u8; 16] = vram[offset..offset + 16]
                .try_into()
                .expect("16-byte tile slice");
            let colors = tile_colors(bytes);
            out[y * vw + x] = shades[colors[line * 8 + (px & 7)] as usize];
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
