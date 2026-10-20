use super::*;

fn mario() -> MapSim {
    let mut map = MapSim::new();
    map.screen_id = 6;
    map.zone_slots = [(0xA5, 0x5A, 0xFF); 16];
    map.zone_slots[2] = (76, 16, 35);
    map.zone_slots[5] = (70, 119, 37);
    map.zone_animation.patch_ids[..4].copy_from_slice(&[0, 3, 6, 9]);
    map
}

#[test]
fn mario_animation_matches_all_eight_rom_table_phases_and_wraps() {
    let mut map = mario();
    let frames = [30, 31, 32, 33, 34, 33, 32, 31];
    let heights = [37, 36, 35, 36, 37, 36, 35, 36];
    let left = [74, 75, 76, 75, 74, 73, 72, 73];
    let right = [94, 95, 96, 95, 94, 93, 92, 93];
    for base in [0, 128] {
        for phase in 0..8 {
            for offset in [0, 15] {
                map.frame_counter = base + (phase * 16) as u8 + offset;
                tick(&mut map, 0);
                assert_eq!(map.zone_slots[6], (0xA5, 0x5A, frames[phase]));
                assert_eq!(map.zone_slots[0], (0xA5, heights[phase], 255));
                assert_eq!(map.zone_slots[1], (0xA5, heights[phase], 255));
                assert_eq!(map.zone_slots[3], (left[phase], 0x5A, 255));
                assert_eq!(map.zone_slots[4], (right[phase], 0x5A, 255));
            }
        }
    }
    map.frame_counter = 0;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[6].2, 30);
}

#[test]
fn mario_growth_requires_four_frame_gate_last_path_and_uncollected_coin() {
    for (frame, path, coin, expected) in [
        (4, 0x80, 0, (77, 17, 35)),
        (3, 0x80, 0, (76, 16, 35)),
        (4, 0x7F, 0, (76, 16, 35)),
        (4, 0x80, 0x80, (76, 16, 35)),
        (4, 0x80, 0x7F, (77, 17, 35)),
    ] {
        let mut map = mario();
        map.frame_counter = frame;
        map.level_progress[28] = path;
        map.level_progress[29] = coin;
        tick(&mut map, 0);
        assert_eq!(map.zone_slots[2], expected);
    }
}

#[test]
fn mario_growth_clamps_x_before_y_and_does_not_wrap_high_coordinates() {
    for (before, after) in [
        ((91, 18, 35), (92, 19, 35)),
        ((91, 19, 35), (92, 19, 35)),
        ((92, 18, 35), (92, 18, 35)),
        ((255, 255, 35), (255, 255, 35)),
    ] {
        let mut map = mario();
        map.level_progress[28] = 0x80;
        map.frame_counter = 4;
        map.zone_slots[2] = before;
        tick(&mut map, 0);
        assert_eq!(map.zone_slots[2], after);
    }
}

#[test]
fn mario_moving_part_requires_even_frame_cursor_threshold_and_unfinished_x() {
    for (frame, cursor, x, expected) in [
        (2, 46, 73, 74),
        (2, 45, 73, 73),
        (3, 46, 73, 73),
        (2, 46, 74, 74),
        (2, 46, 255, 255),
    ] {
        let mut map = mario();
        map.frame_counter = frame;
        map.cursor_x = cursor;
        map.zone_slots[5].0 = x;
        tick(&mut map, 0);
        assert_eq!(map.zone_slots[5], (expected, 119, 37));
    }
}

#[test]
fn mario_route_tables_step_every_sixteen_frames_with_thirty_two_frame_phases() {
    let mut map = mario();
    let expected_a = [0, 1, 2, 1, 0, 14, 15, 14];
    let expected_b = [3, 12, 13, 12, 3, 4, 5, 4];
    for phase in 0..8 {
        for offset in [0, 16] {
            map.frame_counter = (phase * 32) as u8 + offset;
            tick(&mut map, 0);
            assert_eq!(
                &map.zone_animation.patch_ids[..4],
                &[expected_a[phase], expected_b[phase], 6, 9]
            );
            map.zone_animation.patch_ids[0] = 0xAA;
            map.frame_counter += 1;
            tick(&mut map, 0);
            assert_eq!(map.zone_animation.patch_ids[0], 0xAA);
        }
    }
}

#[test]
fn mario_route_growth_obeys_predecessor_bits_and_clamps_patch_ids() {
    for (paths, initial, expected) in [
        ([0x80, 0, 0], [6, 9], [7, 9]),
        ([0x80, 0, 0], [7, 9], [8, 9]),
        ([0x80, 0, 0], [8, 9], [8, 9]),
        ([0x80, 0x80, 0], [6, 9], [6, 10]),
        ([0, 0x80, 0], [6, 10], [6, 11]),
        ([0, 0x80, 0], [6, 11], [6, 11]),
        ([0x80, 0x80, 0x80], [6, 9], [6, 9]),
        ([0x7F, 0x7F, 0], [6, 9], [6, 9]),
        ([0x80, 0, 0], [255, 255], [255, 255]),
    ] {
        let mut map = mario();
        map.level_progress[26..29].copy_from_slice(&paths);
        map.zone_animation.patch_ids[2..4].copy_from_slice(&initial);
        map.frame_counter = 16;
        tick(&mut map, 0);
        assert_eq!(&map.zone_animation.patch_ids[2..4], expected);
        map.frame_counter = 17;
        tick(&mut map, 0);
        assert_eq!(&map.zone_animation.patch_ids[2..4], expected);
    }
}
