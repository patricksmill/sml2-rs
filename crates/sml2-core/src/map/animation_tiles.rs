//! Zone IRQ tile writers $3EC7D/$3ECF5/$3ED35 and home-bank copy helpers.
//!
//! `source` for `apply` concatenates ROM $41300..$41600 (Pumpkin),
//! $39800..$39900 (Tree), and $39C00..$39C80 (Turtle), in that order.

use super::super::MapSim;

pub(super) const SOURCE_LEN: usize = 1152;

const PUMPKIN_PHASES: [u8; 64] = [
    4, 4, 5, 5, 6, 6, 7, 7, 4, 4, 5, 5, 6, 6, 7, 7, 4, 4, 5, 5, 6, 6, 7, 7, 4, 4, 5, 5, 6, 6, 7, 7,
    8, 11, 10, 9, 8, 8, 4, 4, 8, 9, 10, 11, 8, 8, 8, 8, 4, 4, 5, 5, 8, 8, 8, 8, 0, 1, 2, 3, 0, 0,
    0, 0,
];
const TREE_PHASES: [u8; 8] = [0, 1, 2, 3, 3, 3, 3, 3];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TileAnimation {
    pub(super) irq_divider: u8,
    pub(super) patch_index: u8,
    pub(super) pumpkin_frame: Option<u8>,
    pub(super) tree_frame: Option<u8>,
    pub(super) turtle_frame: Option<u8>,
}

pub(super) fn reset(map: &mut MapSim) {
    let state = &mut map.zone_animation.tiles;
    if matches!(map.screen_id, 1 | 2) {
        state.irq_divider = 0;
        state.patch_index = 0;
    }
    // Every loader reinstalls its base graphics, replacing previous IRQ writes.
    state.pumpkin_frame = None;
    state.tree_frame = None;
    state.turtle_frame = None;
}

pub(super) fn tick(map: &mut MapSim, _div: u8) {
    let state = &mut map.zone_animation.tiles;
    let enabled = match map.screen_id {
        1 => map.level_progress[39] & 0x80 != 0, // sMapGatePumpkinC ($A86F)
        2 => map.level_progress[36] & 0x80 != 0, // sMapGatePumpkinA ($A86C)
        5 => {
            // Source skips the write on multiples of 16, retaining prior VRAM.
            if map.frame_counter & 0x0F != 0 {
                state.turtle_frame = Some((map.frame_counter & 0x30) >> 4);
            }
            return;
        }
        _ => return,
    };
    if !enabled {
        return;
    }
    state.irq_divider = state.irq_divider.wrapping_add(1);
    if state.irq_divider >= 14 {
        state.irq_divider = 0;
    }
    if state.irq_divider != 0 {
        return;
    }
    state.patch_index = state.patch_index.wrapping_add(1);
    if map.screen_id == 1 {
        state.pumpkin_frame = Some(PUMPKIN_PHASES[(state.patch_index & 63) as usize]);
    } else {
        state.tree_frame = Some(TREE_PHASES[(state.patch_index & 7) as usize]);
    }
}

impl TileAnimation {
    /// Reapply retained IRQ writes to a fresh decoded VRAM tile bank. Pixel
    /// indices stay unpaletted so presentation may apply the current palette.
    pub(super) fn apply(&self, screen: u8, tiles: &mut [[u8; 64]], source: &[u8]) -> bool {
        if source.len() != SOURCE_LEN || tiles.len() < 258 {
            return false;
        }
        match screen {
            1 => {
                if let Some(frame) = self.pumpkin_frame {
                    if frame >= 12 {
                        return false;
                    }
                    let bytes = &source[frame as usize * 64..][..64];
                    write_pair(tiles, 142, &bytes[..32]); // $88E0-$88FF
                    write_pair(tiles, 158, &bytes[32..]); // $89E0-$89FF
                }
            }
            2 => {
                if let Some(frame) = self.tree_frame {
                    if frame >= 4 {
                        return false;
                    }
                    let bytes = &source[768 + frame as usize * 64..][..64];
                    write_pair(tiles, 130, &bytes[..32]); // $8820-$883F
                    write_pair(tiles, 146, &bytes[32..]); // $8920-$893F
                }
            }
            5 => {
                if let Some(frame) = self.turtle_frame {
                    if frame >= 4 {
                        return false;
                    }
                    write_pair(tiles, 256, &source[1024 + frame as usize * 32..][..32]);
                }
            }
            _ => {}
        }
        true
    }
}

fn write_pair(tiles: &mut [[u8; 64]], first: usize, bytes: &[u8]) {
    for (tile, encoded) in tiles[first..first + 2]
        .iter_mut()
        .zip(bytes.as_chunks::<16>().0)
    {
        for (row, planes) in encoded.as_chunks::<2>().0.iter().enumerate() {
            for x in 0..8 {
                let shift = 7 - x;
                tile[row * 8 + x] = ((planes[0] >> shift) & 1) | (((planes[1] >> shift) & 1) << 1);
            }
        }
    }
}

#[cfg(test)]
#[path = "animation_tiles_tests.rs"]
mod tests;
