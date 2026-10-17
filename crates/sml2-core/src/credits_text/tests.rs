use super::*;

#[test]
fn test_char_to_tile_mapping() {
    // ASCII space (0x20 - 48 = 0xF0) maps to 0xFF (blank tile)
    assert_eq!(char_to_tile(b' '), BLANK_TILE);
    assert_eq!(char_to_tile(0x20), 0xFF);

    // Digits '0'..'9' (0x30..0x39 - 48 = 0..9)
    assert_eq!(char_to_tile(b'0'), 0);
    assert_eq!(char_to_tile(b'9'), 9);

    // Uppercase letters 'A'..'Z' (0x41..0x5A - 48 = 17..42)
    assert_eq!(char_to_tile(b'A'), 17);
    assert_eq!(char_to_tile(b'Z'), 42);

    // Arbitrary font indices
    assert_eq!(char_to_tile(0x82), 130 - 48); // 82
}

#[test]
fn test_phase_and_row_offset_arithmetic() {
    // Phases 4 down to 1 correspond to rows 0 to 3
    assert_eq!(phase_to_row(4), 0);
    assert_eq!(phase_to_row(3), 1);
    assert_eq!(phase_to_row(2), 2);
    assert_eq!(phase_to_row(1), 3);

    // Tilemap offsets base at 0x0220 ($9A20 - $9800 = 544)
    assert_eq!(phase_to_tilemap_offset(4), 544);
    assert_eq!(phase_to_tilemap_offset(3), 544 + 32);
    assert_eq!(phase_to_tilemap_offset(2), 544 + 64);
    assert_eq!(phase_to_tilemap_offset(1), 544 + 96);
}

#[test]
fn test_blank_row_marker() {
    let mut state = CreditsTextState::new(0);
    let mut tilemap = vec![0x00; 1024];
    let text = [BLANK_ROW_MARKER, b'A'];

    let res = state.step(&text, &mut tilemap).unwrap();
    assert_eq!(state.ptr, 1);
    assert_eq!(state.phase, 3);
    assert!(res.is_blank_row);
    assert_eq!(res.row_index, 0);
    assert_eq!(res.dest_offset, 544);
    assert_eq!(&tilemap[544..544 + CREDITS_ROW_LEN], &[BLANK_TILE; 20]);
    assert!(!res.page_completed);
    assert_eq!(res.fade_timer, None);
    assert!(!state.done);
}

#[test]
fn test_regular_row_rendering() {
    let mut state = CreditsTextState::new(0);
    let mut tilemap = vec![0x00; 1024];
    let row_str = b"  THANK YOU MARIO   ";
    assert_eq!(row_str.len(), 20);

    let res = state.step(row_str, &mut tilemap).unwrap();
    assert_eq!(state.ptr, 20);
    assert_eq!(state.phase, 3);
    assert!(!res.is_blank_row);
    assert_eq!(res.row_index, 0);
    assert_eq!(res.dest_offset, 544);

    let expected_tiles: Vec<u8> = row_str.iter().map(|&c| char_to_tile(c)).collect();
    assert_eq!(&tilemap[544..544 + 20], &expected_tiles[..]);
}

#[test]
fn test_page_completion_sets_fade_timer() {
    let mut state = CreditsTextState::new(0);
    let mut tilemap = vec![0x00; 1024];
    let text = [BLANK_ROW_MARKER; 4];

    for expected_phase in (1..=3).rev() {
        let res = state.step(&text, &mut tilemap).unwrap();
        assert!(!res.page_completed);
        assert_eq!(res.fade_timer, None);
        assert_eq!(res.phase_remaining, expected_phase);
    }

    // Fourth row completes the page
    let res = state.step(&text, &mut tilemap).unwrap();
    assert!(res.page_completed);
    assert_eq!(res.phase_remaining, 0);
    assert_eq!(res.fade_timer, Some(256));
}

#[test]
fn test_sequence_terminator_sets_done() {
    let mut state = CreditsTextState::new(0);
    let mut tilemap = vec![0x00; 1024];
    // Blank row marker followed immediately by sequence terminator 0xFE
    let text = [BLANK_ROW_MARKER, SEQUENCE_TERMINATOR];

    let res = state.step(&text, &mut tilemap).unwrap();
    assert_eq!(state.ptr, 1);
    assert!(state.done);
    assert!(res.sequence_done);
}

#[test]
fn test_bounds_and_truncation_errors() {
    let mut state = CreditsTextState::new(0);
    let mut tilemap = vec![0x00; 1024];

    // Empty text
    assert_eq!(
        state.step(&[], &mut tilemap),
        Err(CreditsTextError::TextTruncated {
            required: 1,
            available: 0,
        })
    );

    // Non-blank row with fewer than 20 characters
    let short_text = b"SHORT";
    assert_eq!(
        state.step(short_text, &mut tilemap),
        Err(CreditsTextError::TextTruncated {
            required: 20,
            available: 5,
        })
    );

    // Tilemap too small to fit destination row
    let mut tiny_tilemap = vec![0x00; 500];
    assert_eq!(
        state.step(b"12345678901234567890", &mut tiny_tilemap),
        Err(CreditsTextError::TilemapOutOfBounds {
            required: 564,
            available: 500,
        })
    );
}

#[test]
fn test_real_rom_credits_full_simulation() {
    use std::path::Path;
    let rom_path = Path::new("../sml2decomp/baserom.gb");
    if !rom_path.is_file() {
        return;
    }
    let rom = std::fs::read(rom_path).expect("read baserom.gb");
    let text_start = 0x696CE;
    let text_end = 0x69DD4;
    let text = &rom[text_start..text_end];
    assert_eq!(text.len(), 1798);
    assert_eq!(text[text.len() - 1], SEQUENCE_TERMINATOR);

    let mut state = CreditsTextState::new(0);
    let mut tilemap = vec![0x00; 1024];
    let mut page_count = 0;
    let mut rows_count = 0;

    while !state.done && state.ptr < text.len() {
        if state.phase == 0 {
            state.start_page();
            page_count += 1;
        }
        let res = state.step(text, &mut tilemap).expect("render credits row");
        rows_count += 1;
        if res.page_completed {
            assert_eq!(res.fade_timer, Some(256));
        }
    }

    assert!(state.done);
    assert_eq!(state.ptr, 1797);
    assert_eq!(rows_count, 144);
    assert_eq!(page_count + 1, 36);
}
