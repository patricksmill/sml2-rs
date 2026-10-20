use super::*;

fn wario_castle() -> MapSim {
    let mut map = MapSim::default();
    map.screen_id = 7;
    // ROM $3E6AA's four stride-three decoration records.
    map.zone_slots[..4].copy_from_slice(&[
        (25, 32, 52),
        (80, 33, 53),
        (52, 28, 255),
        (112, 98, 57),
    ]);
    reset(&mut map);
    map
}

#[test]
fn wario_castle_rom_frame_tables_and_stride_three_aliases() {
    assert_eq!(STEP_FRAMES, [0, 1, 2, 1]);
    assert_eq!(BLINK_FRAMES, [0, 1, 0, 2]);
    assert_eq!(LIGHTNING_X, [0x30, 0x70, 0x60, 0x40]);
    let mut map = wario_castle();
    map.frame_counter = 7;
    tick(&mut map, 2);
    assert_eq!(map.zone_slots[1], (80, 33, 53));
    map.frame_counter = 8;
    tick(&mut map, 2);
    assert_eq!(map.zone_slots[1], (80, 33, 54));
    map.frame_counter = 16;
    tick(&mut map, 2);
    assert_eq!(map.zone_slots[1], (80, 33, 55));
    map.frame_counter = 24;
    tick(&mut map, 2);
    assert_eq!(map.zone_slots[1], (80, 33, 53));
    assert_eq!(map.zone_slots[0], (25, 32, 52));
    assert_eq!(map.zone_slots[2], (52, 28, 255));
}

#[test]
fn wario_castle_lightning_position_and_three_stage_palette_flash() {
    let mut map = wario_castle();
    map.frame_counter = 4;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[2], (0x70, 28, 56));
    assert_eq!(
        (
            map.zone_animation.wario_castle.flash,
            map.zone_animation.wario_castle.bg_palette,
            map.zone_animation.wario_castle.obj_palette
        ),
        (1, 0x1E, 0xC6)
    );
    map.frame_counter = 5;
    tick(&mut map, 2);
    assert_eq!(map.zone_slots[2].2, 56);
    map.frame_counter = 8;
    tick(&mut map, 2);
    assert_eq!(map.zone_slots[2].2, 255);
    assert_eq!(
        (
            map.zone_animation.wario_castle.flash,
            map.zone_animation.wario_castle.bg_palette,
            map.zone_animation.wario_castle.obj_palette
        ),
        (2, 0xE1, 0x39)
    );
    map.frame_counter = 12;
    tick(&mut map, 2);
    assert_eq!(
        (
            map.zone_animation.wario_castle.flash,
            map.zone_animation.wario_castle.bg_palette
        ),
        (0, 0x1E)
    );
    map.frame_counter = 16;
    tick(&mut map, 2);
    assert_eq!(map.zone_animation.wario_castle.bg_palette, 0xE1);
    for (phase, x) in [(2, 0x60), (3, 0x40), (0, 0x30)] {
        tick(&mut map, 0x81);
        assert_eq!(
            (map.zone_animation.wario_castle.phase, map.zone_slots[2].0),
            (phase, x)
        );
    }
}

#[test]
fn wario_castle_transition_freezes_only_palette_flash() {
    let mut map = wario_castle();
    map.zone_animation.transition_state = 1;
    map.zone_animation.wario_castle.flash = 1;
    map.frame_counter = 8;
    tick(&mut map, 0);
    assert_eq!(map.zone_slots[1].2, 54);
    assert_eq!(map.zone_slots[2].2, 56);
    assert_eq!(map.zone_animation.wario_castle.flash, 1);
    assert_eq!(map.zone_animation.wario_castle.bg_palette, 0xE1);
}

#[test]
fn wario_castle_wanderer_horizontal_cadence_frames_occlusion_and_edges() {
    let mut state = WarioCastleAnimation::default();
    state.movement = 1;
    let mut wanderer = (0x60, 0x50, 0);
    for (count, expected) in [
        (0, (0x60, 0x50, 57)),
        (1, (0x5F, 0x50, 255)),
        (2, (0x5F, 0x50, 255)),
        (3, (0x5E, 0x50, 255)),
    ] {
        state.accum = count;
        animate_wanderer(&mut state, &mut wanderer, 0, 0);
        assert_eq!(wanderer, expected);
    }
    state.movement = 2;
    state.accum = 2;
    wanderer = (0x3F, 0x4F, 0);
    animate_wanderer(&mut state, &mut wanderer, 0, 0);
    assert_eq!(wanderer, (0x3F, 0x4F, 62));
    state.accum = 3;
    animate_wanderer(&mut state, &mut wanderer, 0, 0);
    assert_eq!(wanderer, (0x40, 0x4F, 61));
    wanderer = (0x78, 0x50, 0);
    animate_wanderer(&mut state, &mut wanderer, 3, 0);
    assert_eq!(wanderer, (0x78, 0x50, 61));
    assert_eq!(state.movement, 3);
    assert_eq!(state.blink_timer, 30);
}

#[test]
fn wario_castle_wanderer_vertical_endpoint_preserves_asymmetric_timer_write() {
    let mut state = WarioCastleAnimation::default();
    for movement in [3, 4] {
        state.movement = movement;
        let mut wanderer = (0x32, if movement == 3 { 0x36 } else { 0x61 }, 0);
        animate_wanderer(&mut state, &mut wanderer, 0, 0);
        assert_eq!(
            wanderer,
            (0x32, if movement == 3 { 0x32 } else { 0x65 }, 255)
        );
        state.blink_timer = 9;
        animate_wanderer(&mut state, &mut wanderer, 0, 0);
        assert_eq!((state.movement, state.blink_timer, wanderer.2), (1, 9, 63));
        state.movement = movement;
        wanderer.0 = 0x31;
        animate_wanderer(&mut state, &mut wanderer, 0, 0);
        assert_eq!((state.movement, state.blink_timer, wanderer.2), (2, 30, 63));
    }
}

#[test]
fn wario_castle_blink_gate_accumulator_wrap_and_random_rejection() {
    let mut map = wario_castle();
    map.zone_animation.wario_castle.accum = 255;
    map.frame_counter = 1;
    tick(&mut map, 3);
    assert_eq!(map.zone_animation.wario_castle.accum, 255);
    map.frame_counter = 2;
    tick(&mut map, 3);
    assert_eq!(map.zone_animation.wario_castle.accum, 0);
    assert_eq!(
        (map.zone_animation.wario_castle.blink_timer, map.zone_slots[3].2),
        (0, 63)
    );
    tick(&mut map, 3);
    assert_eq!(
        (
            map.zone_animation.wario_castle.movement,
            map.zone_animation.wario_castle.blink_timer
        ),
        (3, 30)
    );
    let mut state = WarioCastleAnimation::default();
    for (previous, sample, expected) in [(0, 0, 3), (1, 5, 3), (3, 7, 0), (3, 5, 1), (4, 6, 2)] {
        state.movement = previous;
        choose_movement(&mut state, sample, 0);
        assert_eq!(
            state.movement, expected,
            "previous {previous}, sample {sample}"
        );
    }
    for (accum, id) in [(0, 63), (4, 64), (8, 63), (12, 65)] {
        state.movement = 0;
        state.blink_timer = 2;
        state.accum = accum;
        let mut wanderer = (0, 0, 0);
        animate_wanderer(&mut state, &mut wanderer, 0, 0);
        assert_eq!(wanderer.2, id);
    }
}

#[test]
fn wario_castle_raster_bands_have_distinct_cadence_and_wrap() {
    let mut state = WarioCastleAnimation::default();
    for frame in 1..=16 {
        update_bands(&mut state);
        assert_eq!(
            state.band_scroll,
            [
                0u8.wrapping_sub(frame / 4),
                23 - frame / 8,
                0u8.wrapping_sub(frame / 16)
            ]
        );
    }
    state.raster_phase = 255;
    state.band_scroll = [0, 0, 0];
    update_bands(&mut state);
    assert_eq!(state.raster_phase, 0);
    assert_eq!(state.band_scroll, [255, 23, 255]);
}

#[test]
fn wario_castle_loader_resets_only_source_fields() {
    let mut map = wario_castle();
    let state = &mut map.zone_animation.wario_castle;
    state.anim_count = 2;
    state.phase = 3;
    state.flash = 2;
    state.accum = 255;
    state.movement = 4;
    state.blink_timer = 30;
    state.raster_phase = 20;
    state.band_scroll = [9, 4, 8];
    state.bg_palette = 0;
    state.obj_palette = 0;
    reset(&mut map);
    let state = &map.zone_animation.wario_castle;
    assert_eq!(
        (state.anim_count, state.phase, state.flash, state.accum),
        (2, 3, 2, 255)
    );
    assert_eq!(
        (state.movement, state.blink_timer, state.raster_phase),
        (0, 1, 20)
    );
    assert_eq!(state.band_scroll, [9, 23, 8]);
    assert_eq!((state.bg_palette, state.obj_palette), (0xE1, 0x39));
}
