//! Title-screen and file-select VRAM/tilemap extraction.
//!
//! The title loader (`$14043`) copies `GFX_TitleScreen` (`$41E5`, bank 5) to
//! `$8000` and the tilemap at `$59E5` to `$9800`. The file-select loader
//! (`$30000`/`$2B8B`) composes the whole `$8000-$97FF` window from five
//! sources; the four bank/address slices are assembled here.

use std::error::Error;
use std::fs;
use std::path::Path;

use sml2_core::front_end::{FrontEndAssets, DEMO_TAPES_LEN, TILEMAP_LEN, VRAM_LEN};

/// Bank 5 `$41E5`, 0x1800 bytes (`GFX_TitleScreen`).
const TITLE_VRAM_ROM: usize = 5 * 0x4000 + (0x41E5 - 0x4000);
/// Bank 5 `$59E5`, 0x400 bytes, copied to `$9800`.
const TITLE_TILEMAP_ROM: usize = 5 * 0x4000 + (0x59E5 - 0x4000);
/// Bank 12 `$4A52`, 0x400 bytes, copied to `$9800`.
const FILE_SELECT_TILEMAP_ROM: usize = 12 * 0x4000 + (0x4A52 - 0x4000);
/// Bank 5 `$7C00-$7FFF`: the four 256-byte attract-demo tapes.
const DEMO_TAPES_ROM: usize = 5 * 0x4000 + (0x7C00 - 0x4000);

/// The four slices `$2B8B` copies after `GFX_Mario`, as
/// `(ROM offset, length, VRAM offset)`.
const FILE_SELECT_BLOCKS: [(usize, usize, usize); 4] = [
    // bank 27 `$7000` -> `$8800`
    (27 * 0x4000 + (0x7000 - 0x4000), 0x300, 0x800),
    // bank 12 `$6ACA` -> `$8B00`
    (12 * 0x4000 + (0x6ACA - 0x4000), 0x380, 0x0B00),
    // bank 7 `$6A00` -> `$8E80`
    (7 * 0x4000 + (0x6A00 - 0x4000), 0x380, 0x0E80),
    // bank 7 `$5E00` -> `$9200`
    (7 * 0x4000 + (0x5E00 - 0x4000), 0x600, 0x1200),
];

/// `GFX_Mario`, the first 0x800 bytes of both windows.
const MARIO_LEN: usize = 0x800;

fn take<'a>(rom: &'a [u8], offset: usize, len: usize, what: &str) -> Result<&'a [u8], String> {
    rom.get(offset..offset + len)
        .ok_or_else(|| format!("{what}: truncated at ROM ${offset:05X}..${:05X}", offset + len))
}

/// Compose both screens from the ROM plus the decomp's `gfx/mario.2bpp`.
pub fn extract(rom: &[u8], mario: &[u8]) -> Result<FrontEndAssets, String> {
    if mario.len() != MARIO_LEN {
        return Err(format!(
            "gfx/mario.2bpp is {} bytes, expected {MARIO_LEN}",
            mario.len()
        ));
    }
    let title_vram = take(rom, TITLE_VRAM_ROM, VRAM_LEN, "title gfx")?.to_vec();
    let title_tilemap = take(rom, TITLE_TILEMAP_ROM, TILEMAP_LEN, "title tilemap")?.to_vec();

    let mut file_select_vram = vec![0u8; VRAM_LEN];
    file_select_vram[..MARIO_LEN].copy_from_slice(mario);
    for (rom_offset, len, vram_offset) in FILE_SELECT_BLOCKS {
        let block = take(rom, rom_offset, len, "file-select gfx")?;
        file_select_vram[vram_offset..vram_offset + len].copy_from_slice(block);
    }
    let file_select_tilemap = take(
        rom,
        FILE_SELECT_TILEMAP_ROM,
        TILEMAP_LEN,
        "file-select tilemap",
    )?
    .to_vec();
    let demo_tapes = take(rom, DEMO_TAPES_ROM, DEMO_TAPES_LEN, "demo tapes")?.to_vec();

    FrontEndAssets::new(
        title_vram,
        title_tilemap,
        file_select_vram,
        file_select_tilemap,
        demo_tapes,
    )
}

pub fn emit(rom: &[u8], decomp: &Path, extracted: &Path) -> Result<(), Box<dyn Error>> {
    let mario = fs::read(decomp.join("gfx/mario.2bpp"))?;
    let title_incbin = fs::read(decomp.join("gfx/titlescreen.2bpp"))?;
    let assets = extract(rom, &mario)?;
    if assets.title_vram() != title_incbin.as_slice() {
        return Err("GFX_TitleScreen differs from gfx/titlescreen.2bpp".into());
    }

    let dir = extracted.join("frontend");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("title_vram.bin"), assets.title_vram())?;
    fs::write(dir.join("title_tilemap.bin"), assets.title_tilemap())?;
    fs::write(dir.join("file_select_vram.bin"), assets.file_select_vram())?;
    fs::write(
        dir.join("file_select_tilemap.bin"),
        assets.file_select_tilemap(),
    )?;
    fs::write(dir.join("demo_tapes.bin"), assets.demo_tapes())?;
    println!(
        "wrote frontend/: title {VRAM_LEN}+{TILEMAP_LEN}, file select {VRAM_LEN}+{TILEMAP_LEN}, demos {DEMO_TAPES_LEN} bytes"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Largest source end, so a full synthetic ROM covers every slice.
    const ROM_LEN: usize = 0x6F000 + 0x300;

    fn synthetic() -> (Vec<u8>, [u8; MARIO_LEN]) {
        let mut rom = vec![0u8; ROM_LEN];
        for (offset, len, seed) in [
            (TITLE_VRAM_ROM, VRAM_LEN, 0x10u8),
            (TITLE_TILEMAP_ROM, TILEMAP_LEN, 0x20),
            (FILE_SELECT_TILEMAP_ROM, TILEMAP_LEN, 0x30),
            (FILE_SELECT_BLOCKS[0].0, 0x300, 0x40),
            (FILE_SELECT_BLOCKS[1].0, 0x380, 0x50),
            (FILE_SELECT_BLOCKS[2].0, 0x380, 0x60),
            (FILE_SELECT_BLOCKS[3].0, 0x600, 0x70),
            (DEMO_TAPES_ROM, DEMO_TAPES_LEN, 0x80),
        ] {
            for (i, byte) in rom[offset..offset + len].iter_mut().enumerate() {
                *byte = seed.wrapping_add(i as u8);
            }
        }
        let mut mario = [0u8; MARIO_LEN];
        for (i, byte) in mario.iter_mut().enumerate() {
            *byte = 0x80u8.wrapping_add((i * 3) as u8);
        }
        (rom, mario)
    }

    #[test]
    fn both_windows_are_composed_from_the_source_slices() {
        let (rom, mario) = synthetic();
        let assets = extract(&rom, &mario).unwrap();
        assert_eq!(assets.title_vram(), &rom[TITLE_VRAM_ROM..TITLE_VRAM_ROM + VRAM_LEN]);
        assert_eq!(
            assets.title_tilemap(),
            &rom[TITLE_TILEMAP_ROM..TITLE_TILEMAP_ROM + TILEMAP_LEN]
        );
        assert_eq!(
            assets.file_select_tilemap(),
            &rom[FILE_SELECT_TILEMAP_ROM..FILE_SELECT_TILEMAP_ROM + TILEMAP_LEN]
        );
        assert_eq!(&assets.file_select_vram()[..MARIO_LEN], &mario[..]);
        for (rom_offset, len, vram_offset) in FILE_SELECT_BLOCKS {
            assert_eq!(
                &assets.file_select_vram()[vram_offset..vram_offset + len],
                &rom[rom_offset..rom_offset + len],
                "block at ROM ${rom_offset:05X} -> VRAM ${:04X}",
                0x8000 + vram_offset
            );
        }
    }

    #[test]
    fn truncated_sources_and_a_wrong_mario_length_are_rejected() {
        let (rom, mario) = synthetic();
        assert!(extract(&rom[..ROM_LEN - 1], &mario).is_err());
        assert!(extract(&rom[..0x159E5], &mario).is_err(), "title tilemap cut");
        assert!(extract(&rom, &mario[..MARIO_LEN - 1]).is_err());
    }
}
