//! Mario Zone scenery and route tiles, ROM $0F:$5EC7-$5FE7.
use super::super::MapSim;

// $5FA0..$5FC7: frame, paired hand heights, and paired shoulder positions.
const DECOR_FRAMES: [u8; 8] = [30, 31, 32, 33, 34, 33, 32, 31];
const HAND_HEIGHTS: [u8; 8] = [37, 36, 35, 36, 37, 36, 35, 36];
const LEFT_SHOULDER: [u8; 8] = [74, 75, 76, 75, 74, 73, 72, 73];
const RIGHT_SHOULDER: [u8; 8] = [94, 95, 96, 95, 94, 93, 92, 93];
// $5FC8/$5FD0: eight phases, each retained for thirty-two map frames.
const ROUTE_A: [u8; 8] = [0, 1, 2, 1, 0, 14, 15, 14];
const ROUTE_B: [u8; 8] = [3, 12, 13, 12, 3, 4, 5, 4];

/// The compatibility boundary names the growth sprite coordinates; the ROM's
/// `sMapSprite1Y/Id` aliases are the third zone triple's X/Y coordinates.
fn grow_statue(position: &mut (u8, u8, u8)) {
    if position.0 < 92 {
        position.0 += 1;
        if position.1 < 19 {
            position.1 += 1;
        }
    }
}

pub(super) fn tick(map: &mut MapSim, _div: u8) {
    let frame = map.frame_counter;
    let last_route_open = map.level_progress[28] & 0x80 != 0;
    if frame & 3 == 0 && map.level_progress[29] & 0x80 == 0 && last_route_open {
        grow_statue(&mut map.zone_slots[2]);
    }
    // $A6C4 (sMapSprite3Id) is the sixth triple's X coordinate.
    if frame & 1 == 0 && map.cursor_x >= 46 && map.zone_slots[5].0 < 74 {
        map.zone_slots[5].0 += 1;
    }

    let phase = ((frame & 0x70) >> 4) as usize;
    map.zone_slots[6].2 = DECOR_FRAMES[phase];
    map.zone_slots[0].1 = HAND_HEIGHTS[phase];
    map.zone_slots[1].1 = HAND_HEIGHTS[phase];
    map.zone_slots[3].0 = LEFT_SHOULDER[phase];
    map.zone_slots[4].0 = RIGHT_SHOULDER[phase];
    if frame & 0x0F != 0 {
        return;
    }

    let phase = ((frame & 0xE0) >> 5) as usize;
    let first_route_open = map.level_progress[26] & 0x80 != 0;
    let middle_route_open = map.level_progress[27] & 0x80 != 0;
    let patches = &mut map.zone_animation.patch_ids;
    patches[0] = ROUTE_A[phase];
    patches[1] = ROUTE_B[phase];
    if !middle_route_open && first_route_open && patches[2] < 8 {
        patches[2] += 1;
    }
    if !last_route_open && middle_route_open && patches[3] < 11 {
        patches[3] += 1;
    }
}

#[cfg(test)]
#[path = "animation_mario_tests.rs"]
mod tests;
