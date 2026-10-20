use super::*;

fn tree_map() -> MapSim {
    let mut map = MapSim::new();
    map.zone_slots[0] = (68, 17, 0);
    map.zone_slots[1] = (128, 88, 4);
    map.zone_slots[2] = (28, 50, 6);
    reset(&mut map);
    map
}

#[test]
fn tree_animation_matches_rom_bytes_at_both_clock_gates() {
    let mut map = tree_map();
    map.tick_timer = 39;
    map.rand_latch = 0xB0;
    map.frame_counter = 1;
    map.zone_animation.tree.tick = 8;
    map.level_progress[3] = 0x80;
    tick(&mut map, 0);
    assert_eq!(
        &map.zone_slots[..3],
        &[(68, 17, 3), (129, 87, 5), (28, 50, 6)]
    );
    assert_eq!(map.zone_slots[6].2, 14);
    assert_eq!(map.zone_slots[4].2, 255);
    assert_eq!(map.zone_animation.tree.index_b, 1);
    assert_eq!(map.zone_animation.tree.tick, 9);
    map.tick_timer = 38;
    tick(&mut map, 0);
    assert_eq!(
        &map.zone_slots[..3],
        &[(68, 17, 3), (129, 87, 5), (27, 51, 92)]
    );
    assert_eq!(map.zone_animation.tree.index_b, 1);
    assert_eq!(map.zone_animation.tree.tick, 0);
    assert_eq!(map.zone_animation.tree.step, 17);
}

#[test]
fn tree_animation_signed_motion_and_counter_wrap_match_rom_bytes() {
    for (index, before, after) in [
        (16, (255, 255), (255, 0)), // $01: positive y only
        (19, (0, 255), (0, 0)),     // $81: negative x magnitude zero
        (20, (0, 255), (255, 0)),   // $91: negative x, positive y
        (30, (0, 0), (255, 255)),   // $99: both negative
        (79, (255, 0), (0, 255)),   // wrap to table[0] = $19
    ] {
        let mut map = tree_map();
        map.tick_timer = 1;
        map.frame_counter = 1;
        map.zone_animation.tree.index_b = index;
        map.zone_slots[1].0 = before.0;
        map.zone_slots[1].1 = before.1;
        tick(&mut map, 0);
        assert_eq!(map.zone_slots[1], (after.0, after.1, 4), "index {index}");
        assert_eq!(
            map.zone_animation.tree.index_b,
            if index == 79 { 0 } else { index + 1 }
        );
    }
    let mut map = tree_map();
    map.zone_animation.tree.tick = 9;
    map.zone_animation.tree.step = 255;
    map.zone_slots[2] = (255, 0, 0);
    map.tick_timer = 0;
    map.frame_counter = 1;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[2], (0, 255, 87));
    assert_eq!(map.zone_animation.tree.step, 0);
    map.level_progress[5] = 0x80;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[0].2, 255);
}

#[test]
fn tree_secret_growth_has_sixteen_frame_gate_and_final_sound() {
    let mut map = tree_map();
    map.zone_slots[9].2 = 106;
    for (frame, expected_id, expected_sound) in [
        (15, 106, 0),
        (16, 107, 6),
        (32, 108, 6),
        (48, 109, 21),
        (64, 109, 21),
    ] {
        map.frame_counter = frame;
        tick(&mut map, 0);
        assert_eq!(
            (map.zone_slots[9].2, map.zone_animation.sfx0),
            (expected_id, expected_sound)
        );
    }
    map.zone_slots[9].2 = 255;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[9].2, 255);
}

#[test]
fn tree_load_resets_step_and_index_but_retains_clock() {
    let mut map = tree_map();
    map.zone_animation.tree.tick = 7;
    map.zone_animation.tree.step = 45;
    map.zone_animation.tree.index_b = 60;
    reset(&mut map);
    assert_eq!(
        (
            map.zone_animation.tree.tick,
            map.zone_animation.tree.step,
            map.zone_animation.tree.index_b
        ),
        (7, 16, 0)
    );
}
