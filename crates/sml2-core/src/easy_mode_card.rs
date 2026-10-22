//! Easy mode's "EASY MODE" card, home `$3573` (state `$16`) and `$35CD`
//! (state `$17`). With `sEasyMode` set, the level loader first copies bank
//! 26 `$4656..$4955` to `$8800`, turns the background off (`rLCDC = $E2`)
//! and, for the 64 frames of `sFadeTimer`, draws sprite 240 at (52, 80)
//! with `sOAMPalette1 = $D2` / `sOAMPalette2 = $39`; then it goes on to
//! state `$0B` as a normal entry does.

#[cfg(test)]
mod tests;

use crate::room_sprites::SpriteMetas;
use crate::video::{tile_colors, OutputMode, PaletteSource};

/// `sFadeTimer` frames `$35CD` holds the card.
pub const EASY_CARD_FRAMES: u16 = 64;
/// Bytes `$3573` copies to `$8800`: OBJ tiles `$80..$AF`.
pub const EASY_CARD_FONT_LEN: usize = 0x300;
/// `$35CD`'s `$FFC5`/`$FFC4`/`$FFC6`: x, y and sprite id.
const CARD: (u8, u8, u8) = (52, 80, 240);
/// `sOAMPalette1`, `sOAMPalette2`.
const OBP: [u8; 2] = [0xD2, 0x39];

/// One frame of the card. `font` is the `$8800` copy; the background is
/// disabled, so everything else is colour 0 (white under any palette).
pub fn render_easy_card_output(
    font: &[u8],
    metas: &SpriteMetas,
    view: (usize, usize),
    output: OutputMode,
) -> Vec<u8> {
    let (vw, vh) = view;
    let mut img = vec![output.palette(0xE0, PaletteSource::Bg)[0]; vw * vh];
    let (x, y, id) = CARD;
    for (cx, cy, tile, attr) in metas.expand(id, x, y) {
        let Some(index) = usize::from(tile).checked_sub(0x80) else {
            continue;
        };
        let Some(bytes) = font.get(index * 16..index * 16 + 16) else {
            continue;
        };
        let colors = tile_colors(bytes.try_into().unwrap());
        let palette = if attr & 0x10 == 0 {
            output.palette(OBP[0], PaletteSource::Obj0)
        } else {
            output.palette(OBP[1], PaletteSource::Obj1)
        };
        for row in 0..8 {
            for col in 0..8 {
                let (px, py) = (i32::from(cx) - 8 + col, i32::from(cy) - 16 + row);
                if px < 0 || py < 0 || px >= vw as i32 || py >= vh as i32 {
                    continue;
                }
                let sx = if attr & 0x20 != 0 { 7 - col } else { col };
                let sy = if attr & 0x40 != 0 { 7 - row } else { row };
                let color = colors[(sy * 8 + sx) as usize];
                if color != 0 {
                    img[py as usize * vw + px as usize] = palette[usize::from(color)];
                }
            }
        }
    }
    img
}
