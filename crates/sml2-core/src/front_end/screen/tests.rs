use super::*;

fn solid_tile(color: u8) -> [u8; 16] {
    let mut tile = [0u8; 16];
    for y in 0..8 {
        let (mut lo, mut hi) = (0u8, 0u8);
        for x in 0..8 {
            let bit = 7 - x;
            if color & 1 != 0 {
                lo |= 1 << bit;
            }
            if color & 2 != 0 {
                hi |= 1 << bit;
            }
        }
        tile[y * 2] = lo;
        tile[y * 2 + 1] = hi;
    }
    tile
}

fn pixel_tile(x: usize, y: usize, color: u8) -> [u8; 16] {
    let mut tile = [0u8; 16];
    if color & 1 != 0 {
        tile[y * 2] |= 1 << (7 - x);
    }
    if color & 2 != 0 {
        tile[y * 2 + 1] |= 1 << (7 - x);
    }
    tile
}

fn put(vram: &mut [u8], tile: usize, bytes: [u8; 16]) {
    vram[tile * 16..tile * 16 + 16].copy_from_slice(&bytes);
}

#[test]
fn an_empty_map_renders_shade_zero() {
    let vram = vec![0u8; VRAM_LEN];
    let map = vec![0u8; TILEMAP_LEN];
    let frame = render_bg(&vram, &map, 0xE4, (0, 0), (16, 16)).unwrap();
    assert_eq!(frame.len(), 256);
    assert!(
        frame.iter().all(|&p| p == 255),
        "BGP $E4 maps color 0 to 255"
    );
}

#[test]
fn tile_pixels_and_the_palette_decide_the_shade() {
    let mut vram = vec![0u8; VRAM_LEN];
    let map = vec![0u8; TILEMAP_LEN];
    put(&mut vram, 0, pixel_tile(0, 0, 1));
    let frame = render_bg(&vram, &map, 0xE4, (0, 0), (8, 8)).unwrap();
    assert_eq!(frame[0], 170, "color 1 -> BGP $E4 field 1 -> 170");
    assert_eq!(frame[1], 255);

    put(&mut vram, 0, pixel_tile(3, 2, 3));
    let frame = render_bg(&vram, &map, 0xE4, (0, 0), (8, 8)).unwrap();
    assert_eq!(frame[2 * 8 + 3], 0, "color 3 -> 0");

    put(&mut vram, 0, solid_tile(1));
    let frame = render_bg(&vram, &map, 0x1B, (0, 0), (8, 8)).unwrap();
    assert!(
        frame.iter().all(|&p| p == 85),
        "BGP $1B maps color 1 to field 2 (85)"
    );
}

#[test]
fn the_map_scrolls_and_wraps_at_256_pixels() {
    let mut vram = vec![0u8; VRAM_LEN];
    let mut map = vec![0u8; TILEMAP_LEN];
    put(&mut vram, 1, solid_tile(3));
    put(&mut vram, 2, solid_tile(1));
    map[0] = 1;
    map[1] = 2;
    map[31] = 2;

    let frame = render_bg(&vram, &map, 0xE4, (0, 0), (8, 8)).unwrap();
    assert!(frame.iter().all(|&p| p == 0), "tile 1 is color 3");

    let frame = render_bg(&vram, &map, 0xE4, (8, 0), (8, 8)).unwrap();
    assert!(frame.iter().all(|&p| p == 170), "scrolled to tile 2");

    let frame = render_bg(&vram, &map, 0xE4, (248, 0), (8, 8)).unwrap();
    assert!(
        frame.iter().all(|&p| p == 170),
        "column 31 wraps in from the right"
    );
}

#[test]
fn short_background_inputs_are_rejected() {
    let vram = vec![0u8; VRAM_LEN];
    let map = vec![0u8; TILEMAP_LEN];
    assert!(render_bg(&vram[..VRAM_LEN - 1], &map, 0, (0, 0), (8, 8))
        .unwrap_err()
        .contains("background VRAM"));
    assert!(render_bg(&vram, &map[..TILEMAP_LEN - 1], 0, (0, 0), (8, 8))
        .unwrap_err()
        .contains("background map"));
}

#[test]
fn file_select_backdrop_repeats_the_blocks_and_runs_pipes_on() {
    let mut map = vec![0u8; TILEMAP_LEN];
    for (row, tiles) in [(0, [0xF8, 0xF9]), (1, [0xFA, 0xFB]), (17, [0xFA, 0xFB])] {
        for col in 0..20 {
            map[row * 32 + col] = tiles[col % 2];
        }
    }
    map[2] = 0x13;
    map[17 * 32 + 3] = 0x14;
    // Beside the map: the 2×2 checker in step with columns 0-1.
    assert_eq!(file_select_backdrop(&map, -1, 5, 3), ((1, 1), 3));
    assert_eq!(file_select_backdrop(&map, 20, 0, 0), ((0, 0), 0));
    // Above and below: blocks continue, pipe columns repeat their edge tile.
    assert_eq!(file_select_backdrop(&map, 4, -1, 2), ((0, 1), 2));
    assert_eq!(file_select_backdrop(&map, 2, -3, 6), ((2, 0), 6));
    assert_eq!(file_select_backdrop(&map, 3, 20, 1), ((3, 17), 1));
    assert_eq!(file_select_backdrop(&map, 2, 20, 1), ((0, 0), 1));
}
