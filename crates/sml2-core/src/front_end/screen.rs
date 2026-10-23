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

/// What an off-map tile `(tx, ty)` of a 20×18 front-end screen shows: the map
/// cell `(col, row)` to copy and which of its lines to read for pixel line
/// `line`, given the screen's map.
pub type Backdrop = fn(&[u8], i32, i32, usize) -> ((usize, usize), usize);

/// The title sign stands on a `$85`/`$86` wall checker (map `(0, 0)` is
/// `$85`) above row 17's `$95` ground, whose last line runs on below.
pub fn title_backdrop(_map: &[u8], tx: i32, ty: i32, line: usize) -> ((usize, usize), usize) {
    match ty {
        ..17 => (((tx + ty).rem_euclid(2) as usize, 0), line),
        17 => ((0, 17), line),
        _ => ((0, 17), 7),
    }
}

/// The file select is framed by a 2×2 block checker (map rows 0-1, columns
/// 0-1); a column whose top or bottom tile breaks it — a pipe — runs on.
pub fn file_select_backdrop(map: &[u8], tx: i32, ty: i32, line: usize) -> ((usize, usize), usize) {
    let block = (tx.rem_euclid(2) as usize, ty.rem_euclid(2) as usize);
    if !(0..20).contains(&tx) {
        return (block, line);
    }
    let (col, edge) = (tx as usize, if ty < 0 { 0 } else { 17 });
    if map[edge * 32 + col] != map[edge * 32 + col % 2] {
        ((col, edge), line)
    } else {
        (block, line)
    }
}

/// `native`, a 160×144 frame of the screen in `tilemap`, at `at` in a
/// `view`-sized frame whose other pixels come from `backdrop`.
#[allow(clippy::too_many_arguments)]
pub fn render_bg_wide_output(
    native: &[u8],
    vram: &[u8],
    tilemap: &[u8],
    bgp: u8,
    tile_data: TileData,
    view: (usize, usize),
    at: (i32, i32),
    output: OutputMode,
    backdrop: Backdrop,
) -> Vec<u8> {
    let shades = output.palette(bgp, PaletteSource::Bg);
    let tiles: Vec<[u8; 64]> = (0..=255u8)
        .map(|tile| {
            let offset = match tile_data {
                TileData::Unsigned => tile as usize * 16,
                TileData::Signed if tile < 0x80 => 0x1000 + tile as usize * 16,
                TileData::Signed => tile as usize * 16,
            };
            vram.get(offset..offset + 16)
                .and_then(|b| b.try_into().ok())
                .map_or([0; 64], tile_colors)
        })
        .collect();
    let mut out = Vec::with_capacity(view.0 * view.1);
    for y in 0..view.1 as i32 {
        let ny = y - at.1;
        for x in 0..view.0 as i32 {
            let nx = x - at.0;
            if (0..160).contains(&nx) && (0..144).contains(&ny) {
                out.push(native[(ny * 160 + nx) as usize]);
                continue;
            }
            let ((col, row), line) = backdrop(
                tilemap,
                nx.div_euclid(8),
                ny.div_euclid(8),
                ny.rem_euclid(8) as usize,
            );
            let tile = tiles[tilemap[row * 32 + col] as usize];
            out.push(shades[tile[line * 8 + nx.rem_euclid(8) as usize] as usize]);
        }
    }
    out
}

#[cfg(test)]
mod tests;
