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
    )
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
) -> Vec<u8> {
    let shades = crate::video::bgp_shades(bgp);
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
        let palette = crate::video::bgp_shades(obp[((attr >> 4) & 1) as usize]);
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
