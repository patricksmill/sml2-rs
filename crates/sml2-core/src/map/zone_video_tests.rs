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
