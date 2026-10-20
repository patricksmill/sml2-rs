use super::*;

#[test]
fn room_objects_use_hardware_priority_and_both_palettes() {
    let mut tiles = vec![[0; 64]; 3];
    tiles[1] = [1; 64];
    tiles[2] = [2; 64];
    let mut frame = vec![255; 16 * 8];
    // Smaller X wins even when it is later in OAM; bit 4 chooses $39.
    draw_room_objects(
        &mut frame,
        &[0; 128],
        &tiles,
        &[(16, 12, 2, 0), (16, 8, 1, 0x10)],
        0xD0,
        16,
        8,
    );
    assert_eq!(frame[4], 85);
    assert_eq!(frame[8], 170);
    // An equal-X later entry loses, including when the winner is behind BG.
    frame.fill(255);
    draw_room_objects(
        &mut frame,
        &[1; 128],
        &tiles,
        &[(16, 8, 1, 0x80), (16, 8, 2, 0)],
        0xD0,
        16,
        8,
    );
    assert_eq!(frame, vec![255; 128]);
    frame.fill(255);
    draw_room_objects(
        &mut frame,
        &[0; 128],
        &tiles,
        &[(16, 8, 1, 0), (16, 8, 2, 0)],
        0xE4,
        16,
        8,
    );
    assert_eq!(frame[0], 170);
}

#[test]
fn bonus_rooms_draw_the_background_without_the_map_cursor_or_status_strip() {
    let maps =
        OverworldTilemaps::from_bytes(include_bytes!("../../../../extracted/map/tilemaps.bin"))
            .unwrap();
    let manifest: OverworldManifest =
        serde_json::from_str(include_str!("../../../../extracted/map/route_patches.json")).unwrap();
    // Distinct solid OBJ and BG colors make an accidental cursor/strip visible.
    let tiles = vec![[1; 64]; 384];
    for screen in 9..=14 {
        let frame = render_overworld_view(
            screen,
            0,
            0,
            80,
            64,
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
            0x0C,
            160,
            144,
        );
        assert_eq!(frame, vec![170; 160 * 144], "room {screen}");
        let with_room_sprite = render_overworld_view(
            screen,
            0,
            0,
            80,
            64,
            0,
            0,
            0,
            0,
            &[(100, 40, 0x90, 0)],
            None,
            false,
            0,
            &maps,
            &manifest,
            &[0; 64],
            &tiles,
            0xE4,
            0x0C,
            160,
            144,
        );
        assert_eq!(
            with_room_sprite[84 * 160 + 32],
            0,
            "room sprite still draws"
        );
        assert_eq!(with_room_sprite[136 * 160 + 100], 170, "no hub strip");
    }
}
