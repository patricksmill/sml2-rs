//! Pure projection of bank-12 VRAM/OAM staging; rendering never consumes state.
use super::*;
use crate::castle::CastleScripts;
use crate::castle_sequence::{CastleSequence, STATE_CONGRATS, STATE_MENU, STATE_WAIT_START};
use crate::room_sprites::SpriteMetas;

/// $309CD copies a different overlapping window into the same five tile runs.
fn door_tilemap(reveal: &CastleRevealTiles, rows: u8, windows: &[u8]) -> Vec<u8> {
    let mut map = reveal.tilemap.clone();
    if (1..=5).contains(&rows) {
        let start = (5 - rows) as usize * 4;
        if let Some(window) = windows.get(start..start + 20) {
            for (row, &run) in CastleRevealTiles::COIN_RUNS.iter().rev().enumerate() {
                map[run..run + 4].copy_from_slice(&window[row * 4..row * 4 + 4]);
            }
        }
    }
    map
}

pub fn render_castle_sequence(
    reveal: &CastleRevealTiles,
    scripts: &CastleScripts,
    metas: &SpriteMetas,
    seq: &CastleSequence,
    view: (usize, usize),
) -> Vec<u8> {
    render_castle_sequence_output(
        reveal,
        scripts,
        metas,
        seq,
        view,
        crate::video::OutputMode::Grayscale,
    )
}

pub fn render_castle_sequence_output(
    reveal: &CastleRevealTiles,
    scripts: &CastleScripts,
    metas: &SpriteMetas,
    seq: &CastleSequence,
    view: (usize, usize),
    output: crate::video::OutputMode,
) -> Vec<u8> {
    let mut vram = reveal.vram_tiles();
    let congrats = matches!(seq.state, STATE_CONGRATS | STATE_WAIT_START);
    let map = if congrats {
        if let Some(bytes) = scripts.blob("castle_congrats_tiles") {
            for (index, bytes) in bytes.as_chunks::<16>().0.iter().enumerate() {
                if let Some(tile) = vram.get_mut(128 + index) {
                    *tile = crate::video::tile_colors(bytes);
                }
            }
        }
        scripts
            .blob("castle_congrats_tilemap")
            .unwrap_or(&[])
            .to_vec()
    } else {
        door_tilemap(
            reveal,
            seq.door_open_rows,
            scripts.blob("castle_row_windows").unwrap_or(&[]),
        )
    };
    let mut sprites = Vec::new();
    if !congrats {
        if let Some(sprite) = seq.sparkle {
            sprites.push(sprite);
        }
        if !(seq.state == STATE_MENU && seq.menu >= 2) {
            sprites.push((seq.walk_x, seq.walk_y, seq.sprite_id));
        }
        // $307BC: draw collected icons in order 1,2,6,5,4,3. These are
        // the $4985/$499D/$49B5 entries for the six boss levels.
        for (coin, x, y, id) in [
            (0, 59, 121, 0xC4),
            (1, 105, 70, 0xDA),
            (5, 117, 93, 0xDB),
            (4, 117, 121, 0xD8),
            (3, 59, 93, 0xD7),
            (2, 71, 70, 0xD9),
        ] {
            if seq.coins[coin] & 0x80 != 0 {
                sprites.push((x, y, id));
            }
        }
    }
    draw(
        &vram,
        &map,
        seq.bgp,
        [seq.obp1, seq.obp2],
        &sprites,
        metas,
        view,
        output,
    )
}

/// What the GAME OVER screen (state `$07`) draws from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameOverView {
    /// `sFadeTimer`, armed to 255 by `$27AF`.
    pub fade_timer: u16,
    /// `$FF97`, which picks Mario's frame.
    pub frame_counter: u8,
    /// `sGoldenCoinFlags`, folded by `$27AF` before it clears the coins.
    pub golden_coin_flags: u8,
    /// `sOAMPalette1`/`sOAMPalette2`, which `$27AF` leaves as they were.
    pub obp: [u8; 2],
}

/// `$27AF`'s backdrop is the castle picture (`$4C92` tiles, `$6492` tilemap,
/// `sBGPalette = $E4`). Each `$07` frame `$30884` draws the collected coins
/// at their castle-screen spots, rising off the top through the spinning
/// frames once the fade drops below 128, then `$28A7` draws Mario, who slides
/// down while the fade is above 191.
pub fn render_game_over_output(
    reveal: &CastleRevealTiles,
    metas: &SpriteMetas,
    view_state: GameOverView,
    view: (usize, usize),
    output: crate::video::OutputMode,
) -> Vec<u8> {
    draw(
        &reveal.vram_tiles(),
        &reveal.tilemap,
        0xE4,
        view_state.obp,
        &game_over_sprites(view_state),
        metas,
        view,
        output,
    )
}

/// The `$5297` calls of one `$07` frame, as `(x, y, id)` in draw order.
pub fn game_over_sprites(view_state: GameOverView) -> Vec<(u8, u8, u8)> {
    let fade_high = (view_state.fade_timer >> 8) as u8;
    let fade_low = view_state.fade_timer as u8;
    // `sCoinIconDropY`: `fade - 128` once the fade is below 128, added to y.
    let rise = if fade_high == 0 && fade_low < 0x80 {
        fade_low.wrapping_sub(0x80)
    } else {
        0
    };
    let mut sprites = Vec::new();
    // `sGoldenCoinFlags` bits 5..0, the `$498A..$49CC` entries (the same
    // per-level spots `$307BC` uses).
    for (bit, x, y, id) in [
        (5, 59, 121u8, 0xC4),
        (4, 105, 70, 0xDA),
        (3, 117, 93, 0xDB),
        (2, 117, 121, 0xD8),
        (1, 59, 93, 0xD7),
        (0, 71, 70, 0xD9),
    ] {
        if view_state.golden_coin_flags & (1 << bit) == 0 {
            continue;
        }
        // `$3097C`: a rising coin spins through ids 222..225.
        let id = if rise == 0 {
            id
        } else {
            222 + ((rise & 6) >> 1)
        };
        sprites.push((x, y.wrapping_add(rise), id));
    }
    if fade_high == 0 {
        let id = 227 + ((view_state.frame_counter & 0x10) >> 4);
        let y = match fade_low.checked_sub(192) {
            Some(over) => 32 - (over >> 1),
            None => 32,
        };
        sprites.push((68, y, id));
    }
    sprites
}

/// Compatibility static-frame renderer used by callers without sequence state.
pub fn render_castle_reveal(
    reveal: &CastleRevealTiles,
    all_six_coins: bool,
    bgp: u8,
    obp1: u8,
    mario: Option<(u8, u8, u8)>,
    metas: &SpriteMetas,
    vw: usize,
    vh: usize,
) -> Vec<u8> {
    let mut map = reveal.tilemap.clone();
    if all_six_coins {
        for run in CastleRevealTiles::COIN_RUNS {
            map[run..run + 4].fill(0x74);
        }
    }
    draw(
        &reveal.vram_tiles(),
        &map,
        bgp,
        [obp1; 2],
        &mario.into_iter().collect::<Vec<_>>(),
        metas,
        (vw, vh),
        crate::video::OutputMode::Grayscale,
    )
}

fn draw(
    vram: &[[u8; 64]],
    map: &[u8],
    bgp: u8,
    obp: [u8; 2],
    sprites: &[(u8, u8, u8)],
    metas: &SpriteMetas,
    (vw, vh): (usize, usize),
    output: crate::video::OutputMode,
) -> Vec<u8> {
    let shades = output.palette(bgp, crate::video::PaletteSource::Bg);
    let mut img = vec![shades[0]; vw * vh];
    for y in 0..vh.min(144) {
        for x in 0..vw.min(256) {
            let Some(&id) = map.get((y / 8) * 32 + x / 8) else {
                continue;
            };
            if let Some(tile) = vram.get(signed_bg_tile_index(id)) {
                img[y * vw + x] = shades[tile[(y % 8) * 8 + x % 8] as usize];
            }
        }
    }
    let mut cells = Vec::new();
    for &(x, y, id) in sprites {
        cells.extend(metas.expand(id, x, y));
    }
    // Hardware gives an overlapping cell with smaller x (then lower OAM index)
    // priority. Paint winners last, preserving both OBJ palette selections.
    let mut cells: Vec<_> = cells.into_iter().enumerate().collect();
    cells.sort_by_key(|&(index, (x, _, _, _))| (x, index));
    for (_, (x, y, tile, attr)) in cells.into_iter().rev() {
        let palette = output.palette(
            obp[((attr >> 4) & 1) as usize],
            if attr & 0x10 != 0 {
                crate::video::PaletteSource::Obj1
            } else {
                crate::video::PaletteSource::Obj0
            },
        );
        draw_obj_tile(
            &mut img,
            vram,
            &palette,
            x as i32 - 8,
            y as i32 - 16,
            tile,
            attr,
            vw,
            vh,
        );
    }
    img
}

#[cfg(test)]
#[path = "castle_render_tests.rs"]
mod tests;
