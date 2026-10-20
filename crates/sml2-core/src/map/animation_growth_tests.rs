use super::*;
use crate::map::MapNodeRecords;

#[test]
fn fresh_macro_route_advances_every_eighth_frame_and_keeps_painted_tiles() {
    let mut map = MapSim::new();
    map.node = 50;
    map.load_screen(3, 1, &MapNodeRecords::default());
    assert_eq!(map.zone_animation.patch_ids[0], 16);
    let initial = map.zone_tile_writes();
    assert!(!initial.is_empty());
    map.frame_counter = 6;
    map.bookkeeping(0);
    assert_eq!(map.zone_animation.patch_ids[0], 16);
    map.bookkeeping(0);
    assert_eq!(map.zone_animation.patch_ids[0], 17);
    assert_eq!(map.zone_animation.sfx0, 6);
    map.zone_animation.patch_ids[0] = 30;
    map.frame_counter = 15;
    map.bookkeeping(0);
    assert_eq!(map.zone_animation.patch_ids[0], 255);
    assert_eq!(map.zone_animation.sfx0, 21);
    for (at, tile) in initial {
        assert!(map.zone_tile_writes().contains(&(at, tile)));
    }
}

#[test]
fn pumpkin_routes_have_separate_growth_slots_and_gate_flags() {
    for (node, slot, first, gate) in [(44, 0, 31, 38), (47, 1, 39, 39)] {
        let mut map = MapSim::new();
        map.node = node;
        map.load_screen(1, 1, &MapNodeRecords::default());
        map.frame_counter = 7;
        map.bookkeeping(0);
        assert_eq!(map.zone_animation.patch_ids[slot], first + 1);
        map.level_progress[gate] = 0x80;
        map.frame_counter = 15;
        map.bookkeeping(0);
        assert_eq!(map.zone_animation.patch_ids[slot], first + 1);
    }
}

#[test]
fn hill_bobs_in_opposite_directions_and_updates_bcd_coin_digits() {
    let mut map = MapSim::new();
    map.load_screen(8, 0, &MapNodeRecords::default());
    map.coins_high = 9;
    map.coins_low = 0x87;
    map.frame_counter = 7;
    map.bookkeeping(0);
    assert_eq!(map.zone_slots[0].0, 7);
    assert_eq!(map.zone_slots[1].0, 129);
    assert_eq!(
        [
            map.zone_slots[3].2,
            map.zone_slots[4].2,
            map.zone_slots[5].2
        ],
        [105, 104, 103]
    );
    map.frame_counter = 127;
    map.bookkeeping(0);
    assert_eq!(map.zone_slots[0].0, 8);
    assert_eq!(map.zone_slots[1].0, 128);
}

#[test]
fn pumpkin_coin_visibility_uses_the_pumpkin_coin_not_the_macro_coin() {
    let mut map = MapSim::new();
    map.load_screen(1, 0, &MapNodeRecords::default());
    map.level_progress[14] = 0x80;
    map.bookkeeping(0);
    assert_ne!(map.zone_slots[2].2, 255);
    map.level_progress[9] = 0x80;
    map.bookkeeping(0);
    assert_eq!(map.zone_slots[2].2, 255);
}
