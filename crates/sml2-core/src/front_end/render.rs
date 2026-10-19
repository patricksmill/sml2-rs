//! Frame composition for the front-end screens: background plus OAM sprites.
//!
//! The file select's background resolves tiles through the signed `$8800`
//! method (measured: its letters live at `$9000+tile*16`, while `$8000` renders
//! the Mario tiles). Sprites use the `$5297` metasprite expander: the cursor,
//! the Easy banner and the erase animation are entries in the player mapping
//! table (`ObjectTables::player_mappings`), while the tally digits are the
//! `$5267` two-cell row.

use super::assets::FrontEndAssets;
use super::file_select::{FileSelect, FILE_SELECT_BGP, FILE_SELECT_OBP0, FILE_SELECT_OBP1};
use super::save_slots::{SaveSlots, SLOT_COUNT};
use super::screen::{render_bg, render_bg_mode, TileData};
use super::title::BGP as TITLE_BGP;
use crate::actor::ScreenPosition;
use crate::object::ObjectTables;
use crate::player::{append_cells, Cell, CellStyle};
use crate::video::{bgp_shades, tile_colors};

/// `$300FE`: tally sprites' shared OAM Y.
const TALLY_Y: u8 = 88;
/// `$300FE`: first slot's OAM X, +32 per slot.
const TALLY_X0: u8 = 32;
/// `$3005F`: the Easy banner's OAM base and metasprite index.
const EASY_X: u8 = 68;
const EASY_Y: u8 = 48;
const EASY_FRAME: u8 = 241;

/// `$1407C`'s palette and `$59E5`'s map.
pub fn render_title(assets: &FrontEndAssets, view: (usize, usize)) -> Vec<u8> {
    render_bg(
        assets.title_vram(),
        assets.title_tilemap(),
        TITLE_BGP,
        (0, 0),
        view,
    )
    .unwrap_or_else(|_| vec![0u8; view.0 * view.1])
}

/// `$30000`'s static screen plus the cursor, tally digits and Easy banner.
pub fn render_file_select(
    fs: &FileSelect,
    slots: &SaveSlots,
    assets: &FrontEndAssets,
    tables: &ObjectTables,
    view: (usize, usize),
) -> Vec<u8> {
    let (vw, vh) = view;
    let mut out = render_bg_mode(
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        (0, 0),
        view,
        TileData::Signed,
    )
    .unwrap_or_else(|_| vec![0u8; vw * vh]);
    let vram = assets.file_select_vram();

    // `$300FE`: high nibble then low nibble, tile + $80, X +8 between digits.
    for slot in 0..SLOT_COUNT {
        let tally = slots.slot(slot).map(|r| r[0x42]).unwrap_or(0);
        let x = TALLY_X0 + 32 * slot as u8;
        draw_cell(
            &mut out,
            vw,
            vh,
            vram,
            Cell {
                y: TALLY_Y,
                x,
                tile: 0x80 | (tally >> 4),
                attr: 0,
            },
        );
        draw_cell(
            &mut out,
            vw,
            vh,
            vram,
            Cell {
                y: TALLY_Y,
                x: x + 8,
                tile: 0x80 | (tally & 0x0F),
                attr: 0,
            },
        );
    }

    if fs.easy_mode() {
        draw_metasprite(
            &mut out,
            vw,
            vh,
            vram,
            tables,
            EASY_FRAME,
            (EASY_X, EASY_Y),
            false,
        );
    }

    if let Some((y, frame)) = fs.erase_hop() {
        draw_metasprite(
            &mut out,
            vw,
            vh,
            vram,
            tables,
            frame,
            (fs.cursor().0, y),
            true,
        );
    }

    let (x, y) = fs.cursor();
    // `$30064`: Easy mode draws the small-Mario frames, 32 ids later.
    let frame = if fs.easy_mode() {
        fs.cursor_sprite().wrapping_add(32)
    } else {
        fs.cursor_sprite()
    };
    draw_metasprite(
        &mut out,
        vw,
        vh,
        vram,
        tables,
        frame,
        (x, y),
        fs.cursor_behind_bg(),
    );
    out
}

/// Expand one `$5297` metasprite and draw its cells.
fn draw_metasprite(
    frame: &mut [u8],
    vw: usize,
    vh: usize,
    vram: &[u8],
    tables: &ObjectTables,
    sprite: u8,
    base: (u8, u8),
    behind_bg: bool,
) {
    let mut cells = Vec::new();
    let Some(()) = append_cells(
        tables,
        sprite,
        ScreenPosition {
            x: base.0,
            y: base.1,
        },
        CellStyle {
            force_palette_one: false,
            behind_background: behind_bg,
        },
        &mut cells,
    ) else {
        return;
    };
    for cell in cells {
        draw_cell(frame, vw, vh, vram, cell);
    }
}

/// Draw one 8x8 OAM cell. Colour 0 is transparent; attr bit 4 selects OBP1,
/// bit 5/6 flip X/Y, and bit 7 puts the cell behind background colour 0.
fn draw_cell(frame: &mut [u8], vw: usize, vh: usize, vram: &[u8], cell: Cell) {
    if cell.tile as usize * 16 + 16 > vram.len() {
        return;
    }
    let tile = &vram[cell.tile as usize * 16..cell.tile as usize * 16 + 16];
    let Ok(bytes) = <&[u8; 16]>::try_from(tile) else {
        return;
    };
    let colors = tile_colors(bytes);
    let obp = if cell.attr & 0x10 != 0 {
        FILE_SELECT_OBP1
    } else {
        FILE_SELECT_OBP0
    };
    let shades = bgp_shades(obp);
    let flip_x = cell.attr & 0x20 != 0;
    let flip_y = cell.attr & 0x40 != 0;
    let behind_bg = cell.attr & 0x80 != 0;
    let sx = cell.x as i16 - 8;
    let sy = cell.y as i16 - 16;
    for row in 0..8usize {
        for col in 0..8usize {
            let src_col = if flip_x { 7 - col } else { col };
            let src_row = if flip_y { 7 - row } else { row };
            let color = colors[src_row * 8 + src_col];
            if color == 0 {
                continue;
            }
            let px = sx + col as i16;
            let py = sy + row as i16;
            if px < 0 || py < 0 || px as usize >= vw || py as usize >= vh {
                continue;
            }
            let i = py as usize * vw + px as usize;
            if behind_bg && frame[i] != 0xFF {
                continue;
            }
            frame[i] = shades[color as usize];
        }
    }
}

#[cfg(test)]
mod tests;
