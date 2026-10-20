use super::*;

fn space() -> MapSim {
    let mut map = MapSim::new();
    map.screen_id = 4;
    map.zone_slots[0] = (36, 40, 73);
    map.zone_slots[1] = (32, 71, 67);
    map.zone_slots[2] = (32, 32, 67);
    map.zone_slots[3] = (64, 129, 67);
    map.zone_slots[6] = (224, 224, 70);
    map.zone_slots[7] = (248, 144, 255);
    map.zone_slots[8] = (248, 144, 255);
    map.zone_slots[9] = (86, 86, 255);
    map
}

#[test]
fn space_stars_use_rom_frames_positions_and_staggered_eight_frame_gates() {
    let mut map = space();
    for (frame, div, frames, positions) in [
        (
            0,
            0x00,
            [0x43, 0xFF, 0x45],
            [(0x20, 0x80), (0x20, 0x20), (0x40, 0x81)],
        ),
        (
            8,
            0x06,
            [0x44, 0x43, 0xFF],
            [(0x20, 0x80), (0x60, 0x30), (0x40, 0x81)],
        ),
        (
            16,
            0x1E,
            [0x45, 0x44, 0x43],
            [(0x20, 0x80), (0x60, 0x30), (0x40, 0x56)],
        ),
        (
            24,
            0x02,
            [0xFF, 0x45, 0x44],
            [(0x20, 0x80), (0x60, 0x30), (0x40, 0x56)],
        ),
    ] {
        map.frame_counter = frame;
        tick(&mut map, div);
        for index in 0..3 {
            assert_eq!(
                map.zone_slots[index + 1],
                (positions[index].0, positions[index].1, frames[index])
            );
        }
        let stars = map.zone_slots[1..4].to_vec();
        map.frame_counter = frame + 1;
        tick(&mut map, 0x10);
        assert_eq!(&map.zone_slots[1..4], stars);
    }
}

#[test]
fn space_blink_random_mixes_swapped_div_accumulator_and_global_frame() {
    let mut map = space();
    map.frame_counter = 4;
    map.zone_animation.random_accum = 0xFE;
    map.zone_animation.global_frame = 0x37;
    tick(&mut map, 0x21);
    // FE + 12 + 21 + 37 = 168 -> 68, divisible by 8.
    assert_eq!(map.zone_animation.random_accum, 0x68);
    assert_eq!(map.zone_animation.space.blink_timer, 4);
    assert_eq!(map.zone_slots[0], (36, 40, 0x49));
    for (frame, expected) in [(8, 0x4B), (12, 0x4A), (16, 0x49), (20, 0x49)] {
        map.frame_counter = frame;
        tick(&mut map, 0xFF);
        assert_eq!(map.zone_slots[0].2, expected);
        assert_eq!(map.zone_animation.random_accum, 0x68);
    }
    assert_eq!(map.zone_animation.space.blink_timer, 0);
    map.frame_counter = 24;
    tick(&mut map, 0);
    assert_eq!(map.zone_animation.random_accum, 0x9F);
    assert_eq!(map.zone_animation.space.blink_timer, 0);
}

#[test]
fn space_drifter_wraps_its_coordinate_and_only_respawns_on_counter_wrap() {
    let mut map = space();
    map.zone_slots[6] = (0, 255, 0);
    map.frame_counter = 3;
    tick(&mut map, 0x55);
    assert_eq!(map.zone_slots[6], (255, 255, 0x46));
    map.frame_counter = 4;
    tick(&mut map, 0x55);
    assert_eq!(map.zone_slots[6], (0x55, 0, 0x46));
    map.frame_counter = 8;
    tick(&mut map, 0x66);
    assert_eq!(map.zone_slots[6], (0x54, 1, 0x47));
}

#[test]
fn space_launch_holds_even_frames_then_returns_and_stages_rom_patches() {
    let mut map = space();
    start_launch(&mut map);
    assert_eq!(
        (
            map.zone_animation.space.launch_state,
            map.zone_animation.space.launch_timer,
            map.zone_slots[9].2
        ),
        (1, 60, 117)
    );
    map.frame_counter = 2;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[7], (248, 144, 255));
    assert_eq!(map.zone_animation.space.launch_timer, 60);
    map.frame_counter = 5;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[7], (251, 141, 111));
    assert_eq!(map.zone_slots[8], (251, 141, 114));
    map.zone_animation.space.launch_timer = 1;
    map.frame_counter = 7;
    tick(&mut map, 0);
    assert_eq!(map.zone_animation.space.launch_state, 2);
    assert_eq!(map.zone_slots[7], (168, 14, 111));
    assert_eq!(map.zone_slots[8].2, 255);
    assert_eq!(map.zone_animation.space.launch_timer, 10);
    for frame in 8..17 {
        map.frame_counter = frame;
        tick(&mut map, 0);
    }
    assert_eq!(map.zone_slots[7], (150, 32, 111));
    assert_eq!(map.zone_animation.space.launch_timer, 1);
    map.frame_counter = 17;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[7], (156, 36, 112));
    assert_eq!(map.zone_animation.space.launch_state, 3);
    assert_eq!(map.zone_animation.space.launch_timer, 20);
    assert_eq!(&map.zone_animation.patch_ids[..2], &[56, 52]);
    assert_eq!(map.zone_animation.sfx0, 24);
}

#[test]
fn space_shake_clears_scroll_and_only_sounds_on_bit_one_at_timer_end() {
    let mut map = space();
    map.zone_animation.space.launch_state = 3;
    map.zone_animation.space.launch_timer = 20;
    map.frame_counter = 2;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[7], (140, 36, 255));
    assert_eq!(
        (map.zone_animation.scroll_x, map.zone_animation.scroll_y),
        (9, 246)
    );
    map.frame_counter = 4;
    tick(&mut map, 0);
    assert_eq!(
        (map.zone_animation.scroll_x, map.zone_animation.scroll_y),
        (0, 0)
    );
    map.zone_animation.space.launch_timer = 1;
    map.zone_animation.sfx0 = 0;
    tick(&mut map, 0);
    assert_eq!(map.zone_animation.sfx0, 0);
    map.zone_animation.space.launch_timer = 1;
    map.frame_counter = 2;
    tick(&mut map, 0);
    assert_eq!(
        (map.zone_animation.scroll_x, map.zone_animation.scroll_y),
        (0, 255)
    );
    assert_eq!(map.zone_animation.sfx0, 21);
}

#[test]
fn space_marker_stops_at_123_and_saved_path_resets_to_landed_state() {
    let mut map = space();
    map.zone_slots[9].2 = 122;
    map.frame_counter = 8;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[9].2, 123);
    assert_eq!(map.zone_animation.render_slots.unwrap()[9].2, 122);
    assert_eq!(map.zone_animation.sfx0, 6);
    map.zone_animation.sfx0 = 0;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[9].2, 123);
    assert_eq!(map.zone_animation.sfx0, 0);
    map.zone_animation.space.launch_timer = 99;
    reset(&mut map);
    assert_eq!(map.zone_animation.space.launch_timer, 99);
    map.level_progress[16] = 1;
    reset(&mut map);
    assert_eq!(map.zone_animation.space.launch_state, 3);
    assert_eq!(map.zone_animation.space.launch_timer, 0);
    assert_eq!(map.zone_slots[7].2, 112);
    assert_eq!(map.zone_slots[9].2, 123);
}
