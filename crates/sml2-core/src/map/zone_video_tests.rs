use super::*;

#[test]
fn zone_video_scrolls_background_bands_without_moving_the_sprites() {
    let mut raw = vec![0; OverworldTilemaps::TOTAL_LEN];
    let base = OverworldTilemaps::HUB_LEN + 6 * 1024;
    for row in 0..32 {
        for col in 0..32 {
            raw[base + row * 32 + col] = (col & 1) as u8;
        }
    }
    let maps = OverworldTilemaps::from_bytes(&raw).unwrap();
    let manifest = OverworldManifest::default();
    let mut tiles = vec![[0; 64]; 384];
    tiles[signed_bg_tile_index(0)] = [1; 64];
    tiles[signed_bg_tile_index(1)] = [2; 64];
    tiles[0x30] = [1; 64];
    let mut video = MapVideo::default();
    video.scroll_rows[..15].fill(8);
    video.obj_palette1 = 0xC6;
    let frame = render_overworld_view_with_video(
        7,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        &[(100, 40, 0x30, 0x10)],
        None,
        false,
        0,
        &maps,
        &manifest,
        &[0; 42],
        &tiles,
        0xE4,
        0xD0,
        160,
        144,
        Some(&video),
    );
    assert_eq!(frame[0], 85, "top band scrolls eight pixels");
    assert_eq!(frame[15 * 160], 170, "lower band keeps its own scroll");
    assert_eq!(
        frame[84 * 160 + 32],
        170,
        "sprite stays anchored and uses live OBP1"
    );
    video.obj_palette1 = 0x39;
    let frame = render_overworld_view_with_video(
        7,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        &[(100, 40, 0x30, 0x10)],
        None,
        false,
        0,
        &maps,
        &manifest,
        &[0; 42],
        &tiles,
        0xE4,
        0xD0,
        160,
        144,
        Some(&video),
    );
    assert_eq!(
        frame[84 * 160 + 32],
        85,
        "normal OBP1 restores sprite shade"
    );
}

#[test]
fn zone_tile_writes_change_only_the_current_screen_copy() {
    let maps = OverworldTilemaps::from_bytes(&vec![0; OverworldTilemaps::TOTAL_LEN]).unwrap();
    let live = maps.with_screen_writes(6, &[(7 * 32 + 4, 0xA5), (1024, 0xFF)]);
    assert_eq!(live.screen_tile(6, 4, 7), 0xA5);
    assert_eq!(maps.screen_tile(6, 4, 7), 0);
    assert_eq!(live.screen_tile(5, 4, 7), 0);
    assert_eq!(live.hub_tile(4, 7), 0);
}

#[test]
fn palette_sources_survive_all_map_room_paths_and_status_strip_composition() {
    use crate::video::{OutputMode, PalettePixel};
    let maps = OverworldTilemaps::from_bytes(&vec![0; OverworldTilemaps::TOTAL_LEN]).unwrap();
    let manifest = OverworldManifest::default();
    let mut tiles = vec![[0; 64]; 384];
    tiles[signed_bg_tile_index(0)] = [1; 64];
    tiles[1] = [1; 64];
    let video = MapVideo {
        obj_palette1: 0xE4,
        ..Default::default()
    };
    let cells = [
        (32, 24, 1, 0),
        (32, 40, 1, 0x10),
        (32, 56, 0, 0x10),
        (32, 72, 1, 0x80),
    ];
    for screen in 0..=14 {
        let draw = |cells: &[(u8, u8, u8, u8)], output| {
            render_overworld_view_output(
                screen,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                cells,
                None,
                false,
                0,
                &maps,
                &manifest,
                &[0; 42],
                &tiles,
                0xE4,
                0xE4,
                160,
                144,
                Some(&video),
                output,
            )
        };
        let tokens = draw(&cells, OutputMode::PaletteIndices);
        assert_eq!(tokens[16 * 160 + 16], 5, "screen {screen}: OBP0 shade 1");
        assert_eq!(tokens[16 * 160 + 32], 9, "screen {screen}: OBP1 shade 1");
        assert_eq!(
            tokens[16 * 160 + 48],
            1,
            "screen {screen}: transparent OBJ preserves BG"
        );
        assert_eq!(
            tokens[16 * 160 + 64],
            1,
            "screen {screen}: BG priority preserves source"
        );
        assert_eq!(
            tokens[16 * 160 + 80],
            1,
            "screen {screen}: BG has the same shade as objects"
        );
        let gray = draw(&cells, OutputMode::Grayscale);
        assert_eq!(
            tokens
                .iter()
                .map(|&p| PalettePixel::from_token(p).grayscale())
                .collect::<Vec<_>>(),
            gray
        );
        let next = draw(&[], OutputMode::PaletteIndices);
        assert_eq!(
            next[16 * 160 + 32],
            1,
            "screen {screen}: source resets with each frame"
        );
        if screen == 0 {
            assert_eq!(
                tokens[136 * 160 + 80],
                0,
                "hub HUD strip restores mapped BG colour zero"
            );
        }
    }
}
