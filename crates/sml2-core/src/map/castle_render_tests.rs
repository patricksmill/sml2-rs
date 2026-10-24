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
fn enlarged_castle_continues_rom_bricks_and_floor_and_preserves_every_native_pixel() {
    let (reveal, scripts, metas) = assets();
    assert_eq!(&reveal.tilemap[..2], &[0x70, 0x71]);
    assert_eq!(&reveal.tilemap[14 * 32..14 * 32 + 2], &[0x72, 0x73]);
    assert_eq!(reveal.tilemap[15 * 32], 0x66);
    let seq = CastleSequence {
        coins: [0x80; 6],
        ..Default::default()
    };
    let snapshot = seq.clone();
    for output in [
        crate::video::OutputMode::Grayscale,
        crate::video::OutputMode::PaletteIndices,
    ] {
        let native =
            render_castle_sequence_output(&reveal, &scripts, &metas, &seq, (160, 144), output);
        for (w, h) in [(256, 144), (384, 216), (385, 217)] {
            let wide =
                render_castle_sequence_output(&reveal, &scripts, &metas, &seq, (w, h), output);
            let (mx, my) = ((w - 160) / 2, (h - 144) / 2);
            for y in 0..144 {
                assert_eq!(
                    &wide[(y + my) * w + mx..(y + my) * w + mx + 160],
                    &native[y * 160..(y + 1) * 160]
                );
            }
            assert_eq!(
                wide[my * w + mx - 1],
                native[15],
                "wall checker continues left"
            );
            assert_eq!(
                wide[(my + 112) * w + mx - 1],
                native[112 * 160 + 15],
                "wall base continues left"
            );
            if my > 0 {
                assert_eq!(
                    wide[(my - 1) * w + mx],
                    native[7 * 160 + 8],
                    "wall continues above"
                );
                assert_eq!(
                    wide[(my + 144) * w],
                    native[120 * 160],
                    "ground continues below"
                );
            }
        }
    }
    assert_eq!(seq, snapshot);
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
        crate::video::OutputMode::Grayscale,
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

#[test]
fn castle_palette_frame_retains_both_object_sources_and_background() {
    use crate::video::{OutputMode, PalettePixel};
    let mut bytes = vec![0; SpriteMetas::ENTRIES * 2];
    bytes[14..16].copy_from_slice(&0x41E4u16.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 8, 0, 0x10, 0x80, 0, 0, 0]);
    let metas = SpriteMetas::new(&bytes);
    let tokens = draw(
        &vec![[1; 64]; 384],
        &vec![0; 0x300],
        0xE4,
        [0xE4; 2],
        &[(8, 16, 7)],
        &metas,
        (160, 144),
        OutputMode::PaletteIndices,
    );
    assert_eq!((tokens[0], tokens[8], tokens[16]), (5, 9, 1));
    let gray = draw(
        &vec![[1; 64]; 384],
        &vec![0; 0x300],
        0xE4,
        [0xE4; 2],
        &[(8, 16, 7)],
        &metas,
        (160, 144),
        OutputMode::Grayscale,
    );
    assert_eq!(
        tokens
            .iter()
            .map(|&p| PalettePixel::from_token(p).grayscale())
            .collect::<Vec<_>>(),
        gray
    );
    let (reveal, scripts, metas) = assets();
    let mut seq = CastleSequence::default();
    for state in [STATE_MENU, STATE_WAIT_START] {
        seq.state = state;
        let tokens = render_castle_sequence_output(
            &reveal,
            &scripts,
            &metas,
            &seq,
            (160, 144),
            OutputMode::PaletteIndices,
        );
        assert_eq!(
            tokens
                .iter()
                .map(|&p| PalettePixel::from_token(p).grayscale())
                .collect::<Vec<_>>(),
            render_castle_sequence(&reveal, &scripts, &metas, &seq, (160, 144))
        );
    }
}

fn game_over(fade_timer: u16, golden_coin_flags: u8) -> Vec<u8> {
    let (reveal, _, metas) = assets();
    render_game_over_output(
        &reveal,
        &metas,
        GameOverView {
            fade_timer,
            frame_counter: 0,
            golden_coin_flags,
            obp: [0xD0, 0xE4],
        },
        (160, 144),
        crate::video::OutputMode::Grayscale,
    )
}

/// `$30884`: a collected coin sits at its castle spot while the fade is at
/// least 128, then rises off the top; at fade 0 it has left the screen.
#[test]
fn game_over_coins_hold_then_rise_off_the_screen() {
    assert_ne!(game_over(200, 0b10_0000), game_over(200, 0), "coin 1 drawn");
    assert_ne!(
        game_over(100, 0b10_0000),
        game_over(200, 0b10_0000),
        "rising"
    );
    assert_eq!(game_over(0, 0b10_0000), game_over(0, 0), "gone at fade 0");
}

/// `$28A7`: Mario slides down while the fade is above 191, then stays at y 32.
#[test]
fn game_over_mario_slides_in_during_the_first_64_frames() {
    assert_ne!(game_over(255, 0), game_over(200, 0));
    assert_ne!(game_over(200, 0), game_over(192, 0));
    assert_eq!(game_over(192, 0), game_over(100, 0));
}

/// The exact `$30884`/`$28A7` sprite calls: coin 1 at its spot, then one
/// pixel up on the spin frame once the fade drops to 127; Mario from y 1 at
/// fade 255, alternating frames every 16 `$FF97` ticks.
#[test]
fn game_over_sprite_calls_match_the_rom_tables() {
    let view = |fade_timer, frame_counter| GameOverView {
        fade_timer,
        frame_counter,
        golden_coin_flags: 0b10_0001,
        obp: [0, 0],
    };
    assert_eq!(
        game_over_sprites(view(255, 0)),
        vec![(59, 121, 0xC4), (71, 70, 0xD9), (68, 1, 227)]
    );
    assert_eq!(
        game_over_sprites(view(127, 0x10)),
        vec![(59, 120, 225), (71, 69, 225), (68, 32, 228)]
    );
}
