use super::*;
use crate::front_end::file_select::FileSelect;
use crate::front_end::save_slots::{SaveSlots, SLOT_LEN};
use crate::front_end::{TILEMAP_LEN, VRAM_LEN};

/// A solid tile at `offset`: `color` 0..=3 in both bitplanes.
fn paint(vram: &mut [u8], offset: usize, color: u8) {
    let lo = if color & 1 != 0 { 0xFF } else { 0x00 };
    let hi = if color & 2 != 0 { 0xFF } else { 0x00 };
    for row in 0..8 {
        vram[offset + row * 2] = lo;
        vram[offset + row * 2 + 1] = hi;
    }
}

/// The real player mapping table (`extracted/objects.json`), so the cursor is
/// the same 16x32 metasprite the ROM expands at `$5297`.
fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .expect("standalone extracted object tables")
}

/// A black background: the file select's signed mapping sends tile 0 to
/// `$9000` (offset 0x1000), while its sprite tiles live at `$8000` (offset 0).
fn assets() -> FrontEndAssets {
    let mut title_vram = vec![0u8; VRAM_LEN];
    paint(&mut title_vram, 0x1000, 3);
    let title_tilemap = vec![0u8; TILEMAP_LEN];

    let mut vram = vec![0u8; VRAM_LEN];
    paint(&mut vram, 0x1000, 3);
    for tile in [
        0u8, 1, 16, 17, 36, 37, 52, 57, 64, 65, 80, 81, 96, 97, 0x83, 0x84, 209,
    ] {
        paint(&mut vram, tile as usize * 16, 1);
    }
    let tilemap = vec![0u8; TILEMAP_LEN];
    FrontEndAssets::new(
        title_vram,
        title_tilemap,
        vram,
        tilemap,
        vec![0u8; crate::front_end::DEMO_TAPES_LEN],
    )
    .unwrap()
}

fn dropped_file_select() -> (FileSelect, SaveSlots) {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    while fs.menu_state() != 1 {
        fs.step(&mut slots, 0, 0);
    }
    (fs, slots)
}

#[test]
fn the_title_renders_its_tilemap_through_the_palette() {
    let frame = render_title(&assets(), (160, 144));
    assert_eq!(frame.len(), 160 * 144);
    assert!(
        frame.iter().all(|&p| p == 0),
        "the synthetic title is a solid colour-3 screen (signed tile 0)"
    );
}

#[test]
fn the_file_select_uses_signed_tile_data() {
    let assets = assets();
    let (fs, slots) = dropped_file_select();
    let frame = render_file_select(&fs, &slots, &assets, &tables(), (160, 144));
    // Background tile 0 lives at the signed base ($9000, offset 0x1000); the
    // unsigned $8000 copy is white. A black border-free screen proves the
    // signed lookup.
    assert_eq!(frame[0], 0);
    assert_eq!(frame[50 * 160 + 120], 0);
}

#[test]
fn the_cursor_is_the_sixteen_by_thirtytwo_player_metasprite() {
    let assets = assets();
    let (fs, slots) = dropped_file_select();
    assert_eq!(fs.cursor(), (40, 0x7C));
    let frame = render_file_select(&fs, &slots, &assets, &tables(), (160, 144));

    // Frame 10, base (40,124): first cell at OAM (32,112) -> screen (24,96).
    assert_eq!(frame[96 * 160 + 24], 255, "cursor top-left");
    assert_eq!(frame[127 * 160 + 39], 255, "cursor bottom-right");
    assert_eq!(frame[88 * 160 + 24], 0, "above the cursor is background");
}

#[test]
fn the_tally_digits_use_both_nibbles() {
    let assets = assets();
    let (fs, mut slots) = dropped_file_select();
    let mut record = [0u8; SLOT_LEN];
    record[0x42] = 0x34;
    slots.write_slot(1, &record);
    let frame = render_file_select(&fs, &slots, &assets, &tables(), (160, 144));

    // Slot 1: OAM X 64 -> screen 56, Y 88 -> screen 72.
    assert_eq!(frame[72 * 160 + 56], 255, "high nibble (tile $83)");
    assert_eq!(frame[72 * 160 + 64], 255, "low nibble (tile $84)");
    // Slot 0's zero tally uses tile $80 (colour 0) and stays invisible.
    assert_eq!(frame[72 * 160 + 24], 0);
}

#[test]
fn the_easy_banner_draws_at_its_rom_position() {
    let assets = assets();
    let (mut fs, mut slots) = dropped_file_select();
    fs.step(
        &mut slots,
        crate::front_end::keys::SELECT,
        crate::front_end::keys::SELECT,
    );
    assert!(fs.easy_mode());
    let frame = render_file_select(&fs, &slots, &assets, &tables(), (160, 144));
    // Frame 241, base (68,48) -> first tile's screen (60,32).
    assert_eq!(frame[32 * 160 + 60], 255);
    // The cursor switches to the small-Mario frame 32 (base 40,124 ->
    // first cell's screen 24,104).
    assert_eq!(frame[104 * 160 + 24], 255);
}

#[test]
fn clear_button_renders_bomb_mario_pixels_at_the_bobbed_position() {
    use crate::front_end::keys::{A, RIGHT};

    let base_assets = assets();
    let mut vram = base_assets.file_select_vram().to_vec();
    // ROM mappings 204..206 use tile 199 for Bomb Mario's upper-left
    // cell and 207/208 for the fuse at the right side.
    paint(&mut vram, 199 * 16, 2);
    for tile in 200..=208 {
        paint(&mut vram, tile * 16, 1);
    }
    let assets = FrontEndAssets::new(
        base_assets.title_vram().to_vec(),
        base_assets.title_tilemap().to_vec(),
        vram,
        base_assets.file_select_tilemap().to_vec(),
        base_assets.demo_tapes().to_vec(),
    )
    .unwrap();
    let (mut fs, mut slots) = dropped_file_select();
    for _ in 0..3 {
        fs.step(&mut slots, RIGHT, 0);
        for _ in 0..32 {
            fs.step(&mut slots, 0, 0);
        }
    }
    let ordinary = render_file_select(&fs, &slots, &assets, &tables(), (160, 144));
    fs.step(&mut slots, A, A);
    for _ in 0..55 {
        fs.step(&mut slots, 0, 0);
    }
    // Select phase 0 explicitly: frame 204, base (136,125). Its tile 199
    // lands at screen (120,113) and its fuse at (136,117).
    for _ in 0..16 {
        if fs.cursor_presentation().sprite == 204 {
            break;
        }
        fs.step(&mut slots, 0, 0);
    }
    assert_eq!(fs.cursor_presentation().sprite, 204);
    assert_eq!(fs.cursor(), (136, 124), "movement row stays unchanged");
    let bomb = render_file_select(&fs, &slots, &assets, &tables(), (160, 144));
    assert_eq!(
        bomb[113 * 160 + 120],
        170,
        "Bomb Mario tile 199, OBP0 shade 1"
    );
    assert_eq!(bomb[117 * 160 + 136], 255, "Bomb Mario's fuse");
    assert_eq!(
        bomb[112 * 160 + 120],
        0,
        "one-pixel bob leaves the row above blank"
    );
    assert_ne!(ordinary[113 * 160 + 120], bomb[113 * 160 + 120]);
}

#[test]
fn palette_frames_keep_front_end_sources_even_when_the_sprite_shade_is_white() {
    use crate::video::{OutputMode, PalettePixel, PaletteSource};
    let assets = assets();
    let (fs, slots) = dropped_file_select();
    let gray = render_file_select(&fs, &slots, &assets, &tables(), (160, 144));
    let tokens = render_file_select_output(
        &fs,
        &slots,
        &assets,
        &tables(),
        (160, 144),
        OutputMode::PaletteIndices,
    );
    assert_eq!(tokens[0], 3, "mapped BG shade 3");
    assert_eq!(
        tokens[96 * 160 + 24],
        4,
        "OBP0 mapped shade 0 is still an OBJ pixel"
    );
    assert_eq!(
        tokens
            .iter()
            .map(|&p| PalettePixel::from_token(p).grayscale())
            .collect::<Vec<_>>(),
        gray
    );
    let title = render_title_output(&assets, (160, 144), OutputMode::PaletteIndices);
    assert!(title.iter().all(|&p| p == 3));
    assert_eq!(PalettePixel::from_token(title[0]).source, PaletteSource::Bg);

    let mut frame = vec![1; 24 * 8];
    let mut vram = vec![0; VRAM_LEN];
    paint(&mut vram, 0, 1);
    draw_cell(
        &mut frame,
        24,
        8,
        &vram,
        OutputMode::PaletteIndices,
        Cell {
            x: 8,
            y: 16,
            tile: 0,
            attr: 0x10,
        },
    );
    assert_eq!(
        frame[0],
        8 | ((FILE_SELECT_OBP1 >> 2) & 3),
        "attr bit 4 selects OBP1"
    );
    draw_cell(
        &mut frame,
        24,
        8,
        &vram,
        OutputMode::PaletteIndices,
        Cell {
            x: 16,
            y: 16,
            tile: 0,
            attr: 0x80,
        },
    );
    assert_eq!(frame[8], 1, "behind-background sprite preserves BG source");
    draw_cell(
        &mut frame,
        24,
        8,
        &vram,
        OutputMode::PaletteIndices,
        Cell {
            x: 24,
            y: 16,
            tile: 1,
            attr: 0x10,
        },
    );
    assert_eq!(frame[16], 1, "transparent OBJ colour preserves BG source");
}

#[test]
fn wide_title_continues_the_wall_and_ground_past_the_map() {
    let mut assets = assets();
    let mut vram = assets.title_vram().to_vec();
    paint(&mut vram, 0x85 * 16, 1);
    paint(&mut vram, 0x86 * 16, 2);
    paint(&mut vram, 0x95 * 16, 3);
    vram[0x95 * 16 + 14] = 0xFF;
    vram[0x95 * 16 + 15] = 0x00;
    let mut map = assets.title_tilemap().to_vec();
    map[..2].copy_from_slice(&[0x85, 0x86]);
    map[17 * 32] = 0x95;
    assets = FrontEndAssets::new(
        vram,
        map,
        assets.file_select_vram().to_vec(),
        assets.file_select_tilemap().to_vec(),
        assets.demo_tapes().to_vec(),
    )
    .unwrap();
    let shades = OutputMode::Grayscale.palette(TITLE_BGP, PaletteSource::Bg);
    let (view, at) = ((192usize, 180usize), (16i32, 12i32));
    let out = render_title_wide_output(&assets, view, at, OutputMode::Grayscale);
    let px = |nx: i32, ny: i32| out[(ny + at.1) as usize * view.0 + (nx + at.0) as usize];

    let native = render_title(&assets, (160, 144));
    assert_eq!(px(0, 0), native[0]);
    assert_eq!(px(159, 143), native[143 * 160 + 159]);
    // `$85`/`$86` checker: map (0, 0) is `$85`.
    assert_eq!(px(-1, 0), shades[2]);
    assert_eq!(px(-9, 0), shades[1]);
    assert_eq!(px(160, 0), shades[1]);
    assert_eq!(px(0, -1), shades[2]);
    assert_eq!(px(-1, 100), px(-1, 116));
    // Row 17 is `$95`; below it its last line repeats.
    assert_eq!(px(-1, 136), shades[3]);
    assert_eq!(px(170, 143), shades[1]);
    assert_eq!(px(-16, 160), shades[1]);
}

#[test]
fn the_hall_is_open_sky_over_a_brick_floor_and_mario_is_a_sprite() {
    use crate::front_end::screen::hall_backdrop;
    use crate::video::OutputMode;
    let _ = hall_backdrop;
    let mut vram = vec![0u8; VRAM_LEN];
    // Signed `$FF` is color 0. Signed `$F8` (the brick checker) is color 3.
    paint(&mut vram, 0xF8 * 16, 3);
    for tile in 0..128 {
        paint(&mut vram, tile * 16, 1);
    }
    let mut map = vec![0u8; TILEMAP_LEN];
    map[1] = 0xF8;
    map[4 * 32 + 4] = 0xFF;
    let assets = FrontEndAssets::new(
        vec![0u8; VRAM_LEN],
        vec![0u8; TILEMAP_LEN],
        vram,
        map,
        vec![0u8; crate::front_end::DEMO_TAPES_LEN],
    )
    .unwrap();
    let frame = render_hall_output(
        &assets,
        &tables(),
        -72,
        (48, 0x7C),
        4,
        OutputMode::PaletteIndices,
    );
    assert_eq!(frame.len(), 160 * 144);
    assert_eq!(frame[0], 3, "the ceiling is the brick checker");
    assert_eq!(frame[32 * 160], 0, "the space under the ceiling is open");
    assert_eq!(frame[128 * 160], 3, "the floor is the same bricks");
    assert!(
        frame.iter().any(|token| token >> 2 == 1),
        "Mario is an object sprite"
    );
}

#[test]
fn glyph_stamping_clips_nonzero_pixels_and_preserves_the_background() {
    let mut glyph = [0u8; 16];
    glyph[0] = 0x80; // color 1 at (0, 0), outside the destination
    glyph[3] = 0x40; // color 2 at (1, 1)
    glyph[4] = 0x20;
    glyph[5] = 0x20; // color 3 at (2, 2)
    let mut tokens = vec![9; 16];
    stamp_glyph(&mut tokens, 4, &glyph, (-1, -1), 3);
    assert_eq!(tokens, [3, 9, 9, 9, 9, 3, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9]);
    let mut rgb = vec![[9, 8, 7]; 16];
    stamp_glyph(&mut rgb, 4, &glyph, (-1, -1), [1, 2, 3]);
    assert_eq!(rgb[0], [1, 2, 3]);
    assert_eq!(rgb[5], [1, 2, 3]);
    assert_eq!(rgb[1], [9, 8, 7]);
    let before = tokens.clone();
    stamp_glyph(&mut tokens, 0, &glyph, (0, 0), 0);
    stamp_glyph(&mut tokens, 4, &glyph[..15], (0, 0), 0);
    assert_eq!(tokens, before);
}
