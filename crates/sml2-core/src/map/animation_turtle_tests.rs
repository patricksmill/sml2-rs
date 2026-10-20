use super::*;

#[test]
fn turtle_animation_rom_frames_and_signed_drift_bytes() {
    let mut map = MapSim::new();
    reset(&mut map);
    map.frame_counter = 0x73;
    tick(&mut map, 0);
    assert_eq!(
        [
            map.zone_slots[1].2,
            map.zone_slots[2].2,
            map.zone_slots[3].2
        ],
        [45; 3]
    );
    assert_eq!(map.zone_slots[7], (0, 52, 49));
    assert_eq!(map.zone_slots[8], (0, 52, 49));
    assert_eq!(map.zone_slots[0].2, 41);
    assert_eq!(map.zone_slots[9], (159, 0, 50));
    map.frame_counter = 2;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[9].0, 159, "low bits 10 do not advance");
    map.frame_counter = 1;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[9].0, 159, "low bits 01 do not advance");
}

#[test]
fn turtle_moving_sprite_wraps_and_reverses_at_source_edges() {
    let mut map = MapSim::new();
    map.frame_counter = 3;
    map.zone_animation.moving_x = 0;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[9].0, 255);
    map.zone_animation.moving_x = 193;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[9], (192, 120, 50));
    assert_eq!(map.zone_animation.moving_direction, 255);
    map.zone_animation.moving_x = 175;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[9], (176, 64, 51));
    assert_eq!(map.zone_animation.moving_direction, 0);
    map.zone_animation.moving_direction = 255;
    reset(&mut map);
    assert_eq!(
        (
            map.zone_animation.moving_x,
            map.zone_animation.moving_direction
        ),
        (160, 255)
    );
}

#[test]
fn turtle_gate_patch_random_and_secret_growth_rom_bytes() {
    let mut map = MapSim::new();
    map.zone_slots[0].2 = 255;
    map.level_progress[37] = 0x80;
    map.zone_animation.random_accum = 250;
    map.zone_animation.global_frame = 7;
    map.zone_slots[10].2 = 127;
    map.frame_counter = 16;
    tick(&mut map, 0x12); // 250 + $21 + $12 + 7 = $34
    assert_eq!(map.zone_animation.random_accum, 52);
    assert_eq!(map.zone_animation.patch_ids[0], 53);
    assert_eq!(map.zone_slots[0].2, 255);
    assert_eq!((map.zone_slots[10].2, map.zone_animation.sfx0), (128, 6));
    map.frame_counter = 17;
    tick(&mut map, 0xFF);
    assert_eq!(map.zone_animation.random_accum, 52);
    assert_eq!(map.zone_slots[10].2, 128);
    map.frame_counter = 32;
    tick(&mut map, 0x11); // $34 + $11 + $11 + 7 = $5D
    assert_eq!(map.zone_animation.patch_ids[0], 54);
    assert_eq!((map.zone_slots[10].2, map.zone_animation.sfx0), (129, 21));
    map.frame_counter = 48;
    tick(&mut map, 0);
    assert_eq!((map.zone_slots[10].2, map.zone_animation.sfx0), (129, 21));
}
