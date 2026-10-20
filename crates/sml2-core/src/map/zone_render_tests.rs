use super::*;

fn render(screen: u8, cells: &[(u8, u8, u8, u8)], manifest: &OverworldManifest) -> Vec<u8> {
    let maps = OverworldTilemaps::from_bytes(&vec![0; OverworldTilemaps::TOTAL_LEN]).unwrap();
    let tiles = vec![[1; 64]; 384];
    render_overworld_view(
        screen, 0, 0, 0, 0, 0, 0, 0, 0, cells, None, false, 0, &maps, manifest, &[0; 64], &tiles,
        0xE4, 0xD0, 160, 144,
    )
}

#[test]
fn zones_keep_their_bottom_background_and_use_the_sprite_palette_bit() {
    for screen in 1..=8 {
        let frame = render(
            screen,
            &[(100, 40, 0x30, 0x10)],
            &OverworldManifest::default(),
        );
        assert_eq!(frame[84 * 160 + 32], 85, "OBP1 in zone {screen}");
        assert_eq!(
            frame[136 * 160 + 100],
            170,
            "zone {screen} has no hub strip"
        );
        let frame = render(
            screen,
            &[(100, 40, 0x30, 0x80)],
            &OverworldManifest::default(),
        );
        assert_eq!(frame[84 * 160 + 32], 170, "priority sprite behind zone BG");
    }
}

#[test]
fn zero_mask_route_patches_are_initial_screen_content() {
    let manifest = OverworldManifest {
        patches: vec![PatchStream {
            id: 0,
            ptr: "$0000".into(),
            tiles: vec![PatchTile {
                dx: 0,
                dy: 0,
                tile: 1,
            }],
        }],
        dynamic_unlocks: vec![DynamicUnlock {
            name: "initial route".into(),
            screen_id: 6,
            progress_offset: 0,
            mask: 0,
            patches: vec![PatchPlacement {
                x: 4,
                y: 7,
                patch_ids: vec![0],
            }],
        }],
        ..Default::default()
    };
    let maps = OverworldTilemaps::from_bytes(&vec![0; OverworldTilemaps::TOTAL_LEN]).unwrap();
    let mut tiles = vec![[0; 64]; 384];
    tiles[signed_bg_tile_index(1)] = [2; 64];
    let frame = render_overworld_view(
        6,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        &[],
        None,
        false,
        0,
        &maps,
        &manifest,
        &[0; 64],
        &tiles,
        0xE4,
        0xD0,
        160,
        144,
    );
    assert_eq!(frame[56 * 160 + 32], 85);
    assert_eq!(frame[56 * 160 + 24], 255);
}
