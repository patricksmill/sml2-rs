//! Pure projection of the state-19 ending: the [`EndingScene`] VRAM, its
//! sprite batch and the controller's raster state. Rendering never mutates.
//!
//! Each scanline takes its SCX from the `$68400` wobble and its tile
//! addressing from the `$6809E`/`$68267` split (signed above line 111,
//! unsigned below once the credits run), as the HBlank loops do.

#[cfg(test)]
mod tests;

use crate::ending_raster::{background_tile_offset, ScanlineProjection};
use crate::ending_scene::EndingScene;
use crate::ending_sequence::EndingSequenceController;
use crate::sprite_batch::MAX_SPRITE_CELLS;
use crate::video::{tile_colors, OutputMode, PaletteSource};

/// Visible screen size.
const SCREEN: (usize, usize) = (160, 144);
/// OAM entries the hardware draws on one line.
const SPRITES_PER_LINE: usize = 10;

/// The ending's substates `0..=3` (`$4520`) only advance the state; the
/// screen still holds the entry fade's all-white palettes until `$29DD`.
const FIRST_SCENE_SUBSTATE: u8 = 5;

/// One native frame of the ending at `view`.
pub fn render_ending_output(
    scene: &EndingScene,
    sequence: &EndingSequenceController,
    view: (usize, usize),
    output: OutputMode,
) -> Vec<u8> {
    let (vw, vh) = view;
    let white = output.palette(0x00, PaletteSource::Bg)[0];
    let (Some(assets), Some(palettes)) = (scene.assets(), scene.palettes) else {
        return vec![white; vw * vh];
    };
    if sequence.substate < FIRST_SCENE_SUBSTATE || !scene.display.display_enabled {
        return vec![white; vw * vh];
    }
    let bg = output.palette(palettes.background, PaletteSource::Bg);
    let obp = [
        output.palette(palettes.object0, PaletteSource::Obj0),
        output.palette(palettes.object1, PaletteSource::Obj1),
    ];
    let split = sequence.substate >= 18;
    let cells = scene.objects.cells();
    let mut img = vec![bg[0]; vw * vh];
    for y in 0..vh.min(SCREEN.1) {
        let line = ScanlineProjection::compute(sequence.frame, y as u8, sequence.raster_y, split);
        let unsigned = scene.display.background_unsigned_tiles || line.unsigned_bg_tiles;
        let scx = scene.scroll_x.wrapping_add(line.scroll_x as u8);
        let by = (y as u8).wrapping_add(scene.scroll_y);
        // Colour index per pixel, kept for the OBJ-behind-BG test.
        let mut bg_index = [0u8; 160];
        for (x, index) in bg_index.iter_mut().enumerate().take(vw.min(SCREEN.0)) {
            if !scene.display.background_enabled {
                break;
            }
            let bx = (x as u8).wrapping_add(scx);
            let id = assets.tilemap()[usize::from(by / 8) * 32 + usize::from(bx / 8)];
            let offset = background_tile_offset(id, unsigned);
            let tile: &[u8; 16] = assets.tiles()[offset..offset + 16].try_into().unwrap();
            *index = tile_colors(tile)[usize::from(by % 8) * 8 + usize::from(bx % 8)];
            img[y * vw + x] = bg[usize::from(*index)];
        }
        if !scene.display.objects_enabled {
            continue;
        }
        // The first ten OAM entries on this line, then the smaller x wins
        // (OAM order breaks ties), so paint winners last.
        let mut on_line: Vec<(usize, &crate::player::Cell)> = cells
            .iter()
            .enumerate()
            .take(MAX_SPRITE_CELLS)
            .filter(|(_, c)| (0..8).contains(&(y as i32 + 16 - i32::from(c.y))))
            .take(SPRITES_PER_LINE)
            .collect();
        on_line.sort_by_key(|&(index, c)| (c.x, index));
        for &(_, cell) in on_line.iter().rev() {
            let row = y as i32 + 16 - i32::from(cell.y);
            let src_y = if cell.attr & 0x40 != 0 { 7 - row } else { row } as usize;
            let colors = tile_colors(assets.object_tile(cell.tile));
            let shades = &obp[usize::from((cell.attr >> 4) & 1)];
            for dx in 0..8 {
                let x = i32::from(cell.x) - 8 + dx;
                if x < 0 || x >= vw.min(SCREEN.0) as i32 {
                    continue;
                }
                let src_x = if cell.attr & 0x20 != 0 { 7 - dx } else { dx } as usize;
                let color = colors[src_y * 8 + src_x];
                if color == 0 || (cell.attr & 0x80 != 0 && bg_index[x as usize] != 0) {
                    continue;
                }
                img[y * vw + x as usize] = shades[usize::from(color)];
            }
        }
    }
    img
}
