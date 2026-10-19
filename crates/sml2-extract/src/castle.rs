//! Bank-24 castle and bonus-room scripts, and the bell-room tileset.
//!
//! `$3E00` is only a far entry (`ld [sCurROMBank], a; jp hl`), so each bonus
//! room "script" is ordinary bank-24 code reached by an entry address:
//! `$4028` (bell room B, screen 14), `$4512` (bell room A, screen 13) and
//! `$4A81` (coin rooms 9-12). The bell rooms index three `$FF`-terminated byte tables
//! at `$4A1A`/`$4A2F`/`$4A58`; `$3E2A2` loads the bank-15 sprite and patch
//! lists at `$6734` (room 13) and `$6761` (room 14).
//!
//! The bell rooms copy bank 25 `$5000` (256 tiles, 4 KiB) to VRAM `$8800`, so
//! that block is emitted next to the tilemaps and rendered here for review.

use std::{error::Error, fs, path::Path};

use crate::render::{bgp_shades, tile_colors, write_gray_png};

/// Flat ROM offset of a `$4000-$7FFF` address in a bank.
fn flat(bank: u8, addr: u16) -> Result<usize, String> {
    if !(0x4000..=0x7FFF).contains(&addr) {
        return Err(format!("${addr:04X} is outside the bank window"));
    }
    Ok(bank as usize * 0x4000 + (addr as usize - 0x4000))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlobKind {
    /// Contiguous code/handler region.
    Code,
    /// Fixed-length table (`end` is exclusive, like `Code`).
    Fixed,
    /// Byte table terminated by `$FF` (the terminator is part of the blob).
    FfTable,
    /// Two lists of 3-byte records (sprites, then patches), each terminated by
    /// a `$FF` record: `$3E2A2` advances DE past the sprite terminator record,
    /// which is exactly where the patch list starts.
    SpriteList,
}

pub struct BlobSpec {
    pub name: &'static str,
    pub bank: u8,
    pub addr: u16,
    /// Exclusive end for `Code` blobs; ignored for scanned kinds.
    pub end: u16,
    pub kind: BlobKind,
}

/// Source order is ROM order; `code` ends were taken from the disassembly's
/// next data block or next entry point.
pub const BLOBS: &[BlobSpec] = &[
    // The coin room's item table at the bank base: `$60D49` indexes
    // `$4F76[node-110] + column` into ten-byte rows.
    BlobSpec {
        name: "coin_room_items",
        bank: 24,
        addr: 0x4000,
        end: 0x4028,
        kind: BlobKind::Fixed,
    },
    BlobSpec {
        name: "bell_room_b_script",
        bank: 24,
        addr: 0x4028,
        end: 0x4512,
        kind: BlobKind::Code,
    },
    BlobSpec {
        name: "bell_room_a_script",
        bank: 24,
        addr: 0x4512,
        end: 0x4A1A,
        kind: BlobKind::Code,
    },
    BlobSpec {
        name: "bell_slots_0",
        bank: 24,
        addr: 0x4A1A,
        end: 0,
        kind: BlobKind::FfTable,
    },
    BlobSpec {
        name: "bell_slots_1",
        bank: 24,
        addr: 0x4A2F,
        end: 0,
        kind: BlobKind::FfTable,
    },
    BlobSpec {
        name: "bell_slots_2",
        bank: 24,
        addr: 0x4A58,
        end: 0,
        kind: BlobKind::FfTable,
    },
    BlobSpec {
        name: "coin_room_script",
        bank: 24,
        addr: 0x4A81,
        // `$4F5A` starts the packed lookup tables the script reads.
        end: 0x4F5A,
        kind: BlobKind::Code,
    },
    // `$60D08` reads the walk offsets at `$4FA8`, `$60D32` the item columns at
    // `$4F7E`, `$60D45` the repeat table at `$4FCD` and `$60EB5` the per-node
    // sprite ids at `$4F5A..$4F69`. (`$4C4D`'s twelve preview bytes live inside
    // `coin_room_script`; the port slices them out by offset.)
    BlobSpec {
        name: "coin_room_tables",
        bank: 24,
        addr: 0x4F5A,
        end: 0x506D,
        kind: BlobKind::Fixed,
    },
    BlobSpec {
        name: "bell_room_a_sprites",
        bank: 15,
        addr: 0x6734,
        end: 0,
        kind: BlobKind::SpriteList,
    },
    // The four coin-room sprite/patch lists `$3F3B9/$3F3F0/$3F427/$3F45E` pass
    // to `$3E2A2` (screens 9-12).
    BlobSpec {
        name: "coin_room_9_sprites",
        bank: 15,
        addr: 0x66D4,
        end: 0,
        kind: BlobKind::SpriteList,
    },
    BlobSpec {
        name: "coin_room_10_sprites",
        bank: 15,
        addr: 0x671C,
        end: 0,
        kind: BlobKind::SpriteList,
    },
    BlobSpec {
        name: "coin_room_11_sprites",
        bank: 15,
        addr: 0x6704,
        end: 0,
        kind: BlobKind::SpriteList,
    },
    BlobSpec {
        name: "coin_room_12_sprites",
        bank: 15,
        addr: 0x66EC,
        end: 0,
        kind: BlobKind::SpriteList,
    },
    BlobSpec {
        name: "bell_room_b_sprites",
        bank: 15,
        addr: 0x6761,
        end: 0,
        kind: BlobKind::SpriteList,
    },
];

/// Bank 25 `$5000`: the 256 tiles the bell rooms copy to `$8800`.
pub const BELL_TILES_ROM: usize = 25 * 0x4000 + (0x5000 - 0x4000);
pub const BELL_TILES_LEN: usize = 0x1000;

/// First `$FF` on a `stride` boundary, counting from `start`. Returns the blob
/// length including the terminator record.
fn scan_ff_terminated(rom: &[u8], start: usize, stride: usize) -> Result<usize, String> {
    for i in 0..=0x4000 / stride {
        let b = *rom
            .get(start + i * stride)
            .ok_or_else(|| format!("stream at ${start:05X} runs past the ROM"))?;
        if b == 0xFF {
            return Ok((i + 1) * stride);
        }
    }
    Err(format!("no $FF terminator within a bank from ${start:05X}"))
}

/// `$3E2A2`'s two lists: the sprite list, then the patch list at the first
/// byte after the sprite terminator record.
fn scan_sprite_lists(rom: &[u8], start: usize) -> Result<usize, String> {
    let sprites = scan_ff_terminated(rom, start, 3)?;
    let patches = scan_ff_terminated(rom, start + sprites, 3)?;
    Ok(sprites + patches)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobEntry {
    pub name: &'static str,
    pub bank: u8,
    pub addr: u16,
    pub offset: usize,
    pub len: usize,
    pub kind: &'static str,
}

fn kind_name(kind: BlobKind) -> &'static str {
    match kind {
        BlobKind::Code => "code",
        BlobKind::Fixed => "fixed",
        BlobKind::FfTable => "ff_table",
        BlobKind::SpriteList => "sprite_list",
    }
}

/// Concatenate every blob in `BLOBS` order and return the manifest entries.
pub fn build_castle_scripts_binary(baserom: &[u8]) -> Result<(Vec<u8>, Vec<BlobEntry>), String> {
    let mut out = Vec::new();
    let mut entries = Vec::with_capacity(BLOBS.len());
    for spec in BLOBS {
        let start = flat(spec.bank, spec.addr)?;
        let len = match spec.kind {
            BlobKind::Code | BlobKind::Fixed => {
                let end = flat(spec.bank, spec.end)?;
                if end <= start {
                    return Err(format!(
                        "{}: end ${:04X} is not after start",
                        spec.name, spec.end
                    ));
                }
                end - start
            }
            BlobKind::FfTable => scan_ff_terminated(baserom, start, 1)?,
            BlobKind::SpriteList => scan_sprite_lists(baserom, start)?,
        };
        let bytes = baserom
            .get(start..start + len)
            .ok_or_else(|| format!("{}: truncated at ROM ${start:05X}", spec.name))?;
        entries.push(BlobEntry {
            name: spec.name,
            bank: spec.bank,
            addr: spec.addr,
            offset: out.len(),
            len,
            kind: kind_name(spec.kind),
        });
        out.extend_from_slice(bytes);
    }
    Ok((out, entries))
}

/// Build the 384-tile VRAM image (`$8000-$9FFF`) for a bell room: the 256 bell
/// tiles sit at `$8800-$97FF` (tile numbers 128..383).
fn bell_vram_tiles(tiles: &[u8]) -> Vec<[u8; 64]> {
    let mut vram = vec![[0u8; 64]; 128];
    for chunk in tiles.as_chunks::<16>().0 {
        let mut tile = [0u8; 16];
        tile.copy_from_slice(chunk);
        vram.push(tile_colors(&tile));
    }
    vram
}

/// Render one bell-bonus screen (32x18 visible tiles) to a greyscale PNG.
pub fn render_bell_screen(baserom: &[u8], screen_id: u8, out: &Path) -> Result<(), String> {
    let spec = crate::map::SCREEN_TILEMAP_SPECS
        .iter()
        .find(|s| s.0 == screen_id)
        .ok_or_else(|| format!("no tilemap spec for screen {screen_id}"))?;
    let tilemap = baserom
        .get(spec.2..spec.2 + spec.3)
        .ok_or_else(|| format!("truncated tilemap at ROM ${:05X}", spec.2))?;
    let tiles = baserom
        .get(BELL_TILES_ROM..BELL_TILES_ROM + BELL_TILES_LEN)
        .ok_or("truncated bell-room tileset")?;
    let vram = bell_vram_tiles(tiles);
    let shades = bgp_shades(spec.5);
    let mut img = vec![shades[0]; 256 * 144];
    for ty in 0..18 {
        for tx in 0..32 {
            let tile_byte = tilemap[ty * 32 + tx];
            let vram_idx = sml2_core::map::signed_bg_tile_index(tile_byte);
            let Some(tile) = vram.get(vram_idx) else {
                continue;
            };
            for py in 0..8 {
                for px in 0..8 {
                    img[(ty * 8 + py) * 256 + tx * 8 + px] = shades[tile[py * 8 + px] as usize];
                }
            }
        }
    }
    write_gray_png(out, 256, 144, &img)
}

/// Emit `map/castle_scripts.bin`, `map/castle_scripts.json`,
/// `map/bell_room_tiles.bin` and the two bell-room renders.
pub fn emit(baserom: &[u8], extracted: &Path) -> Result<(), Box<dyn Error>> {
    let map_dir = extracted.join("map");
    fs::create_dir_all(&map_dir)?;

    let (blob, entries) =
        build_castle_scripts_binary(baserom).map_err(|e| format!("castle scripts: {e}"))?;
    fs::write(map_dir.join("castle_scripts.bin"), &blob)?;
    let manifest = serde_json::json!({
        "blobs": entries.iter().map(|e| serde_json::json!({
            "name": e.name,
            "bank": e.bank,
            "addr": format!("${:04X}", e.addr),
            "offset": e.offset,
            "len": e.len,
            "kind": e.kind,
        })).collect::<Vec<_>>(),
        "total_len": blob.len(),
    });
    fs::write(
        map_dir.join("castle_scripts.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;
    println!(
        "wrote map/castle_scripts.bin: {} blobs ({} bytes)",
        entries.len(),
        blob.len()
    );

    let tiles = baserom
        .get(BELL_TILES_ROM..BELL_TILES_ROM + BELL_TILES_LEN)
        .ok_or("truncated bell-room tileset")?;
    fs::write(map_dir.join("bell_room_tiles.bin"), tiles)?;
    println!(
        "wrote map/bell_room_tiles.bin: 256 tiles ({} bytes)",
        tiles.len()
    );

    for (id, name) in [(13u8, "bell_bonus_a"), (14u8, "bell_bonus_b")] {
        render_bell_screen(baserom, id, &map_dir.join(format!("{name}.png")))
            .map_err(|e| format!("{name}: {e}"))?;
        println!("wrote map/{name}.png");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom() -> Vec<u8> {
        vec![0u8; 0x70000]
    }

    #[test]
    fn scans_ff_tables_on_the_record_boundary() {
        let mut rom = rom();
        let start = flat(24, 0x4A1A).unwrap();
        // A $FF inside a record's third byte must not end the scan; only the
        // first byte of a record does.
        rom[start..start + 9].copy_from_slice(&[
            0x03, 0x01, 0xFF, // value 3, third byte happens to be $FF
            0x04, 0x02, 0xFF, // value 4
            0xFF, 0xFF, 0xFF, // terminator record
        ]);
        assert_eq!(scan_ff_terminated(&rom, start, 3).unwrap(), 9);
    }

    #[test]
    fn scans_sprite_and_patch_lists() {
        let mut rom = rom();
        let start = flat(15, 0x6761).unwrap();
        rom[start..start + 12].copy_from_slice(&[
            0x20, 0x10, 0x2D, // sprite
            0xFF, 0xFF, 0xFF, // sprite terminator
            0x04, 0x04, 0xFF, // patch
            0x08, 0x09, 0xFF, // patch
        ]);
        rom[start + 12..start + 15].copy_from_slice(&[0xFF, 0xFF, 0xFF]);
        assert_eq!(scan_sprite_lists(&rom, start).unwrap(), 15);
    }

    #[test]
    fn castle_blobs_are_concatenated_in_spec_order() {
        let mut rom = rom();
        for (i, spec) in BLOBS.iter().enumerate() {
            let at = flat(spec.bank, spec.addr).unwrap();
            rom[at] = i as u8 + 1;
        }
        // Terminators for the scanned blobs.
        for spec in BLOBS
            .iter()
            .filter(|s| matches!(s.kind, BlobKind::FfTable | BlobKind::SpriteList))
        {
            let at = flat(spec.bank, spec.addr).unwrap();
            rom[at] = 0xFF;
            if spec.kind == BlobKind::SpriteList {
                // Sprite terminator, then the patch list starts immediately.
                rom[at + 3] = 0xFF;
            }
        }
        let (blob, entries) = build_castle_scripts_binary(&rom).unwrap();
        assert_eq!(entries.len(), BLOBS.len());
        let by_name = |name: &str| entries.iter().find(|e| e.name == name).unwrap();
        assert_eq!(entries[0].offset, 0);
        assert_eq!(by_name("coin_room_items").len, 40);
        assert_eq!(by_name("bell_room_b_script").len, 0x4EA);
        assert_eq!(by_name("bell_room_a_script").len, 0x508);
        assert_eq!(by_name("coin_room_script").len, 0x4D9);
        assert_eq!(by_name("coin_room_tables").len, 0x113);
        assert_eq!(
            blob.len(),
            entries.last().unwrap().offset + entries.last().unwrap().len
        );
    }
}
