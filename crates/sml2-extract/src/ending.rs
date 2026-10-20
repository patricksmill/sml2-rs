//! Fixed, uncompressed copies made by home $29DD with bank $1A at entry.
use std::{error::Error, fs, path::Path};

use sml2_core::ending_scene::EndingAssets;

// ld a,$11 / ld hl,$5800 / ld bc,$1800; then bank $1A, de=$548E.
const TILES_ROM: usize = 0x45800;
const TILEMAP_ROM: usize = 0x6948E;
/// `$68572`'s service starts its pointer at bank 26 `$56CE`; rows are 20 bytes,
/// `$FF` is a blank row and `$FE` ends the sequence.
const CREDITS_TEXT_ROM: usize = 26 * 0x4000 + (0x56CE - 0x4000);

fn extract(rom: &[u8]) -> Result<EndingAssets, String> {
    let tiles = rom
        .get(TILES_ROM..TILES_ROM + 0x1800)
        .ok_or("ending tiles truncated at ROM $45800..$46FFF")?;
    let tilemap = rom
        .get(TILEMAP_ROM..TILEMAP_ROM + 0x400)
        .ok_or("ending tilemap truncated at ROM $6948E..$6988D")?;
    EndingAssets::new(tiles.to_vec(), tilemap.to_vec())
}

/// The credits text rows up to and including the `$FE` terminator.
pub fn credits_text(rom: &[u8]) -> Result<Vec<u8>, String> {
    let rest = rom
        .get(CREDITS_TEXT_ROM..)
        .ok_or("credits text truncated")?;
    let len = rest
        .iter()
        .position(|&b| b == 0xFE)
        .ok_or("credits text has no $FE terminator")?
        + 1;
    Ok(rest[..len].to_vec())
}

pub fn emit(rom: &[u8], extracted: &Path) -> Result<(), Box<dyn Error>> {
    let assets = extract(rom)?;
    let dir = extracted.join("ending");
    fs::create_dir_all(&dir)?;
    let credits = credits_text(rom)?;
    fs::write(dir.join("credits.bin"), &credits)?;
    println!("wrote ending/credits.bin: {} bytes", credits.len());
    fs::write(
        dir.join("scene.json"),
        serde_json::to_string_pretty(&assets)? + "\n",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_029dd_exact_banked_sources_and_complete_uncompressed_copies() {
        let mut rom = vec![0xAA; 0x6988E];
        // Independent source expressions from bank and CPU addresses, not extractor constants.
        let tiles_start = 17 * 0x4000 + (0x5800 - 0x4000);
        let map_start = 26 * 0x4000 + (0x548E - 0x4000);
        for (i, byte) in rom[tiles_start..tiles_start + 6144].iter_mut().enumerate() {
            *byte = (i ^ (i >> 8)) as u8;
        }
        for (i, byte) in rom[map_start..map_start + 1024].iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(7);
        }
        let result = extract(&rom).unwrap();
        assert_eq!(result.tiles(), &rom[0x45800..0x47000]);
        assert_eq!(result.tilemap(), &rom[0x6948E..0x6988E]);
        assert_eq!(result.tiles().len(), 384 * 16);
        assert_eq!(result.tilemap().len(), 32 * 32);
        for len in [0, 0x45800, 0x46fff, 0x6948E, 0x6988D] {
            assert!(
                extract(&rom[..len]).is_err(),
                "accepted truncated ROM length {len:x}"
            );
        }
    }
}
