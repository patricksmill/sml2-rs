//! Route growth and minigame-hill scenery ($3D99D, $3DB12, $3DD8C).
use super::super::MapSim;

pub(super) fn tick(map: &mut MapSim) {
    if !map.zone_animation.growth_active || map.frame_counter & 7 != 0 {
        return;
    }
    let target = match (map.screen_id, map.node) {
        (1, 44) if map.level_progress[38] & 0x80 == 0 => Some((0, 39)),
        (1, 47) if map.level_progress[39] & 0x80 == 0 => Some((1, 47)),
        (3, _) => Some((0, 31)),
        _ => None,
    };
    if let Some((slot, end)) = target {
        let id = map.zone_animation.patch_ids[slot];
        if id < end {
            let next = id + 1;
            map.zone_animation.patch_ids[slot] = if next == end { 255 } else { next };
            map.zone_animation.sfx0 = if next == end { 21 } else { 6 };
        }
    }
}

pub(super) fn hill_tick(map: &mut MapSim) {
    if map.frame_counter & 7 == 0 {
        if map.frame_counter & 0x80 == 0 {
            map.zone_slots[0].0 = map.zone_slots[0].0.wrapping_sub(1);
            map.zone_slots[1].0 = map.zone_slots[1].0.wrapping_add(1);
        } else {
            map.zone_slots[0].0 = map.zone_slots[0].0.wrapping_add(1);
            map.zone_slots[1].0 = map.zone_slots[1].0.wrapping_sub(1);
        }
    }
    map.zone_slots[3].2 = 96 + (map.coins_high & 15);
    map.zone_slots[4].2 = 96 + (map.coins_low >> 4);
    map.zone_slots[5].2 = 96 + (map.coins_low & 15);
}

#[cfg(test)]
#[path = "animation_growth_tests.rs"]
mod tests;
