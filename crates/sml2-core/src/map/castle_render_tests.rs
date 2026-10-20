use super::*;

fn assets() -> (CastleRevealTiles, CastleScripts, SpriteMetas) {
    let reveal = CastleRevealTiles::from_bytes(
        include_bytes!("../../../../extracted/map/castle_reveal_tiles.bin"),
        include_bytes!("../../../../extracted/map/castle_reveal_tilemap.bin"),
    )
    .unwrap();
    let scripts = CastleScripts::from_parts(
        include_bytes!("../../../../extracted/map/castle_scripts.bin").to_vec(),
        include_str!("../../../../extracted/map/castle_scripts.json"),
    )
    .unwrap();
    let metas = SpriteMetas::new(scripts.blob("sprite_metasprites").unwrap());
    (reveal, scripts, metas)
}

#[test]
fn all_five_door_windows_match_the_rom_and_leave_other_tiles_alone() {
    let (reveal, scripts, _) = assets();
    let windows = scripts.blob("castle_row_windows").unwrap();
    assert_eq!(
        windows,
        &[
            0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x74,
            0x74, 0x74, 0x74, 0x74, 0x74, 0x74, 0x56, 0x57, 0x58, 0x59, 0x46, 0x47, 0x48, 0x49,
            0x36, 0x37, 0x38, 0x39, 0x26, 0x27, 0x28, 0x29, 0x16, 0x17, 0x18, 0x19,
        ]
    );
    for rows in 1..=5 {
        let map = door_tilemap(&reveal, rows, windows);
        for (index, (&before, &after)) in reveal.tilemap.iter().zip(&map).enumerate() {
            if !CastleRevealTiles::COIN_RUNS
                .iter()
                .any(|&run| (run..run + 4).contains(&index))
            {
                assert_eq!(before, after, "only five four-tile runs are transferred");
            }
        }
        assert_eq!(&map[0x1C8..0x1CC], &[0x74; 4], "bottom run clears first");
        if rows < 5 {
            let top = [0x26, 0x36, 0x46, 0x56][rows as usize - 1];
            assert_eq!(&map[0x148..0x14C], &[top, top + 1, top + 2, top + 3]);
        } else {
            assert_eq!(&map[0x148..0x14C], &[0x74; 4]);
        }
    }
}

#[test]
fn row_transfer_changes_visible_pixels_once_and_rendering_is_read_only() {
    let (reveal, scripts, metas) = assets();
    let mut seq = CastleSequence::default();
    seq.state = STATE_MENU;
    seq.menu = 2;
    seq.row_shift_count = 5;
    let closed = render_castle_sequence(&reveal, &scripts, &metas, &seq, (160, 144));
    seq.tick(Default::default(), 0, &mut [0; 42]);
    assert_eq!(seq.row_shift_index, 5);
    // Source VBlank consumes the transfer before the next logic tick.
    seq.tick(Default::default(), 0, &mut [0; 42]);
    let snapshot = seq.clone();
    let shifted = render_castle_sequence(&reveal, &scripts, &metas, &seq, (160, 144));
    assert_eq!(seq, snapshot);
    assert_eq!(
        shifted,
        render_castle_sequence(&reveal, &scripts, &metas, &seq, (160, 144))
    );
    assert_ne!(closed, shifted);
    for (index, (a, b)) in closed.iter().zip(&shifted).enumerate() {
        if a != b {
            assert!((64..96).contains(&(index % 160)) && (80..120).contains(&(index / 160)));
        }
    }
}

#[test]
fn explicit_white_and_black_palettes_are_not_replaced_with_defaults() {
    let (reveal, scripts, metas) = assets();
    let mut seq = CastleSequence::default();
    for palette in [0, 255] {
        seq.bgp = palette;
        seq.obp1 = palette;
        seq.obp2 = palette;
        let image = render_castle_sequence(&reveal, &scripts, &metas, &seq, (160, 144));
        assert!(image.iter().all(|&pixel| pixel == 255 - palette));
    }
}

#[test]
fn object_palette_bit_selects_the_second_hardware_palette() {
    let mut bytes = vec![0; SpriteMetas::ENTRIES * 2];
    bytes[14..16].copy_from_slice(&0x41E4u16.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0x10, 0x80, 0, 0, 0]);
    let metas = SpriteMetas::new(&bytes);
    let image = draw(
        &vec![[1; 64]; 384],
        &vec![0; 0x300],
        0xE4,
        [0, 255],
        &[(8, 16, 7)],
        &metas,
        (160, 144),
    );
    assert_eq!(
        (image[0], image[8]),
        (0, 170),
        "OBP1 sprite over an OBP0-independent background"
    );
}

#[test]
fn congratulations_uses_its_own_tilemap_and_hides_gameplay_sprites() {
    let (reveal, scripts, metas) = assets();
    let mut seq = CastleSequence::default();
    let castle = render_castle_sequence(&reveal, &scripts, &metas, &seq, (160, 144));
    seq.state = STATE_WAIT_START;
    let image = render_castle_sequence(&reveal, &scripts, &metas, &seq, (160, 144));
    seq.coins = [0x80; 6];
    seq.walk_x = 40;
    seq.walk_y = 40;
    assert_eq!(
        image,
        render_castle_sequence(&reveal, &scripts, &metas, &seq, (160, 144))
    );
    assert_ne!(castle, image);
    assert!(image.iter().any(|&p| p != image[0]));
}
