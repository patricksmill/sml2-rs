use super::*;

#[test]
fn zone_irq_tiles_use_rom_phase_sequences_and_fourteen_frame_cadence() {
    let mut map = MapSim::default();
    map.screen_id = 1;
    map.level_progress[39] = 0x80;
    reset(&mut map);
    for _ in 0..13 {
        tick(&mut map, 0);
    }
    assert_eq!(map.zone_animation.tiles.pumpkin_frame, None);
    assert_eq!(map.zone_animation.tiles.irq_divider, 13);
    tick(&mut map, 0);
    assert_eq!(map.zone_animation.tiles.pumpkin_frame, Some(4));
    assert_eq!(map.zone_animation.tiles.patch_index, 1);
    for (index, frame) in [(2, 5), (3, 5), (4, 6), (5, 6), (6, 7), (7, 7), (8, 4)] {
        for _ in 0..14 {
            tick(&mut map, 0);
        }
        assert_eq!(
            (
                map.zone_animation.tiles.patch_index,
                map.zone_animation.tiles.pumpkin_frame
            ),
            (index, Some(frame))
        );
    }
    map.screen_id = 2;
    map.level_progress[36] = 0x80;
    reset(&mut map);
    for frame in [1, 2, 3, 3, 3, 3, 3, 0] {
        for _ in 0..14 {
            tick(&mut map, 0);
        }
        assert_eq!(map.zone_animation.tiles.tree_frame, Some(frame));
    }
}

#[test]
fn zone_irq_tiles_gate_on_correct_saved_progress_high_bits_and_retain_writes() {
    let mut map = MapSim::default();
    for (screen, gate) in [(1, 39), (2, 36)] {
        map.screen_id = screen;
        reset(&mut map);
        map.level_progress.fill(0x7F);
        for _ in 0..20 {
            tick(&mut map, 0);
        }
        assert_eq!(map.zone_animation.tiles.irq_divider, 0);
        assert_eq!(map.zone_animation.tiles.patch_index, 0);
        map.level_progress[gate] = 0x80;
        for _ in 0..14 {
            tick(&mut map, 0);
        }
        map.level_progress[gate] = 0;
        for _ in 0..40 {
            tick(&mut map, 0);
        }
        assert_eq!(map.zone_animation.tiles.irq_divider, 0);
        assert_eq!(map.zone_animation.tiles.patch_index, 1);
        assert_eq!(
            if screen == 1 {
                map.zone_animation.tiles.pumpkin_frame
            } else {
                map.zone_animation.tiles.tree_frame
            },
            Some(if screen == 1 { 4 } else { 1 })
        );
    }
}

#[test]
fn zone_irq_patch_counter_wraps_before_rom_lookup() {
    let mut map = MapSim::default();
    map.screen_id = 1;
    map.level_progress[39] = 0x80;
    map.zone_animation.tiles.irq_divider = 13;
    map.zone_animation.tiles.patch_index = 255;
    tick(&mut map, 0);
    assert_eq!(
        (
            map.zone_animation.tiles.patch_index,
            map.zone_animation.tiles.pumpkin_frame
        ),
        (0, Some(4))
    );
    // $3ECB5's exceptional tail plays the four-frame gate animation.
    for (index, frame) in [
        (31, 8),
        (32, 11),
        (54, 8),
        (55, 0),
        (56, 1),
        (57, 2),
        (58, 3),
        (59, 0),
        (63, 4),
    ] {
        map.zone_animation.tiles.irq_divider = 13;
        map.zone_animation.tiles.patch_index = index;
        tick(&mut map, 0);
        assert_eq!(map.zone_animation.tiles.pumpkin_frame, Some(frame));
    }
}

#[test]
fn turtle_irq_skips_sixteenth_frame_and_retains_previous_tile_pixels() {
    let mut map = MapSim::default();
    map.screen_id = 5;
    reset(&mut map);
    tick(&mut map, 0);
    assert_eq!(map.zone_animation.tiles.turtle_frame, None);
    for (frame, phase) in [
        (1, 0),
        (15, 0),
        (16, 0),
        (17, 1),
        (31, 1),
        (32, 1),
        (33, 2),
        (47, 2),
        (48, 2),
        (49, 3),
        (63, 3),
        (64, 3),
        (65, 0),
        (255, 3),
        (0, 3),
    ] {
        map.frame_counter = frame;
        tick(&mut map, 0);
        assert_eq!(
            map.zone_animation.tiles.turtle_frame,
            Some(phase),
            "frame {frame}"
        );
    }
}

#[test]
fn zone_irq_tile_writes_decode_planes_and_target_only_exact_vram_ranges() {
    for (screen, offset, first, second) in [
        (1, 4 * 64, 142, Some(158)),
        (2, 768 + 64, 130, Some(146)),
        (5, 1024 + 32, 256, None),
    ] {
        let mut state = TileAnimation::default();
        state.pumpkin_frame = Some(4);
        state.tree_frame = Some(1);
        state.turtle_frame = Some(1);
        let mut source = vec![0; SOURCE_LEN];
        source[offset] = 0xA5;
        source[offset + 1] = 0x3C;
        source[offset + 16..offset + 32].fill(0xFF);
        if second.is_some() {
            for row in 0..8 {
                source[offset + 32 + row * 2] = 0xFF;
                source[offset + 48 + row * 2 + 1] = 0xFF;
            }
        }
        let mut tiles = vec![[9; 64]; 384];
        assert!(state.apply(screen, &mut tiles, &source));
        assert_eq!(&tiles[first][..8], &[1, 0, 3, 2, 2, 3, 0, 1]);
        assert_eq!(&tiles[first][8..], &[0; 56]);
        assert_eq!(tiles[first + 1], [3; 64]);
        if let Some(second) = second {
            assert_eq!(tiles[second], [1; 64]);
            assert_eq!(tiles[second + 1], [2; 64]);
        }
        for (index, tile) in tiles.iter().enumerate() {
            if index != first
                && index != first + 1
                && second != Some(index)
                && second.map(|v| v + 1) != Some(index)
            {
                assert_eq!(*tile, [9; 64], "screen {screen} unexpected tile {index}");
            }
        }
    }
}

#[test]
fn zone_irq_missing_assets_and_uninitialized_phases_leave_base_graphics() {
    let state = TileAnimation::default();
    let mut tiles = vec![[9; 64]; 384];
    assert!(!state.apply(1, &mut tiles, &[]));
    assert!(state.apply(1, &mut tiles, &[0; SOURCE_LEN]));
    assert!(tiles.iter().all(|tile| *tile == [9; 64]));
    assert!(!state.apply(1, &mut tiles[..100], &[0; SOURCE_LEN]));
}

#[test]
fn zone_irq_loader_resets_counters_for_pumpkin_tree_and_restores_base_tiles() {
    let mut map = MapSim::default();
    for screen in [1, 2, 5] {
        map.screen_id = screen;
        map.zone_animation.tiles.irq_divider = 7;
        map.zone_animation.tiles.patch_index = 8;
        map.zone_animation.tiles.pumpkin_frame = Some(3);
        map.zone_animation.tiles.tree_frame = Some(2);
        map.zone_animation.tiles.turtle_frame = Some(1);
        reset(&mut map);
        assert_eq!(map.zone_animation.tiles.pumpkin_frame, None);
        assert_eq!(map.zone_animation.tiles.tree_frame, None);
        assert_eq!(map.zone_animation.tiles.turtle_frame, None);
        assert_eq!(
            (
                map.zone_animation.tiles.irq_divider,
                map.zone_animation.tiles.patch_index
            ),
            if screen == 5 { (7, 8) } else { (0, 0) }
        );
    }
}
