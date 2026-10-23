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
use super::screen::{
    file_select_backdrop, render_bg_output, render_bg_wide_output, title_backdrop, TileData,
};
use super::title::BGP as TITLE_BGP;
use crate::actor::ScreenPosition;
use crate::object::ObjectTables;
use crate::player::{append_cells, Cell, CellStyle};
use crate::video::{tile_colors, OutputMode, PaletteSource};

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
    render_title_output(assets, view, OutputMode::Grayscale)
}

pub fn render_title_output(
    assets: &FrontEndAssets,
    view: (usize, usize),
    output: OutputMode,
) -> Vec<u8> {
    // The title sign also resolves tile 0..127 through the signed `$8800`
    // method: the logo's upper tiles live at `$9000+tile*16`. The unsigned
    // lookup renders a shifted, cropped logo.
    render_bg_output(
        assets.title_vram(),
        assets.title_tilemap(),
        TITLE_BGP,
        (0, 0),
        view,
        TileData::Signed,
        output,
    )
    .unwrap_or_else(|_| vec![output.blank(3); view.0 * view.1])
}

/// The title in a view larger than native, its 160×144 rect at `at` and
/// [`title_backdrop`] past the map's 20×18 tiles.
pub fn render_title_wide_output(
    assets: &FrontEndAssets,
    view: (usize, usize),
    at: (i32, i32),
    output: OutputMode,
) -> Vec<u8> {
    render_bg_wide_output(
        &render_title_output(assets, (160, 144), output),
        assets.title_vram(),
        assets.title_tilemap(),
        TITLE_BGP,
        TileData::Signed,
        view,
        at,
        output,
        title_backdrop,
    )
}

/// The file select in a view larger than native, as
/// [`render_title_wide_output`] with [`file_select_backdrop`].
#[allow(clippy::too_many_arguments)]
pub fn render_file_select_wide_output(
    fs: &FileSelect,
    slots: &SaveSlots,
    assets: &FrontEndAssets,
    tables: &ObjectTables,
    view: (usize, usize),
    at: (i32, i32),
    output: OutputMode,
) -> Vec<u8> {
    render_bg_wide_output(
        &render_file_select_output(fs, slots, assets, tables, (160, 144), output),
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        TileData::Signed,
        view,
        at,
        output,
        file_select_backdrop,
    )
}

/// `$30000`'s static screen plus the cursor, tally digits and Easy banner.
pub fn render_file_select(
    fs: &FileSelect,
    slots: &SaveSlots,
    assets: &FrontEndAssets,
    tables: &ObjectTables,
    view: (usize, usize),
) -> Vec<u8> {
    render_file_select_output(fs, slots, assets, tables, view, OutputMode::Grayscale)
}

pub fn render_file_select_output(
    fs: &FileSelect,
    slots: &SaveSlots,
    assets: &FrontEndAssets,
    tables: &ObjectTables,
    view: (usize, usize),
    output: OutputMode,
) -> Vec<u8> {
    let (vw, vh) = view;
    let mut out = render_bg_output(
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        (0, 0),
        view,
        TileData::Signed,
        output,
    )
    .unwrap_or_else(|_| vec![output.blank(3); vw * vh]);
    for cell in file_select_cells(fs, slots, tables) {
        draw_cell(&mut out, vw, vh, assets.file_select_vram(), output, cell);
    }
    out
}

fn file_select_cells(fs: &FileSelect, slots: &SaveSlots, tables: &ObjectTables) -> Vec<Cell> {
    let mut cells = Vec::new();
    for slot in 0..SLOT_COUNT {
        let tally = slots.slot(slot).map_or(0, |r| r[0x42]);
        let x = TALLY_X0 + 32 * slot as u8;
        for (dx, digit) in [(0, tally >> 4), (8, tally & 0x0F)] {
            cells.push(Cell {
                y: TALLY_Y,
                x: x + dx,
                tile: 0x80 | digit,
                attr: 0,
            });
        }
    }
    let mut sprite = |frame, (x, y), behind_background| {
        append_cells(
            tables,
            frame,
            ScreenPosition { x, y },
            CellStyle {
                force_palette_one: false,
                behind_background,
            },
            &mut cells,
        );
    };
    if fs.easy_mode() {
        sprite(EASY_FRAME, (EASY_X, EASY_Y), false);
    }
    if let Some((y, frame)) = fs.erase_hop() {
        sprite(frame, (fs.cursor().0, y), true);
    }
    let cursor = fs.cursor_presentation();
    sprite(cursor.sprite, cursor.position, cursor.behind_background);
    cells
}

/// The desktop file select opens into the hall and fills the selected view.
pub fn render_file_select_room_output(
    fs: &FileSelect,
    slots: &SaveSlots,
    assets: &FrontEndAssets,
    tables: &ObjectTables,
    view: (usize, usize),
    output: OutputMode,
) -> Vec<u8> {
    let scroll = i32::from(fs.cursor().0.saturating_sub(152));
    let at = ((view.0 as i32 - 160) / 2, (view.1 as i32 - 144) / 2);
    let mut frame = room_background(assets, view, scroll, output);
    for cell in file_select_cells(fs, slots, tables) {
        draw_cell_at(
            &mut frame,
            view.0,
            view.1,
            assets.file_select_vram(),
            output,
            cell,
            (at.0 - scroll, at.1),
        );
    }
    frame
}

fn room_background(
    assets: &FrontEndAssets,
    view: (usize, usize),
    scroll_x: i32,
    output: OutputMode,
) -> Vec<u8> {
    use super::screen::{hall_backdrop, render_backdrop_output};
    let at = ((view.0 as i32 - 160) / 2, (view.1 as i32 - 144) / 2);
    render_backdrop_output(
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        TileData::Signed,
        view,
        (scroll_x - at.0, -at.1),
        output,
        hall_backdrop,
    )
}

pub fn render_hall_output(
    assets: &FrontEndAssets,
    tables: &ObjectTables,
    scroll_x: i32,
    mario: (u8, u8),
    sprite: u8,
    output: OutputMode,
) -> Vec<u8> {
    render_hall_view_output(assets, tables, (160, 144), scroll_x, mario, sprite, output)
}

/// Hall scenery and Mario share the enlarged view's centered native anchor.
pub fn render_hall_view_output(
    assets: &FrontEndAssets,
    tables: &ObjectTables,
    view: (usize, usize),
    scroll_x: i32,
    mario: (u8, u8),
    sprite: u8,
    output: OutputMode,
) -> Vec<u8> {
    let mut frame = room_background(assets, view, scroll_x, output);
    let mut cells = Vec::new();
    append_cells(
        tables,
        sprite,
        ScreenPosition {
            x: mario.0,
            y: mario.1,
        },
        CellStyle::default(),
        &mut cells,
    );
    let at = ((view.0 as i32 - 160) / 2, (view.1 as i32 - 144) / 2);
    for cell in cells {
        draw_cell_at(
            &mut frame,
            view.0,
            view.1,
            assets.file_select_vram(),
            output,
            cell,
            at,
        );
    }
    frame
}

/// Draw one 8x8 OAM cell. Colour 0 is transparent; attr bit 4 selects OBP1,
/// bit 5/6 flip X/Y, and bit 7 puts the cell behind background colour 0.
fn draw_cell(frame: &mut [u8], vw: usize, vh: usize, vram: &[u8], output: OutputMode, cell: Cell) {
    draw_cell_at(frame, vw, vh, vram, output, cell, (0, 0));
}

#[allow(clippy::too_many_arguments)]
fn draw_cell_at(
    frame: &mut [u8],
    vw: usize,
    vh: usize,
    vram: &[u8],
    output: OutputMode,
    cell: Cell,
    at: (i32, i32),
) {
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
    let shades = output.palette(
        obp,
        if cell.attr & 0x10 != 0 {
            PaletteSource::Obj1
        } else {
            PaletteSource::Obj0
        },
    );
    let flip_x = cell.attr & 0x20 != 0;
    let flip_y = cell.attr & 0x40 != 0;
    let behind_bg = cell.attr & 0x80 != 0;
    let sx = i32::from(cell.x) - 8 + at.0;
    let sy = i32::from(cell.y) - 16 + at.1;
    for row in 0..8usize {
        for col in 0..8usize {
            let src_col = if flip_x { 7 - col } else { col };
            let src_row = if flip_y { 7 - row } else { row };
            let color = colors[src_row * 8 + src_col];
            if color == 0 {
                continue;
            }
            let px = sx + col as i32;
            let py = sy + row as i32;
            if px < 0 || py < 0 || px as usize >= vw || py as usize >= vh {
                continue;
            }
            let i = py as usize * vw + px as usize;
            if behind_bg && !output.is_white(frame[i]) {
                continue;
            }
            frame[i] = shades[color as usize];
        }
    }
}

/// Stamp the nonzero pixels of a game glyph, clipping to the destination.
pub fn stamp_glyph<P: Copy>(
    frame: &mut [P],
    stride: usize,
    bytes: &[u8],
    origin: (i32, i32),
    token: P,
) {
    if bytes.len() < 16 || stride == 0 {
        return;
    }
    let height = frame.len() / stride;
    for y in 0..8 {
        let lo = bytes[y * 2];
        let hi = bytes[y * 2 + 1];
        for x in 0..8 {
            let bit = 7 - x;
            let color = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
            if color == 0 {
                continue;
            }
            let px = origin.0 + x as i32;
            let py = origin.1 + y as i32;
            if px < 0 || py < 0 || px >= stride as i32 || py >= height as i32 {
                continue;
            }
            frame[py as usize * stride + px as usize] = token;
        }
    }
}

#[cfg(test)]
mod tests;
