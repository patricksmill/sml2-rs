//! Tree-B decorations, ROM $3DDD3-$3DEC6. Hub WRAM symbol names alias
//! the zone loader's three-byte x/y/id records here.
use super::super::MapSim;

pub(super) fn reset(map: &mut MapSim) {
    map.zone_animation.moving_x = 160;
    // $A6A8 is deliberately retained by this loader.
}

pub(super) fn tick(map: &mut MapSim, div: u8) {
    let frame = map.frame_counter;
    let phase = (frame & 0x30) >> 4;
    for slot in [1, 2, 3] {
        map.zone_slots[slot].2 = 42 + phase;
    }
    for slot in [7, 8] {
        map.zone_slots[slot].2 = 46 + phase;
    }
    let drift = (frame & 0x7C) >> 2;
    map.zone_slots[8].1 = 80 - drift;
    map.zone_slots[7].1 = 64 - (drift & 15);
    if map.level_progress[37] & 0x80 == 0 {
        map.zone_slots[0].2 = 38 + ((frame & 0x60) >> 5);
    } else if frame & 15 == 0 {
        let animation = &mut map.zone_animation;
        animation.random_accum = animation
            .random_accum
            .wrapping_add(div.rotate_left(4))
            .wrapping_add(div)
            .wrapping_add(animation.global_frame);
        animation.patch_ids[0] = 53 + (animation.random_accum & 1);
    }

    // ROM shifts bit 1 down, then ANDs the unshifted frame: only
    // frames with both low bits set advance this moving decoration.
    let distance = ((frame & 2) >> 1) & frame;
    let animation = &mut map.zone_animation;
    let (id, edge, y) = if animation.moving_direction & 1 == 0 {
        animation.moving_x = animation.moving_x.wrapping_sub(distance);
        (50, 0xC0, 120)
    } else {
        animation.moving_x = animation.moving_x.wrapping_add(distance);
        (51, 0xB0, 64)
    };
    if animation.moving_x == edge {
        animation.moving_direction ^= 255;
        map.zone_slots[9].1 = y;
    }
    map.zone_slots[9].0 = animation.moving_x;
    map.zone_slots[9].2 = id;

    if frame & 15 == 0 && map.zone_slots[10].2 < 129 {
        map.zone_slots[10].2 += 1;
        animation.sfx0 = if map.zone_slots[10].2 == 129 { 21 } else { 6 };
    }
}

#[cfg(test)]
#[path = "animation_tree_b_tests.rs"]
mod tests;
