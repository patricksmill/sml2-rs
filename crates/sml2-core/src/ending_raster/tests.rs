use super::*;

#[test]
fn test_raster_sine_table_exact_bytes() {
    // Exact byte definitions from ROM $6844F
    let rom_raw_bytes: [u8; 16] = [
        0x00, 0x05, 0x09, 0x0C, 0x0D, 0x0C, 0x09, 0x05,
        0x00, 0xFB, 0xF7, 0xF4, 0xF3, 0xF4, 0xF7, 0xFB,
    ];

    let expected_signed: [i8; 16] = [
        0, 5, 9, 12, 13, 12, 9, 5,
        0, -5, -9, -12, -13, -12, -9, -5,
    ];

    assert_eq!(RASTER_SINE_TABLE, expected_signed);

    for (i, (&raw, &signed)) in rom_raw_bytes.iter().zip(expected_signed.iter()).enumerate() {
        assert_eq!(raw as i8, signed, "table mismatch at index {i}");
        assert_eq!(RASTER_SINE_TABLE[i], signed);
    }

    // Mathematical symmetry: table[i] == -table[i + 8] for 0..8
    for i in 0..8 {
        assert_eq!(
            RASTER_SINE_TABLE[i],
            -RASTER_SINE_TABLE[i + 8],
            "symmetry broken at index {i}"
        );
    }
}

#[test]
fn test_frame_cadence_mask_and_wave_speed() {
    assert_eq!(FRAME_CADENCE_MASK, 0x3C);

    // Frame counter advances every tick, but wave phase only steps every 4 frames:
    for base_frame in (0..64).step_by(4) {
        let expected_offset = raster_sine_offset(base_frame, 20);
        for subframe in 0..4 {
            let frame = base_frame + subframe;
            assert_eq!(
                raster_sine_offset(frame, 20),
                expected_offset,
                "frame {frame} must have identical phase to {base_frame}"
            );
        }
    }

    // Verify phase wrapping modulo 16 (every 64 frames):
    assert_eq!(raster_sine_offset(0, 20), raster_sine_offset(64, 20));
    assert_eq!(raster_sine_offset(4, 20), raster_sine_offset(68, 20));
    assert_eq!(raster_sine_offset(60, 20), raster_sine_offset(124, 20));

    // High bits outside 0x3C (bits 6, 7, 0, 1) do not alter phase:
    for frame in 0..=255u8 {
        let masked_frame = frame & 0x3C;
        assert_eq!(
            raster_sine_offset(frame, 30),
            raster_sine_offset(masked_frame, 30)
        );
    }
}

#[test]
fn test_scanline_phase_offsets() {
    // Frame 0: phase = 0, so index = scanline & 0x0F
    assert_eq!(raster_sine_offset(0, 0), 0);
    assert_eq!(raster_sine_offset(0, 1), 5);
    assert_eq!(raster_sine_offset(0, 4), 13);
    assert_eq!(raster_sine_offset(0, 8), 0);
    assert_eq!(raster_sine_offset(0, 12), -13);
    assert_eq!(raster_sine_offset(0, 15), -5);

    // Scanline wraps every 16 lines:
    assert_eq!(raster_sine_offset(0, 16), 0);
    assert_eq!(raster_sine_offset(0, 32), 0);
    assert_eq!(raster_sine_offset(0, 17), 5);
    assert_eq!(raster_sine_offset(0, 255), -5); // (0 + 255) & 15 == 15 -> -5

    // Frame 4: phase = 1, so index = (1 + scanline) & 0x0F
    assert_eq!(raster_sine_offset(4, 0), 5);
    assert_eq!(raster_sine_offset(4, 3), 13);
    assert_eq!(raster_sine_offset(4, 15), 0);
}

#[test]
fn test_scanline_scroll_x_threshold_gates() {
    assert_eq!(RASTER_START_SCANLINE, 16);

    // Scanlines 0..=16 are ALWAYS 0 offset, regardless of frame or raster_y
    for scanline in 0..=16 {
        assert_eq!(
            scanline_scroll_x(0, scanline, 139),
            0,
            "scanline {scanline} must have scroll 0"
        );
        assert_eq!(
            scanline_scroll_x(4, scanline, 139),
            0,
            "scanline {scanline} must have scroll 0"
        );
    }

    // Boundary at scanline 16 vs 17:
    assert_eq!(scanline_scroll_x(0, 16, 139), 0);
    assert_eq!(scanline_scroll_x(0, 17, 139), raster_sine_offset(0, 17));
    assert_ne!(scanline_scroll_x(0, 17, 139), 0);

    // When raster_y is retracted to 13 (substates 6 and 10), no scanlines wobble:
    for scanline in 0..144 {
        assert_eq!(
            scanline_scroll_x(0, scanline, 13),
            0,
            "retracted raster_y=13 must disable wave on scanline {scanline}"
        );
    }

    // Arbitrary intermediate raster_y = 60:
    assert_eq!(scanline_scroll_x(0, 60, 60), raster_sine_offset(0, 60));
    assert_eq!(scanline_scroll_x(0, 61, 60), 0);
    assert_eq!(scanline_scroll_x(0, 143, 60), 0);

    // Full ending wobble raster_y = 139 (substates 7, 8, 9, 18):
    assert_eq!(scanline_scroll_x(0, 139, 139), raster_sine_offset(0, 139));
    assert_eq!(scanline_scroll_x(0, 140, 139), 0);
    assert_eq!(scanline_scroll_x(0, 143, 139), 0);
}

#[test]
fn test_scanline_unsigned_bg_tiles_split() {
    assert_eq!(BG_SPLIT_SCANLINE, 111);

    // Lines 0..=110 are signed ($8800..$97FF, LCDC bit 4 = 0)
    for scanline in 0..111 {
        assert!(
            !scanline_unsigned_bg_tiles(scanline),
            "scanline {scanline} must use signed tile addressing"
        );
    }

    // Exact split boundary
    assert!(!scanline_unsigned_bg_tiles(110));
    assert!(scanline_unsigned_bg_tiles(111));

    // Lines 111..=143 are unsigned ($8000..$8FFF, LCDC bit 4 = 1)
    for scanline in 111..144 {
        assert!(
            scanline_unsigned_bg_tiles(scanline),
            "scanline {scanline} must use unsigned tile addressing"
        );
    }
}

#[test]
fn test_background_tile_offsets() {
    // Unsigned addressing (base $8000)
    assert_eq!(background_tile_offset(0, true), 0x0000);
    assert_eq!(background_tile_offset(1, true), 0x0010);
    assert_eq!(background_tile_offset(127, true), 127 * 16);
    assert_eq!(background_tile_offset(128, true), 0x0800);
    assert_eq!(background_tile_offset(255, true), 255 * 16);

    // Signed addressing (base $8800)
    // IDs 0..127 map to $9000..$97FF (offset 0x1000..0x1800)
    assert_eq!(background_tile_offset(0, false), 0x1000);
    assert_eq!(background_tile_offset(1, false), 0x1010);
    assert_eq!(background_tile_offset(127, false), 0x17F0);

    // IDs 128..255 map to $8800..$8FFF (offset 0x0800..0x1000)
    assert_eq!(background_tile_offset(128, false), 0x0800);
    assert_eq!(background_tile_offset(255, false), 0x0FF0);

    // Verify consistency with crate::ending_scene::EndingAssets:
    // In EndingAssets, background_tile(id) uses offset 0x800 + bg_tile_offset(id).
    for id in 0..=255u8 {
        assert_eq!(
            background_tile_offset(id, false),
            0x800 + crate::video::bg_tile_offset(id)
        );
    }
}

#[test]
fn test_scanline_projection_and_project_frame() {
    // Frame projection when split is disabled (e.g. substates 7..10)
    let frame_no_split = project_frame(0, 139, false);
    assert_eq!(frame_no_split.len(), 144);

    for (y, proj) in frame_no_split.iter().enumerate() {
        assert_eq!(proj.scanline, y as u8);
        assert!(!proj.unsigned_bg_tiles, "split disabled must keep all lines signed");
        if y <= 16 || y > 139 {
            assert_eq!(proj.scroll_x, 0);
        } else {
            assert_eq!(proj.scroll_x, raster_sine_offset(0, y as u8));
        }
    }

    // Frame projection when split is enabled (substates 18 and 19)
    let frame_with_split = project_frame(4, 139, true);
    assert_eq!(frame_with_split.len(), 144);

    for (y, proj) in frame_with_split.iter().enumerate() {
        assert_eq!(proj.scanline, y as u8);
        if y < 111 {
            assert!(!proj.unsigned_bg_tiles);
        } else {
            assert!(proj.unsigned_bg_tiles);
        }

        if y <= 16 || y > 139 {
            assert_eq!(proj.scroll_x, 0);
        } else {
            assert_eq!(proj.scroll_x, raster_sine_offset(4, y as u8));
        }
    }
}
