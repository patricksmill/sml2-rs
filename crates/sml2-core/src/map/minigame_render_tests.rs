use super::*;

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
