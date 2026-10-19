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
    for tile in [0u8, 1, 16, 17, 36, 37, 52, 57, 64, 65, 80, 81, 96, 97, 0x83, 0x84, 209] {
        paint(&mut vram, tile as usize * 16, 1);
    }
    let tilemap = vec![0u8; TILEMAP_LEN];
    FrontEndAssets::new(title_vram, title_tilemap, vram, tilemap).unwrap()
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
