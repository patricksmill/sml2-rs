//! Extraction of overworld screen tilemaps, dynamic route patches, and cursor metasprites.

use std::{error::Error, fs, path::Path};

/// ROM constants for overworld background tilemaps.
pub const OVERWORLD_1X1_ROM: usize = 0x37000;
pub const OVERWORLD_2X1_ROM: usize = 0x37400;
pub const OVERWORLD_1X2_ROM: usize = 0x37800;
pub const OVERWORLD_2X2_ROM: usize = 0x37C00;

pub const SCREEN_TILEMAP_SPECS: &[(u8, &str, usize, usize, &str, u8)] = &[
    (1, "pumpkin", 0x3B000, 576, "gfx/overworld/pumpkinzone.png", 0xE4),
    (2, "tree_a", 0x3B240, 576, "gfx/overworld/treezone.png", 0xE4),
    (3, "macro", 0x3B6C0, 576, "gfx/overworld/macrozone.png", 0xE4),
    (4, "space", 0x3B480, 576, "gfx/overworld/spacezone.png", 0xE4),
    (5, "tree_b", 0x3B900, 576, "gfx/overworld/treezone.png", 0xE4),
    (6, "mario", 0x3BB40, 576, "gfx/overworld/mariozone.png", 0xE4),
    (7, "turtle", 0x3BD80, 576, "gfx/overworld/wariocastle.png", 0xE1),
    (8, "castle", 0x43000, 576, "gfx/overworld/mariocastle.png", 0xE4),
    (9, "bonus_30", 0x43240, 576, "gfx/overworld/mariocastle.png", 0xE4),
    (10, "bonus_200", 0x43480, 576, "gfx/overworld/mariocastle.png", 0xE4),
    (11, "bonus_50", 0x436C0, 576, "gfx/overworld/mariocastle.png", 0xE4),
    (12, "bonus_999", 0x43900, 576, "gfx/overworld/mariocastle.png", 0xE4),
];

/// Table of 58 patch pointers at bank 24 $63EA (flat $623EA).
pub const PATCH_TABLE_ROM: usize = 24 * 0x4000 + (0x63EA - 0x4000);
pub const PATCH_COUNT: usize = 58;

/// Table of 16 cursor metasprite pointers at bank 15 $4AD7 (flat $3CAD7).
pub const CURSOR_TABLE_ROM: usize = 15 * 0x4000 + (0x4AD7 - 0x4000);
pub const CURSOR_POSE_COUNT: usize = 16;

/// Build the stitched 64x64 hub tilemap and all 32x32 zone screen tilemaps.
pub fn build_tilemaps_binary(baserom: &[u8]) -> Result<Vec<u8>, String> {
    let q1x1 = baserom
        .get(OVERWORLD_1X1_ROM..OVERWORLD_1X1_ROM + 1024)
        .ok_or("truncated overworld1x1")?;
    let q2x1 = baserom
        .get(OVERWORLD_2X1_ROM..OVERWORLD_2X1_ROM + 1024)
        .ok_or("truncated overworld2x1")?;
    let q1x2 = baserom
        .get(OVERWORLD_1X2_ROM..OVERWORLD_1X2_ROM + 1024)
        .ok_or("truncated overworld1x2")?;
    let q2x2 = baserom
        .get(OVERWORLD_2X2_ROM..OVERWORLD_2X2_ROM + 1024)
        .ok_or("truncated overworld2x2")?;

    let mut out = Vec::with_capacity(4096 + 12 * 1024);

    // Screen 0: Hub stitched 64x64 tiles
    let mut hub = vec![0u8; 64 * 64];
    for y in 0..64 {
        for x in 0..64 {
            let (quad, lx, ly) = match (x >= 32, y >= 32) {
                (false, false) => (q1x1, x, y),
                (true, false) => (q2x1, x - 32, y),
                (false, true) => (q1x2, x, y - 32),
                (true, true) => (q2x2, x - 32, y - 32),
            };
            hub[y * 64 + x] = quad[ly * 32 + lx];
        }
    }
    out.extend_from_slice(&hub);

    // Screens 1..=12: 32x32 tilemaps (576 bytes filled, padded to 1024 bytes)
    for &(_id, _name, rom_addr, len, _, _) in SCREEN_TILEMAP_SPECS {
        let src = baserom
            .get(rom_addr..rom_addr + len)
            .ok_or_else(|| format!("truncated tilemap at ROM ${rom_addr:05X}"))?;
        let mut screen_map = vec![0u8; 1024];
        screen_map[..len].copy_from_slice(src);
        out.extend_from_slice(&screen_map);
    }

    Ok(out)
}

/// Extract all 58 patch streams from bank 24.
pub fn extract_patches(baserom: &[u8]) -> Result<Vec<serde_json::Value>, String> {
    let mut patches = Vec::with_capacity(PATCH_COUNT);
    for i in 0..PATCH_COUNT {
        let at = PATCH_TABLE_ROM + i * 2;
        let ptr = baserom[at] as usize | ((baserom[at + 1] as usize) << 8);
        if !(0x4000..0x8000).contains(&ptr) {
            return Err(format!("patch {i}: pointer ${ptr:04X} outside bank window"));
        }
        let flat = 24 * 0x4000 + (ptr - 0x4000);
        let mut tiles = Vec::new();
        let mut curr = flat;
        while baserom[curr] != 0xFF {
            if curr + 3 > baserom.len() {
                return Err(format!("patch {i}: stream truncated before $FF at ${curr:05X}"));
            }
            tiles.push(serde_json::json!({
                "dx": baserom[curr],
                "dy": baserom[curr + 1],
                "tile": baserom[curr + 2],
            }));
            curr += 3;
        }
        patches.push(serde_json::json!({
            "id": i,
            "ptr": format!("${ptr:04X}"),
            "tiles": tiles,
        }));
    }
    Ok(patches)
}

/// Extract all 16 cursor poses (metasprites) from bank 15.
pub fn extract_cursor_poses(baserom: &[u8]) -> Result<Vec<serde_json::Value>, String> {
    let dirs = ["Left", "Right", "Up", "Down"];
    let mut cursor_poses = Vec::with_capacity(CURSOR_POSE_COUNT);
    for i in 0..CURSOR_POSE_COUNT {
        let at = CURSOR_TABLE_ROM + i * 2;
        let ptr = baserom[at] as usize | ((baserom[at + 1] as usize) << 8);
        if !(0x4000..0x8000).contains(&ptr) {
            return Err(format!("cursor pose {i}: pointer ${ptr:04X} outside bank window"));
        }
        let flat = 15 * 0x4000 + (ptr - 0x4000);
        let b = baserom
            .get(flat..flat + 8)
            .ok_or_else(|| format!("cursor pose {i}: truncated at ${flat:05X}"))?;
        cursor_poses.push(serde_json::json!({
            "pose": i,
            "direction": dirs[i / 4],
            "step": i % 4,
            "tiles": [
                { "dx": 0, "dy": 0, "tile": b[0], "attr": b[1] },
                { "dx": 8, "dy": 0, "tile": b[2], "attr": b[3] },
                { "dx": 0, "dy": 8, "tile": b[4], "attr": b[5] },
                { "dx": 8, "dy": 8, "tile": b[6], "attr": b[7] },
            ]
        }));
    }
    Ok(cursor_poses)
}

pub const OVERWORLD_GFX_ROM: usize = 0x34000;
pub const OVERWORLD_GFX_LEN: usize = 6144;
pub const COMMON_CURSOR_LEN: usize = 512;
pub const ZONE_GFX_LEN: usize = 5632;

pub const SCREEN_TILES_SPECS: &[(u8, Option<usize>)] = &[
    (0, None), // Hub: all 6144 bytes from OVERWORLD_GFX_ROM
    (1, Some(0x38200)), // Pumpkin
    (2, Some(0x39A00)), // Tree A
    (3, Some(0x35A00)), // Macro
    (4, Some(0x40200)), // Space
    (5, Some(0x39A00)), // Tree B
    (6, Some(0x44200)), // Mario
    (7, Some(0x45A00)), // Turtle
    (8, Some(0x38200)), // Castle / Minigame hill
    (9, Some(0x64000)), // Bonus 30
    (10, Some(0x64000)), // Bonus 200
    (11, Some(0x64000)), // Bonus 50
    (12, Some(0x64000)), // Bonus 999
];

/// Build the composed 384-tile VRAM blocks for all 13 screens (0..=12).
pub fn build_tiles_binary(baserom: &[u8]) -> Result<Vec<u8>, String> {
    let ow_gfx = baserom
        .get(OVERWORLD_GFX_ROM..OVERWORLD_GFX_ROM + OVERWORLD_GFX_LEN)
        .ok_or("truncated overworld gfx")?;
    let common_cursor = &ow_gfx[..COMMON_CURSOR_LEN];

    let mut out = Vec::with_capacity(SCREEN_TILES_SPECS.len() * OVERWORLD_GFX_LEN);

    for &(id, zone_addr_opt) in SCREEN_TILES_SPECS {
        match zone_addr_opt {
            None => {
                // Screen 0: entire 6144 bytes from overworld.2bpp
                out.extend_from_slice(ow_gfx);
            }
            Some(zone_addr) => {
                let zone_gfx = baserom
                    .get(zone_addr..zone_addr + ZONE_GFX_LEN)
                    .ok_or_else(|| format!("screen {id}: truncated zone gfx at ${zone_addr:05X}"))?;
                out.extend_from_slice(common_cursor);
                out.extend_from_slice(zone_gfx);
            }
        }
    }

    Ok(out)
}

/// Emit `extracted/map/tilemaps.bin`, `extracted/map/route_patches.json`, and `extracted/map/tiles.bin`.
pub fn emit(baserom: &[u8], extracted: &Path) -> Result<(), Box<dyn Error>> {
    let map_dir = extracted.join("map");
    fs::create_dir_all(&map_dir)?;

    // 1. Tilemaps binary
    let tilemaps_bin = build_tilemaps_binary(baserom).map_err(|e| format!("map tilemaps: {e}"))?;
    fs::write(map_dir.join("tilemaps.bin"), &tilemaps_bin)?;
    println!(
        "wrote map/tilemaps.bin: 64x64 hub + 12 zone screens ({} bytes)",
        tilemaps_bin.len()
    );

    // 2. Route patches and cursor metadata
    let patches = extract_patches(baserom).map_err(|e| format!("route patches: {e}"))?;
    let cursor_poses = extract_cursor_poses(baserom).map_err(|e| format!("cursor poses: {e}"))?;

    let mut screens_meta = Vec::new();
    screens_meta.push(serde_json::json!({
        "id": 0,
        "name": "hub",
        "width": 64,
        "height": 64,
        "tilemap_offset": 0,
        "tilemap_len": 4096,
        "gfx": "gfx/overworld/overworld.png",
        "bg_palette": 0xE4,
    }));
    for (idx, &(id, name, _, len, gfx, bg_palette)) in SCREEN_TILEMAP_SPECS.iter().enumerate() {
        screens_meta.push(serde_json::json!({
            "id": id,
            "name": name,
            "width": 32,
            "height": 18,
            "tilemap_offset": 4096 + idx * 1024,
            "tilemap_len": len,
            "gfx": gfx,
            "bg_palette": bg_palette,
        }));
    }

    let dynamic_unlocks = serde_json::json!([
        {
            "name": "pumpkin_path_1",
            "screen_id": 1,
            "progress_offset": sml2_core::map::progress_offset::PUMPKIN_PATH_1,
            "mask": 0x01,
            "patches": [
                { "x": 0, "y": 0, "patch_ids": (31..=38).collect::<Vec<_>>() }
            ]
        },
        {
            "name": "pumpkin_path_2",
            "screen_id": 1,
            "progress_offset": sml2_core::map::progress_offset::PUMPKIN_PATH_2,
            "mask": 0x01,
            "patches": [
                { "x": 0, "y": 0, "patch_ids": (39..=46).collect::<Vec<_>>() }
            ]
        },
        {
            "name": "mario_initial_paths",
            "screen_id": 6,
            "progress_offset": 0,
            "mask": 0x00,
            "patches": [
                { "x": 4, "y": 7, "patch_ids": [0] },
                { "x": 14, "y": 7, "patch_ids": [3] },
                { "x": 8, "y": 9, "patch_ids": [6] },
                { "x": 8, "y": 7, "patch_ids": [9] }
            ]
        }
    ]);

    let manifest = serde_json::json!({
        "screens": screens_meta,
        "patches": patches,
        "cursor_poses": cursor_poses,
        "dynamic_unlocks": dynamic_unlocks,
    });

    fs::write(
        map_dir.join("route_patches.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;
    println!(
        "wrote map/route_patches.json: {} screen specs, {} patch streams, {} cursor poses",
        SCREEN_TILEMAP_SPECS.len() + 1,
        PATCH_COUNT,
        CURSOR_POSE_COUNT
    );

    // 3. Composed VRAM tiles binary
    let tiles_bin = build_tiles_binary(baserom).map_err(|e| format!("map tiles: {e}"))?;
    fs::write(map_dir.join("tiles.bin"), &tiles_bin)?;
    println!(
        "wrote map/tiles.bin: {} screens x 384 tiles ({} bytes)",
        SCREEN_TILES_SPECS.len(),
        tiles_bin.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tilemaps_and_patches_structure() {
        // Construct minimum test buffer
        let mut rom = vec![0u8; 0x70000];
        // Populate 4 quadrants
        for (idx, addr) in [
            OVERWORLD_1X1_ROM,
            OVERWORLD_2X1_ROM,
            OVERWORLD_1X2_ROM,
            OVERWORLD_2X2_ROM,
        ]
        .iter()
        .enumerate()
        {
            rom[*addr..*addr + 1024].fill(idx as u8 + 1);
        }
        // Build tilemaps
        let blob = build_tilemaps_binary(&rom).expect("valid binary");
        assert_eq!(blob.len(), 4096 + 12 * 1024);
        // Verify quadrant stitching
        assert_eq!(blob[0], 1, "top-left quadrant");
        assert_eq!(blob[32], 2, "top-right quadrant");
        assert_eq!(blob[32 * 64], 3, "bottom-left quadrant");
        assert_eq!(blob[32 * 64 + 32], 4, "bottom-right quadrant");
    }

    #[test]
    fn test_tiles_binary_structure() {
        let mut rom = vec![0u8; 0x70000];
        // Fill overworld GFX with 0xAA
        rom[OVERWORLD_GFX_ROM..OVERWORLD_GFX_ROM + OVERWORLD_GFX_LEN].fill(0xAA);
        // Fill Pumpkin Zone with 0xBB
        rom[0x38200..0x38200 + ZONE_GFX_LEN].fill(0xBB);

        let blob = build_tiles_binary(&rom).expect("valid tiles blob");
        assert_eq!(blob.len(), 13 * OVERWORLD_GFX_LEN);

        // Screen 0 should be all 0xAA
        assert!(blob[..OVERWORLD_GFX_LEN].iter().all(|&b| b == 0xAA));

        // Screen 1 (Pumpkin): first 512 bytes 0xAA (cursor), next 5632 bytes 0xBB (zone bg)
        let s1 = &blob[OVERWORLD_GFX_LEN..OVERWORLD_GFX_LEN * 2];
        assert!(s1[..COMMON_CURSOR_LEN].iter().all(|&b| b == 0xAA));
        assert!(s1[COMMON_CURSOR_LEN..].iter().all(|&b| b == 0xBB));
    }
}
