//! SML2 asset + level manifest extractor.
//!
//! Run from the sml2-rs repo root (or via `cargo run -p sml2-extract`):
//! reads the sibling decomp tree (`SML2_DECOMP_ROOT`, default `../sml2decomp`)
//! and writes `extracted/` here.
//!
//! Steps:
//! 1. RLE-decompress all 32 levels (`levels/levelNN.bin` -> `$3000`-byte
//!    layouts written to `extracted/levels/levelNN.decompressed.bin`).
//! 2. Parse the disassembly sources for the level headers, metatile tables,
//!    per-level properties, enemy graphics and object-spawn streams,
//!    cross-checked byte-for-byte against `baserom.gb` where addresses are
//!    known.
//! 3. Emit one structured manifest per level plus a usage index:
//!    `extracted/levels/levelNN.manifest.json` and `index.json`.

mod ending;
mod map;
mod objects;
mod parse;
mod render;

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use sml2_core::collision;
use sml2_core::level::{
    GraphicsRef, HudStrip, IndexLevel, IndexTilesetMap, LayoutSummary, LevelHeader, LevelLayout,
    LevelManifest, LevelProperties, LevelsIndex, MetatileTable, ObjectStream, SpawnRecord,
    TileAnim, TilesetMapRef, INDEX_SCHEMA, LAYOUT_HEIGHT, LAYOUT_SIZE, LAYOUT_WIDTH,
    MANIFEST_SCHEMA, METATILE_COUNT,
};
use sml2_core::object;

const LEVEL_COUNT: usize = 32;
const METATILE_TABLE_COUNT: usize = 28;
/// Per-level tile-animation config table (`UnknownData_0x35EA`, 32 bytes).
const TILEANIM_TABLE_ROM: usize = 0x35EA;
/// Bank-$1D tile-animation frame data: 16 rows x 4 phases x 64 bytes.
const TILEANIM_ROWS_ROM: usize = 0x76680;

/// Decompresses SML2 level RLE format (home.asm: Read_Level_Data, $0386).
///
/// Control byte bit 7 clear: literal byte. Bit 7 set: output `c & $7F`, then
/// a count byte (0 = a full 256-value pass) of repeats. Stops at exactly
/// `$3000` bytes.
pub fn decompress_level(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(LAYOUT_SIZE);
    let mut in_idx = 0;

    while out.len() < LAYOUT_SIZE && in_idx < input.len() {
        let b = input[in_idx];
        in_idx += 1;

        if (b & 0x80) == 0 {
            out.push(b);
        } else {
            let val = b & 0x7F;
            out.push(val);
            if out.len() >= LAYOUT_SIZE {
                break;
            }
            if in_idx >= input.len() {
                return Err("Unexpected EOF while reading RLE repeat count".to_string());
            }
            let count = input[in_idx];
            in_idx += 1;
            let repeat = if count == 0 { 256 } else { count as usize };
            for _ in 0..repeat {
                out.push(val);
                if out.len() >= LAYOUT_SIZE {
                    break;
                }
            }
        }
    }

    if out.len() == LAYOUT_SIZE {
        Ok(out)
    } else {
        Err(format!(
            "Decompressed {} bytes, expected {}",
            out.len(),
            LAYOUT_SIZE
        ))
    }
}

/// Decodes a 16-byte GB 2bpp slice into an 8x8 grid of color indices (0..=3).
#[allow(dead_code)]
pub fn decode_2bpp_tile(bytes: &[u8; 16]) -> [u8; 64] {
    let mut tile = [0u8; 64];
    for y in 0..8 {
        let low = bytes[y * 2];
        let high = bytes[y * 2 + 1];
        for x in 0..8 {
            let bit = 7 - x;
            let color = (((high >> bit) & 1) << 1) | ((low >> bit) & 1);
            tile[y * 8 + x] = color;
        }
    }
    tile
}

/// Read one packed spawn stream from the ROM: 3-byte packs, terminated by a
/// `$FF` control byte (returned inclusive).
fn read_packed_stream(rom: &[u8], start: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut p = start;
    loop {
        let c = *rom
            .get(p)
            .ok_or_else(|| format!("spawn stream at ${start:05X}: ran past the end of the ROM"))?;
        out.push(c);
        if c == 0xFF {
            return Ok(out);
        }
        let b2 = *rom
            .get(p + 1)
            .ok_or_else(|| format!("spawn stream at ${start:05X}: truncated pack"))?;
        let b3 = *rom
            .get(p + 2)
            .ok_or_else(|| format!("spawn stream at ${start:05X}: truncated pack"))?;
        out.push(b2);
        out.push(b3);
        p += 3;
    }
}

/// Expand 3-byte packs into 6-byte spawn records (mirror of `$EBBB`).
///
/// Pack `c b2 b3` becomes record `[r0..r5]`:
/// `r0 = c & $0F` (X hi), `r1 = (b2 & $1F) << 3` (X lo),
/// `r2 = b3 >> 7` (mode flag), `r3 = (b3 >> 5) & 3` (Y hi),
/// `r4 = (b3 & $1F) << 3` (Y lo),
/// `r5 = ((c >> 5) & 7) * 8 + ($40 if c bit 4) + ((b2 >> 5) & 7)` (anim).
fn decode_object_stream(packed: &[u8]) -> Result<Vec<SpawnRecord>, String> {
    let mut records = Vec::new();
    let mut p = 0;
    while p < packed.len() {
        let c = packed[p];
        if c == 0xFF {
            break;
        }
        if p + 2 >= packed.len() {
            return Err("spawn stream: truncated pack at the stream end".to_string());
        }
        let b2 = packed[p + 1];
        let b3 = packed[p + 2];
        let r0 = c & 0x0F;
        let r1 = (b2 & 0x1F) << 3;
        let r2 = (b3 >> 7) & 0x01;
        let r3 = (b3 >> 5) & 0x03;
        let r4 = (b3 & 0x1F) << 3;
        let r5 = ((c >> 5) & 0x07) * 8 + if c & 0x10 != 0 { 0x40 } else { 0 } + ((b2 >> 5) & 0x07);
        records.push(SpawnRecord {
            x: ((r0 as u16) << 8) | r1 as u16,
            y: ((r3 as u16) << 8) | r4 as u16,
            anim_index: r5,
            mode_flag: r2,
            sprite_id: 0,
            hitbox: [0; 4],
            raw: [r0, r1, r2, r3, r4, r5],
        });
        p += 3;
    }
    Ok(records)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Workspace root (`sml2-rs/`), via this crate's manifest dir.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Byte-identical disassembly repo root.
///
/// `SML2_DECOMP_ROOT` overrides; otherwise the sibling `../sml2decomp` of
/// this repo is used.
fn decomp_root() -> Result<PathBuf, Box<dyn Error>> {
    if let Ok(p) = env::var("SML2_DECOMP_ROOT") {
        let pb = PathBuf::from(p);
        if !pb.join("baserom.gb").is_file() {
            return Err(format!("SML2_DECOMP_ROOT={} has no baserom.gb", pb.display()).into());
        }
        return Ok(pb);
    }
    let sibling = repo_root().join("../sml2decomp");
    let sibling = sibling.canonicalize().unwrap_or(sibling);
    if sibling.join("baserom.gb").is_file() {
        return Ok(sibling);
    }
    Err("sml2decomp not found — set SML2_DECOMP_ROOT to the decomp repo root".into())
}

/// Find the active section bank at a `INCLUDE "..."` site in main.asm.
fn bank_at_include(src: &str, needle: &str) -> Option<u8> {
    let mut cur: Option<u8> = None;
    for line in src.lines() {
        let code = line.split(';').next().unwrap_or("").trim();
        if code.starts_with("SECTION ") {
            cur = code
                .find("BANK[$")
                .and_then(|p| u8::from_str_radix(&code[p + 6..p + 8], 16).ok());
        }
        if code.contains(needle) {
            return cur;
        }
    }
    None
}

/// Cached file read (the extra-block slices reuse the same files often).
fn read_cached<'a>(
    cache: &'a mut std::collections::HashMap<String, Vec<u8>>,
    decomp: &Path,
    path: &str,
) -> Result<&'a [u8], String> {
    if !cache.contains_key(path) {
        let full = decomp.join(path);
        let data = fs::read(&full).map_err(|e| format!("{}: {e}", full.display()))?;
        cache.insert(path.to_string(), data);
    }
    Ok(&cache[path])
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== SML2 asset + level manifest extractor ===");

    let decomp = decomp_root()?;
    let extracted = repo_root().join("extracted");
    println!("decomp root: {}", decomp.display());
    println!("extracted out: {}", extracted.display());

    for f in [
        "baserom.gb",
        "main.asm",
        "home.asm",
        "levels/levelheaders.asm",
        "levels/tilesets.asm",
        "levels/enemysets.asm",
    ] {
        let p = decomp.join(f);
        if !p.exists() {
            return Err(format!("{} not found under {}", f, decomp.display()).into());
        }
    }

    let baserom = fs::read(decomp.join("baserom.gb"))?;
    ending::emit(&baserom, &extracted)?;
    let main_src = fs::read_to_string(decomp.join("main.asm"))?;
    let home_src = fs::read_to_string(decomp.join("home.asm"))?;
    let secret_entries = parse::parse_secret_entries(&home_src, &baserom)?;
    let headers_src = fs::read_to_string(decomp.join("levels/levelheaders.asm"))?;
    let tilesets_src = fs::read_to_string(decomp.join("levels/tilesets.asm"))?;
    let enemysets_src = fs::read_to_string(decomp.join("levels/enemysets.asm"))?;

    // ---- parse the disassembly sources ----
    let main_facts = parse::parse_main_asm(&main_src)?;

    let hdr_bank = bank_at_include(&main_src, "INCLUDE \"levels/levelheaders.asm\"")
        .ok_or("could not find the levelheaders.asm include site")?;
    let mut header_label_rom = parse::parse_labels_in_file(&headers_src, Some(hdr_bank));
    // Only Level_Header_01 carries a ;$addr comment; the 32 headers are
    // contiguous 20-byte records, so derive the rest. (The baserom
    // cross-check below proves the assumption per header.)
    let first_rom = header_label_rom
        .get("Level_Header_01")
        .copied()
        .ok_or("Level_Header_01: no ROM address comment")?;
    for i in 2..=LEVEL_COUNT {
        header_label_rom
            .entry(format!("Level_Header_{i:02}"))
            .or_insert(first_rom + (i as u32 - 1) * 20);
    }

    let headers = parse::parse_level_headers(&headers_src, &main_facts, &header_label_rom)?;
    let tilesets = parse::parse_gfx_pointer_table(&tilesets_src, "LevelTileSets")?;
    let enemy_sets = parse::parse_gfx_pointer_table(&enemysets_src, "LevelEnemySets")?;

    let props_unk = parse::parse_byte_table(&home_src, "LevelPropertiesUNK")?;
    let props_scroll = parse::parse_byte_table(&home_src, "LevelPropertiesScroll")?;
    let props_space = parse::parse_byte_table(&home_src, "LevelPropertiesSpace")?;
    for (name, t) in [
        ("UNK", &props_unk),
        ("Scroll", &props_scroll),
        ("Space", &props_space),
    ] {
        if t.len() != LEVEL_COUNT {
            return Err(format!("LevelProperties{name}: {} entries, expected 32", t.len()).into());
        }
    }

    // Boss/special GFX path ($25AF): the $26B5 table holds bank-$08 CPU
    // addresses of metatile maps, indexed by sLevelGfxIndex.
    let boss_ptrs = parse::parse_word_table(&home_src, "UnknownData_0x26B5")?;
    let mut boss_map_usage: Vec<(u8, u8)> = Vec::new(); // (table index, gfx index)
    for (gfx_idx, v) in boss_ptrs.iter().enumerate() {
        let rom = 0x20000u32 + (*v as u32 - 0x4000);
        let label = main_facts
            .rom_to_label
            .get(&rom)
            .ok_or_else(|| format!("$26B5[{gfx_idx}] = ${v:04X}: no label at ROM ${rom:05X}"))?;
        let nn: u8 = label
            .strip_prefix("Level_TileMap_")
            .ok_or_else(|| format!("$26B5[{gfx_idx}] resolves to {label}, not a Level_TileMap_NN"))?
            .parse()
            .map_err(|e| format!("bad map index in {label}: {e}"))?;
        boss_map_usage.push((nn, gfx_idx as u8));
    }

    // Object-spawn stream pointer table (bank $03, `UnknownData_0xE037`,
    // indexed by sCurLevel, read by $EBBB; values are bank-3 CPU addresses).
    let stream_ptrs = parse::parse_word_table(&main_src, "UnknownData_0xE037")?;
    if stream_ptrs.len() != LEVEL_COUNT {
        return Err(format!(
            "UnknownData_0xE037: {} entries, expected 32",
            stream_ptrs.len()
        )
        .into());
    }
    {
        let rom = parse::label_rom(&main_facts, "UnknownData_0xE037")
            .ok_or("UnknownData_0xE037: no ROM address")? as usize;
        let mut expected = Vec::with_capacity(stream_ptrs.len() * 2);
        for w in &stream_ptrs {
            expected.push(*w as u8);
            expected.push((*w >> 8) as u8);
        }
        if baserom[rom..rom + expected.len()] != expected[..] {
            return Err(format!("UnknownData_0xE037 differs from baserom.gb at ${rom:05X}").into());
        }
    }

    // The scene scripts, written once for the whole ROM rather than per level:
    // the 32-entry `dw` table at bank 5 `$4125` followed by its payload at
    // `$4165`. The scene VM (`$20A4`) looks a script up as
    // `$4125 + sSceneId * 2` and then executes the pointed-at bytes in bank 5,
    // so the port keeps the table and its payload in one blob and treats the
    // pointers as bank-5 addresses. `$41E5` is where the title-screen gfx
    // begin, so the region is exactly `$4125..$41E5` = 192 bytes (64 table +
    // 128 payload). The only label in the region is the 4 title-screen bytes
    // at `$4121`, hence the +4.
    let script_rom = parse::label_rom(&main_facts, "UnknownData_0x14121")
        .ok_or("UnknownData_0x14121: no ROM address")? as usize
        + 4;
    fs::create_dir_all(extracted.join("scenes"))?;
    fs::write(
        extracted.join("scenes/script_table.bin"),
        &baserom[script_rom..script_rom + 192],
    )?;

    // The `$6x` opcode's tileset swap (`$25AF`), written once for the whole ROM
    // because all three of its tables are global and keyed only by
    // `sLevelGfxIndex`: `$268B` (bank pairs), `$2699` (source `dw` pairs) and
    // `$26B5` (bank-8 metatile-map pointers). Seven entries — every arena script
    // in the ROM uses one of them (indices 0..6).
    let mut tileset_entries = Vec::new();
    for idx in 0..7usize {
        let bg_bank = baserom[0x268B + idx * 2] as usize;
        let obj_bank = baserom[0x268B + idx * 2 + 1] as usize;
        let word = |off: usize| baserom[off] as usize | ((baserom[off + 1] as usize) << 8);
        let bg_addr = word(0x2699 + idx * 4);
        let obj_addr = word(0x2699 + idx * 4 + 2);
        let map_addr = word(0x26B5 + idx * 2);
        if !(0x4000..0x8000).contains(&bg_addr)
            || !(0x4000..0x8000).contains(&obj_addr)
            || !(0x4000..0x8000).contains(&map_addr)
        {
            return Err(
                format!("$25AF index {idx}: a source address is outside $4000-$7FFF").into(),
            );
        }
        let bg_rom = bg_bank * 0x4000 + (bg_addr - 0x4000);
        let obj_rom = obj_bank * 0x4000 + (obj_addr - 0x4000);
        let map_rom = 8 * 0x4000 + (map_addr - 0x4000);
        let label = main_facts
            .rom_to_label
            .get(&(map_rom as u32))
            .ok_or_else(|| {
                format!("$26B5[{idx}] = ${map_addr:04X}: no label at ROM ${map_rom:05X}")
            })?
            .clone();
        let map_index: u8 = label
            .strip_prefix("Level_TileMap_")
            .ok_or_else(|| format!("$26B5[{idx}] resolves to {label}, not a Level_TileMap_NN"))?
            .parse()
            .map_err(|e| format!("bad map index in {label}: {e}"))?;
        // The metatile table must be the one the disassembly ships for that
        // index — the same cross-check the usage index does.
        let file = format!("tilemaps/mappings/{map_index:02}.bin");
        let known = fs::read(decomp.join(&file))?;
        if baserom[map_rom..map_rom + 512] != known[..] {
            return Err(format!("{file} differs from baserom.gb at ${map_rom:05X}").into());
        }
        tileset_entries.push(sml2_core::tileset::TilesetEntry {
            index: idx as u8,
            bg_bank: bg_bank as u8,
            bg_addr: bg_addr as u16,
            obj_bank: obj_bank as u8,
            obj_addr: obj_addr as u16,
            map_index,
            map_label: label,
            bg_tiles: baserom[bg_rom..bg_rom + 0x600].to_vec(),
            obj_tiles: baserom[obj_rom..obj_rom + 0x400].to_vec(),
            metatiles: baserom[map_rom..map_rom + 512].to_vec(),
        });
    }
    let extra_rom = 7 * 0x4000 + (0x7800 - 0x4000);
    let gfx_tilesets = sml2_core::tileset::Tilesets {
        schema: sml2_core::tileset::TILESET_SCHEMA.to_string(),
        notes: vec![
            "$25AF (the $6x scene opcode's handler) swaps the level's graphics and metatile \
             definitions in place: bank 7 $7800 -> VRAM $8E80 (0x50, index-independent), \
             bank $268B[i*2] $2699[i*4] -> $9200 (0x600), bank $268B[i*2+1] $2699[i*4+2] -> \
             $8B00 (a literal 0x400, which overruns the 0x380-byte enemy region by 0x20 and \
             lands on the first two tiles of the $8E80 block), and $26B5[i] -> sMetatileMap."
                .to_string(),
            "The copies land inside the $8800-$97FF tile block the port holds as \
             levelNN.bgblock.bin ($8E80 = 0x680, $9200 = 0xA00, $8B00 = 0x300), and the OBJ \
             space $8000-$8FFF shares that VRAM ($8800-$8FFF = objblock.bin's high half), so \
             applying a swap repatches both."
                .to_string(),
        ],
        extra_8e80: baserom[extra_rom..extra_rom + 0x50].to_vec(),
        entries: tileset_entries,
    };
    fs::write(
        extracted.join("scenes/tilesets.json"),
        serde_json::to_string_pretty(&gfx_tilesets)?,
    )?;
    println!(
        "wrote scenes/tilesets.json: {} gfx-index entries, maps {}",
        gfx_tilesets.entries.len(),
        gfx_tilesets
            .entries
            .iter()
            .map(|e| e.map_index.to_string())
            .collect::<Vec<_>>()
            .join("/")
    );

    // The `$6x` reload's LEVEL DATA (`$0361` with the hardcoded
    // `sLevelLayoutIndex = 3` and `sLevelBank = 19`). This is the piece that
    // looks like a mistake and is not: the ROM reloads the layout at
    // bank 19 `$42F0[3]` — one blob, the same for every arena — so every boss
    // room in the game is authored as a screen inside *that* layout, and the
    // per-arena look comes from the tileset swap above it. The port renders a
    // level's own layout at the arena's cell coordinates, which is why a
    // swapped-tileset arena came out blank.
    //
    // Cross-checked the same way the level layouts are: the blob must be the
    // decompressed layout of whichever level header names (bank 19, index 3).
    let arena_src = 19 * 0x4000 + (0x42F0 - 0x4000) + 3 * 2;
    let arena_ptr = (baserom[arena_src] as usize) | ((baserom[arena_src + 1] as usize) << 8);
    if !(0x4000..0x8000).contains(&arena_ptr) {
        return Err(format!("$42F0[3] in bank 19 = ${arena_ptr:04X}, outside $4000-$7FFF").into());
    }
    let arena_rom = 19 * 0x4000 + (arena_ptr - 0x4000);
    let arena_layout = decompress_level(&baserom[arena_rom..])?;
    fs::write(extracted.join("scenes/arena_layout.bin"), &arena_layout)?;
    println!(
        "wrote scenes/arena_layout.bin: {} bytes from bank 19 ${arena_ptr:04X}",
        arena_layout.len()
    );

    // Bank 2 boss spawn tables ($8D9C / $8DFD / $8E6C):
    // $4E3F..$4E51: 8 pointers to 3-byte records (count, hits, sprite)
    // $4E9A..$4EAC: 8 pointers to sObject0Def (18-byte records)
    // $4EAC..$4EBC: 8 pointers to sObject1Def.. (count * 18-byte records)
    let mut boss_spawns = Vec::new();
    for idx in 0..9usize {
        let rom_bank2 = |addr: usize| -> usize { 2 * 0x4000 + (addr - 0x4000) };
        let t0_ptr = rom_bank2(0x4E3F + idx * 2);
        let t0_addr = (baserom[t0_ptr] as usize) | ((baserom[t0_ptr + 1] as usize) << 8);
        let t0_off = rom_bank2(t0_addr);
        let count = baserom[t0_off];
        let hits = baserom[t0_off + 1];
        let sprite = baserom[t0_off + 2];

        let obj0_ptr = rom_bank2(0x4E9A + idx * 2);
        let obj0_addr = (baserom[obj0_ptr] as usize) | ((baserom[obj0_ptr + 1] as usize) << 8);
        let obj0_off = rom_bank2(obj0_addr);
        let obj0_def = baserom[obj0_off..obj0_off + 18].to_vec();

        // The `$4EAC` table has 8 entries (indices 0..7). The 9th record
        // (`$4E69`) carries `sObjectDefCount` 0, and `$8E6C` returns on that
        // before it ever dereferences the table, so entry 8 stages no
        // sub-objects and no pointer is read for it.
        let mut subs = Vec::new();
        if count > 0 {
            let obj1_ptr = rom_bank2(0x4EAC + idx * 2);
            let obj1_addr = (baserom[obj1_ptr] as usize) | ((baserom[obj1_ptr + 1] as usize) << 8);
            let obj1_off = rom_bank2(obj1_addr);
            for c in 0..(count as usize) {
                let start = obj1_off + c * 18;
                subs.push(baserom[start..start + 18].to_vec());
            }
        }

        boss_spawns.push(serde_json::json!({
            "def_index": idx,
            "count": count,
            "hits": hits,
            "sprite": sprite,
            "obj0_def": obj0_def,
            "sub_obj_defs": subs,
        }));
    }
    fs::write(
        extracted.join("scenes/boss_spawns.json"),
        serde_json::to_string_pretty(&boss_spawns)?,
    )?;

    // The overworld's parallel node tables (bank 15): `$4218` (level index) and
    // `$650E` (progress slot) are indexed by node id, 130 bytes each; `$6591`
    // (the key/direction mask `$3E1D7` tests) is indexed by *progress slot*, and
    // the slot range the extraction found runs `$00`-`$29`, so 42 bytes are
    // taken. Note the disassembly gives no label between `$650E` and `$67BE` —
    // the whole region is one 16-byte-`db`-split blob — so the mask table's
    // exact length is inferred from the values (they stop looking like direction
    // masks after index 40) and from the slot range. The 42nd byte is whatever
    // data follows the table, which is what the ROM reads for a slot of `$29`
    // too; the port takes the same byte rather than pretending to know better.
    let nodes_rom = parse::label_rom(&main_facts, "UnknownData_0x3C218")
        .ok_or("UnknownData_0x3C218: no ROM address")? as usize;
    let slots_rom = parse::label_rom(&main_facts, "UnknownData_0x3E50E")
        .ok_or("UnknownData_0x3E50E: no ROM address")? as usize;
    fs::create_dir_all(extracted.join("map"))?;
    let mut nodes = Vec::with_capacity(2 * 130);
    nodes.extend_from_slice(&baserom[nodes_rom..nodes_rom + 130]);
    nodes.extend_from_slice(&baserom[slots_rom..slots_rom + 130]);
    // `$650E`'s slot region runs 131 bytes — the 130 node slots plus a trailing
    // `$FE` at `$6590` — so `$6591` is `slots_rom + 131`, which the ROM's own
    // 130-byte node arrays do not reach.
    nodes.extend_from_slice(&baserom[slots_rom + 131..slots_rom + 131 + 42]);
    fs::write(extracted.join("map/nodes.bin"), &nodes)?;

    // The overworld's walk graph (bank 24). The base is not a labelled address —
    // the cursor code loads the table with `ld hl, $563A` (main.asm:18936) while
    // bank 24 is banked in, and `$563A` is a bank-24 *window* address, i.e. flat
    // `$6163A`. The disassembly's `UnknownData_0x61A40` label (main.asm:37954,
    // "object records pointed to by OW_Movement_Pointers") is what the pointers
    // point into, and it must therefore sit *past* the table: 128 nodes x 8 bytes
    // ends at `$61A3A`, and the first stream (node $06's Down) starts there.
    //
    // 128, not the 130 nodes the other node tables carry: node $80/$81 read past
    // the table into the following data, so the port stops at $7F and the two
    // bytes at $6163A + 128*8 are the graph's first stream, not a table row.
    const WALK_NODES: usize = 128;
    let walk_rom = 0x6163Ausize;
    let walk_anchor = parse::label_rom(&main_facts, "UnknownData_0x61A40")
        .ok_or("UnknownData_0x61A40: no ROM address")? as usize;
    if walk_anchor < walk_rom + WALK_NODES * 8 {
        return Err(format!(
            "walk table base ${walk_rom:05X} + {WALK_NODES} nodes runs past \
             UnknownData_0x61A40 (${walk_anchor:05X})"
        )
        .into());
    }
    // The streams all live between the table's end and the end of the tallest one;
    // follow each stream to its `$00` terminator plus the arrival node byte, so
    // the blob never truncates a stream (a truncated stream would read as `$00`
    // and end a walk at the wrong node).
    let mut walk_end = walk_rom + WALK_NODES * 8;
    for node in 0..WALK_NODES {
        for dir in 0..4 {
            let at = walk_rom + node * 8 + dir * 2;
            let ptr = baserom[at] as usize | (baserom[at + 1] as usize) << 8;
            if ptr == 0xFFFF {
                continue;
            }
            // Bank-24 window address -> flat.
            let mut end = 24 * 0x4000 + (ptr - 0x4000);
            let mut guard = 0;
            while baserom[end] != 0 {
                end += 1;
                guard += 1;
                if guard > 0x400 {
                    return Err(format!("walk stream at ${ptr:04X} has no terminator").into());
                }
            }
            walk_end = walk_end.max(end + 2);
        }
    }
    if walk_end > baserom.len() {
        return Err(format!("walk stream region ends at ${walk_end:05X}, past the ROM").into());
    }
    fs::write(extracted.join("map/walk.bin"), &baserom[walk_rom..walk_end])?;
    println!(
        "wrote map/walk.bin: {WALK_NODES} nodes x 8 bytes at ${walk_rom:05X}, streams to ${walk_end:05X}"
    );

    // The per-node camera/cursor records: `$3EEA4` reads the six bytes at
    // `$5EE6 + node*8` (a bank-24 window address, so flat `$61EE6`) into the map
    // camera staging and the cursor — X hi, X lo, Y hi, Y lo, cursor X, cursor Y.
    // Also 128 records: bytes 6-7 are `$00` in every one of them and non-zero from
    // node $80 on, which is the assertion below.
    const NODE_RECORD_LEN: usize = 8;
    let records_rom = 24 * 0x4000 + (0x5EE6 - 0x4000);
    let records = &baserom[records_rom..records_rom + WALK_NODES * NODE_RECORD_LEN];
    if records
        .as_chunks::<NODE_RECORD_LEN>()
        .0
        .iter()
        .any(|r| r[6] != 0 || r[7] != 0)
    {
        return Err(format!(
            "map node record at ${records_rom:05X}: bytes 6/7 are not zero inside \
             the {WALK_NODES}-node table"
        )
        .into());
    }
    let past = &baserom[records_rom + WALK_NODES * NODE_RECORD_LEN..][..NODE_RECORD_LEN];
    if past[6] == 0 && past[7] == 0 {
        return Err(format!(
            "map node record table: node $80 at ${:05X} still looks like a record",
            records_rom + WALK_NODES * NODE_RECORD_LEN
        )
        .into());
    }
    fs::write(extracted.join("map/node_records.bin"), records)?;
    println!(
        "wrote map/node_records.bin: {WALK_NODES} records at ${records_rom:05X} (camera + cursor)"
    );

    map::emit(&baserom, &extracted)?;

    // Per-level GFX config (UnknownData_0x360A -> sLevelGfxConfig): the high
    // nibble selects the two "extra" tile blocks loaded by LoadMarioGFX
    // ($06C0): $8800 <- bank $1B [$7000 + n*$300], $8E80 <- bank $07
    // [$6A00 + n*$380].
    let gfx_config = parse::parse_byte_table(&home_src, "UnknownData_0x360A")?;
    if gfx_config.len() != LEVEL_COUNT {
        return Err(format!(
            "UnknownData_0x360A: {} entries, expected 32",
            gfx_config.len()
        )
        .into());
    }

    // ROM address -> INCBIN'd file resolver (for slicing the extra blocks).
    let mut incbin_ranges: Vec<(u32, String, u64)> = Vec::new();
    for (label, file) in &main_facts.incbin {
        if let (Some(rom), Ok(md)) = (
            parse::label_rom(&main_facts, label),
            fs::metadata(decomp.join(file)),
        ) {
            incbin_ranges.push((rom, file.clone(), md.len()));
        }
    }
    let resolve_rom = |rom: u32| -> Option<(String, usize)> {
        incbin_ranges
            .iter()
            .find(|(r, _, len)| rom >= *r && (rom as u64) < *r as u64 + *len)
            .map(|(r, f, _)| (f.clone(), (rom - r) as usize))
    };

    // ---- object taxonomy tables (bank $02/$03) ----
    let obj_tables = objects::extract(&baserom).map_err(|e| e.to_string())?;
    fs::create_dir_all(&extracted)?;
    fs::write(
        extracted.join("objects.json"),
        serde_json::to_string_pretty(&obj_tables)? + "\n",
    )?;
    println!(
        "wrote objects.json: anim maps {}A/{}B, hitbox records {}A/{}B, sprite mappings {}A/{}B, anim profiles {}A/{}B",
        obj_tables.anim_to_sprite_a.len(),
        obj_tables.anim_to_sprite_b.len(),
        obj_tables.hitbox_records_a.len(),
        obj_tables.hitbox_records_b.len(),
        obj_tables.sprite_mappings_a.len(),
        obj_tables.sprite_mappings_b.len(),
        obj_tables.anim_profiles_a.len(),
        obj_tables.anim_profiles_b.len()
    );

    // ---- per-level pass: decompress layouts, cross-check, emit manifests ----
    let anim_table: Vec<u8> = baserom
        .get(TILEANIM_TABLE_ROM..TILEANIM_TABLE_ROM + LEVEL_COUNT)
        .ok_or("tile-anim config table out of bounds")?
        .to_vec();
    let anim_rows = baserom
        .get(TILEANIM_ROWS_ROM..TILEANIM_ROWS_ROM + 16 * 0x100)
        .ok_or("tile-anim frame rows out of bounds")?;
    fs::write(extracted.join("tileanim.bin"), anim_rows)?;
    println!("wrote tileanim.bin: 16 rows x 4 phases x 64 bytes (bank $1D $6680)");

    // ---- HUD strip (window layer $9C00, template $07EA + digit columns) ----
    let hud = HudStrip {
        template: baserom
            .get(0x07EA..0x07EA + 20)
            .ok_or("HUD template out of bounds")?
            .to_vec(),
        window_x: 7,
        window_y: 136,
        digit_columns: vec![2, 3, 7, 8, 9, 13, 14, 17, 18, 19],
    };
    fs::write(
        extracted.join("hud.json"),
        serde_json::to_string_pretty(&hud)? + "\n",
    )?;
    println!("wrote hud.json: 20-tile status bar (window at 7,136; 10 digit columns)");

    let out_dir = extracted.join("levels");
    fs::create_dir_all(&out_dir)?;
    let renders_dir = extracted.join("renders");
    fs::create_dir_all(&renders_dir)?;
    let mut file_cache: std::collections::HashMap<String, Vec<u8>> =
        std::collections::HashMap::new();

    let mut index_levels: Vec<IndexLevel> = Vec::new();
    let mut map_used_by: Vec<Vec<u8>> = vec![Vec::new(); METATILE_TABLE_COUNT + 1];

    for (i, raw) in headers.iter().enumerate() {
        let level_id = i as u8;
        let nn = i + 1;

        if raw.bytes[10] as usize != i {
            return Err(format!(
                "{}: level number byte ${:02X} != expected {}",
                raw.label, raw.bytes[10], i
            )
            .into());
        }

        // header bytes vs the reference ROM
        let hdr_rom = header_label_rom[&raw.label] as usize;
        if baserom[hdr_rom..hdr_rom + 20] != raw.bytes[..] {
            return Err(format!(
                "{}: parsed bytes differ from baserom.gb at ${hdr_rom:05X}",
                raw.label
            )
            .into());
        }
        let header = LevelHeader::from_bytes(&raw.bytes);

        // metatile table file for this level
        let map_nn: u8 = raw
            .tileset_label
            .strip_prefix("Level_TileMap_")
            .ok_or_else(|| format!("{}: bad tileset label {}", raw.label, raw.tileset_label))?
            .parse()
            .map_err(|e| format!("{}: bad map index: {e}", raw.label))?;
        if map_nn as usize > METATILE_TABLE_COUNT {
            return Err(format!("{}: map index {map_nn} out of range", raw.label).into());
        }
        let map_file = format!("tilemaps/mappings/{map_nn:02}.bin");
        let map_bytes = fs::read(decomp.join(&map_file))?;
        if map_bytes.len() != 512 {
            return Err(format!("{map_file}: {} bytes, expected 512", map_bytes.len()).into());
        }
        let map_rom = main_facts
            .label_to_rom
            .get(&raw.tileset_label)
            .copied()
            .ok_or_else(|| format!("{}: no ROM address for {}", raw.label, raw.tileset_label))?
            as usize;
        if baserom[map_rom..map_rom + 512] != map_bytes[..] {
            return Err(format!("{map_file} differs from baserom.gb at ${map_rom:05X}").into());
        }
        let metatiles = MetatileTable::from_bytes(&map_bytes)?;

        // decompress the level layout
        let in_bin = format!("levels/level{nn:02}.bin");
        let src = fs::read(decomp.join(&in_bin))?;
        let decompressed = decompress_level(&src).map_err(|e| format!("{in_bin}: {e}"))?;
        let layout_file = format!("level{nn:02}.decompressed.bin");
        fs::write(out_dir.join(&layout_file), &decompressed)?;
        let layout = LevelLayout::from_bytes(&decompressed)?;
        // A header naming (bank 19, layout index 3) is the level whose data the
        // `$6x` reload puts in every boss arena — so its layout must be the
        // blob `scenes/arena_layout.bin` carries. This is the cross-check that
        // keeps the arena's room data honest: if the reload ever picked a
        // different blob, this fails instead of silently rendering the wrong
        // room.
        {
            let h = LevelHeader::from_bytes(&raw.bytes);
            if h.level_data_bank == 19 && h.layout_index == 3 && decompressed != arena_layout {
                return Err(format!(
                    "{} claims (bank 19, layout 3) but differs from scenes/arena_layout.bin",
                    raw.label
                )
                .into());
            }
        }

        // The level's two 48-byte 3×16 cell maps — `sScreenEdgeMap` and the
        // parallel `sSceneMap` (`$0F0B` latches `sSceneId` from the cell under
        // Mario). `$03F1` copies both from the level data bank, at the header
        // pointers `[$4000]` and `[$4002]` respectively, plus
        // `(sLevelLayoutIndex & $0F) * 48` (the ROM builds that offset as
        // `idx*3` with its nibbles swapped — `idx * 3 * 16`). They are handled
        // together so the two files can never drift apart.
        //
        // Edge bits: bit0 right, bit1 left, bit2 up, bit3 down — `$0B39` reads
        // them into `$FF98` and the vertical follow's hard-edge branches test
        // them. Scene map: the id `$0F0B` stores into `sSceneId`, which indexes
        // the bank-5 script table at `$4125`; 14 of the 32 levels are nonzero,
        // and level 1's is all zeros (which is why no level-1 capture can
        // reach a scene).
        let map_bank = header.level_data_bank as usize;
        for (ptr_at, suffix, what) in [
            (0usize, "edge", "sScreenEdgeMap"),
            (2, "scene", "sSceneMap"),
        ] {
            let base = baserom[map_bank * 0x4000 + ptr_at] as usize
                | ((baserom[map_bank * 0x4000 + ptr_at + 1] as usize) << 8);
            if !(0x4000..0x8000).contains(&base) {
                return Err(format!(
                    "{}: {what} pointer ${base:04X} (header +{ptr_at}) is outside bank {map_bank}",
                    raw.label
                )
                .into());
            }
            let off =
                map_bank * 0x4000 + (base - 0x4000) + (header.layout_index & 0x0F) as usize * 48;
            let file = format!("level{nn:02}.{suffix}.bin");
            fs::write(out_dir.join(&file), &baserom[off..off + 48])?;
        }

        // graphics
        let ts = &tilesets[i];
        let gfx_bank = *main_facts
            .sections
            .get(&ts.gfx_label)
            .ok_or_else(|| format!("{}: no section bank found", ts.gfx_label))?;
        let gfx_file = main_facts
            .incbin
            .get(&ts.gfx_label)
            .ok_or_else(|| format!("{}: no INCBIN file found", ts.gfx_label))?
            .clone();

        // enemy graphics (LevelEnemySets, same 4-byte record shape)
        let es = &enemy_sets[i];
        let enemy_bank = *main_facts
            .sections
            .get(&es.gfx_label)
            .ok_or_else(|| format!("{}: no section bank found", es.gfx_label))?;
        let enemy_file = main_facts
            .incbin
            .get(&es.gfx_label)
            .ok_or_else(|| format!("{}: no INCBIN file found", es.gfx_label))?
            .clone();

        // object-spawn stream (packed source in bank $03)
        let stream_cpu = stream_ptrs[i];
        if !(0x4000..0x8000).contains(&stream_cpu) {
            return Err(format!(
                "{}: stream pointer ${stream_cpu:04X} outside the $4000-$7FFF window",
                raw.label
            )
            .into());
        }
        let stream_rom = 0x03 * 0x4000 + (stream_cpu as u32 - 0x4000);
        let packed = read_packed_stream(&baserom, stream_rom as usize)?;
        let mut records = decode_object_stream(&packed)?;
        // Resolve each record's anim -> sprite id and 4-byte extents through
        // the mode's tables (see sml2-core/object.rs for the chain).
        for r in &mut records {
            let sprite = obj_tables
                .sprite_for(level_id, r.anim_index)
                .ok_or_else(|| {
                    format!(
                        "level {level_id}: anim ${:02X} has no anim->sprite entry",
                        r.anim_index
                    )
                })?;
            r.sprite_id = sprite;
            r.hitbox = obj_tables.hitbox_for(level_id, sprite).ok_or_else(|| {
                format!("level {level_id}: sprite ${sprite:02X} has no extent record")
            })?;
        }
        if !records.windows(2).all(|w| w[0].x <= w[1].x) {
            return Err(format!("{}: spawn stream is not sorted by X", raw.label).into());
        }
        let spawn_count = records.len() as u32;

        // ---- compose the level's BG tile block ($8800-$97FF) and render ----
        // $8800 <- bank $1B [$7000 + n*$300] (48 tiles); $8B00 <- enemy set
        // (56); $8E80 <- bank $07 [$6A00 + n*$380] (56); $9200 <- tileset (96).
        let nibble = (gfx_config[i] >> 4) as u32;
        let (extra_a_file, extra_a_off) =
            resolve_rom(0x1EA00 + nibble * 0x380).ok_or_else(|| {
                format!(
                    "{}: no INCBIN file covers ROM ${:05X}",
                    raw.label,
                    0x1EA00 + nibble * 0x380
                )
            })?;
        let (extra_b_file, extra_b_off) =
            resolve_rom(0x6F000 + nibble * 0x300).ok_or_else(|| {
                format!(
                    "{}: no INCBIN file covers ROM ${:05X}",
                    raw.label,
                    0x6F000 + nibble * 0x300
                )
            })?;
        let mut bg_vram = vec![0u8; render::BG_VRAM_LEN];
        {
            let b = read_cached(&mut file_cache, &decomp, &extra_b_file)?;
            let s = b
                .get(extra_b_off..extra_b_off + 0x300)
                .ok_or_else(|| format!("{extra_b_file}: too short for the $8800 block"))?;
            bg_vram[0x000..0x300].copy_from_slice(s);
        }
        {
            let b = read_cached(&mut file_cache, &decomp, &enemy_file)?;
            let s = b
                .get(0..0x380)
                .ok_or_else(|| format!("{enemy_file}: too short for the enemy set"))?;
            bg_vram[0x300..0x680].copy_from_slice(s);
        }
        {
            let b = read_cached(&mut file_cache, &decomp, &extra_a_file)?;
            let s = b
                .get(extra_a_off..extra_a_off + 0x380)
                .ok_or_else(|| format!("{extra_a_file}: too short for the $8E80 block"))?;
            bg_vram[0x680..0xA00].copy_from_slice(s);
        }
        {
            let b = read_cached(&mut file_cache, &decomp, &gfx_file)?;
            let s = b
                .get(0..0x600)
                .ok_or_else(|| format!("{gfx_file}: too short for the tileset"))?;
            bg_vram[0xA00..0x1000].copy_from_slice(s);
        }
        fs::write(out_dir.join(format!("level{nn:02}.bgblock.bin")), &bg_vram)?;
        // OBJ tile space $8000-$8FFF: low half = GFX_Mario variant by the gfx
        // config's low nibble (0 = Mario, 1 = MarioDark, else MarioMoon — see
        // LoadMarioGFX $0669), high half = the $8800 block (bank $1B / enemy
        // set start) = bg_vram[0..$800].
        let mario_gfx_file = match gfx_config[i] & 0x0F {
            0 => "gfx/mario.2bpp",
            1 => "gfx/mariodark.2bpp",
            _ => "gfx/mariomoon.2bpp",
        };
        let mario_gfx = fs::read(decomp.join(mario_gfx_file))?;
        let mut objblock = mario_gfx
            .get(..0x800)
            .ok_or_else(|| format!("{mario_gfx_file}: too short for the $8000 block"))?
            .to_vec();
        objblock.extend_from_slice(&bg_vram[0..0x800]);
        fs::write(
            out_dir.join(format!("level{nn:02}.objblock.bin")),
            &objblock,
        )?;

        render::write_world_render(
            &renders_dir.join(format!("level{nn:02}.png")),
            &decompressed,
            &map_bytes,
            &bg_vram,
            header.palettes[0],
        )?;
        render::write_metatile_sheet(
            &renders_dir.join(format!("level{nn:02}.metatiles.png")),
            &map_bytes,
            &bg_vram,
            header.palettes[0],
        )?;
        let mappings = if object::sprite_map_index(level_id) == 0 {
            &obj_tables.sprite_mappings_a
        } else {
            &obj_tables.sprite_mappings_b
        };
        let used: Vec<bool> = (0..mappings.len())
            .map(|i| records.iter().any(|r| r.sprite_id as usize == i))
            .collect();
        render::write_sprite_sheet(
            &renders_dir.join(format!("level{nn:02}.sprites.png")),
            mappings,
            &used,
            &bg_vram,
            header.palettes[2],
        )?;

        let manifest = LevelManifest {
            schema: MANIFEST_SCHEMA.to_string(),
            level_id,
            header_label: raw.label.clone(),
            header_raw_hex: hex_encode(&raw.bytes),
            header,
            // `$5579` indexes all 32 records without checking for zero/padding.
            // The final record overlaps the instructions beginning at `$3873`.
            secret_entry: Some(secret_entries[i]),
            level_data_label: raw.data_bank_symbol.clone(),
            tileset_map: TilesetMapRef {
                index: map_nn,
                label: raw.tileset_label.clone(),
                file: map_file.clone(),
            },
            graphics: GraphicsRef {
                label: ts.gfx_label.clone(),
                bank: gfx_bank,
                file: gfx_file.clone(),
            },
            enemy_graphics: GraphicsRef {
                label: es.gfx_label.clone(),
                bank: enemy_bank,
                file: enemy_file,
            },
            metatiles,
            metatile_classes: (0..METATILE_COUNT as u8).map(collision::classify).collect(),
            sprite_map_index: object::sprite_map_index(level_id),
            tile_anim: TileAnim::from_table_value(anim_table[level_id as usize]),
            layout: LayoutSummary {
                file: layout_file.clone(),
                width: LAYOUT_WIDTH as u16,
                height: LAYOUT_HEIGHT as u16,
                size: LAYOUT_SIZE as u32,
                distinct_metatile_ids: layout.distinct_metatile_ids(),
                most_common: layout.most_common_metatile(),
            },
            properties: LevelProperties {
                unknown: props_unk[i],
                auto_scroll: props_scroll[i],
                space_physics: props_space[i],
            },
            spawns: ObjectStream {
                table_label: "UnknownData_0xE037".to_string(),
                table_bank: 0x03,
                stream_rom,
                packed_len: packed.len() as u32,
                records,
            },
        };

        let out_path = out_dir.join(format!("level{nn:02}.manifest.json"));
        fs::write(&out_path, serde_json::to_string_pretty(&manifest)? + "\n")?;

        index_levels.push(IndexLevel {
            level_id,
            header_label: raw.label.clone(),
            tileset_map_index: map_nn,
            graphics_file: gfx_file,
            layout_file,
            spawn_count,
        });
        map_used_by[map_nn as usize].push(level_id);

        println!(
            "level {nn:02}: id={level_id} map={map_nn:02} gfx={} spawns={spawn_count} properties={}{}{}",
            ts.gfx_label, props_unk[i], props_scroll[i], props_space[i]
        );
    }

    // ---- index: metatile table usage across the whole ROM ----
    let mut tileset_maps = Vec::new();
    let mut verified_tables = 0;
    for (n, used_by) in map_used_by.iter().enumerate().skip(1) {
        let file = format!("tilemaps/mappings/{n:02}.bin");
        let bytes = fs::read(decomp.join(&file))?;
        let label = format!("Level_TileMap_{n:02}");
        let rom = main_facts
            .label_to_rom
            .get(&label)
            .copied()
            .ok_or_else(|| format!("{label}: no ROM address"))? as usize;
        if baserom[rom..rom + 512] != bytes[..] {
            return Err(format!("{file} differs from baserom.gb at ${rom:05X}").into());
        }
        verified_tables += 1;
        tileset_maps.push(IndexTilesetMap {
            index: n as u8,
            file,
            used_by_levels: used_by.clone(),
            boss_gfx_indices: boss_map_usage
                .iter()
                .filter(|(m, _)| *m as usize == n)
                .map(|(_, g)| *g)
                .collect(),
        });
    }

    let index = LevelsIndex {
        schema: INDEX_SCHEMA.to_string(),
        levels: index_levels,
        tileset_maps,
        notes: vec![
            "tile animation: each level's manifest carries tile_anim {table_value, \
             enabled, row, speed} from UnknownData_0x35EA ($35EA, 32 bytes; the \
             engine stores value+1 in sTileAnimConfig, 0 = off). While enabled, \
             the 4 VRAM tiles at $9200 are replaced by 64 bytes from \
             extracted/tileanim.bin at row*$100 + phase*$40 (phases 0..3) every \
             time sTileAnimTimer reaches speed*2. Phase 0 equals the static \
             tileset art."
                .to_string(),
            "HUD: extracted/hud.json is the in-level status bar — a 20-tile row \
             (template from ROM $07EA, copied to the window map $9C00 by $07DB) \
             drawn by the window layer at rWY=136/rWX=7 (bottom 8px of screen). \
             $076B patches digit tiles ($80 + value) each frame at columns \
             2-3 (lives), 7-9 (coins), 13-14 (money bags), 17-19 (time)."
                .to_string(),
            "OBJ tiles: levelNN.objblock.bin is the $8000-$8FFF object tile \
             space (4 KB) — low half $8000-$87FF = GFX_Mario / GFX_MarioDark / \
             GFX_MarioMoon picked by the level gfx config's low nibble \
             (LoadMarioGFX $0669: 0/1/else), high half $8800-$8FFF = the same \
             bytes as bgblock.bin[0..$800]. OAM mapping tile ids index this \
             block directly ($8000 + t*16)."
                .to_string(),
            "metatile = [top-left, top-right, bottom-left, bottom-right] tile numbers; \
             a tile id < 128 reads VRAM $9000 + t*16, >= 128 reads $8800 + (t-128)*16 \
             (LCDC bit 4 = 0). Proof: ScrollLevelMap ($0AFB) writes the staged tile \
             bytes to dest, dest+1, dest+$20, dest+$21."
                .to_string(),
            "layout = 256x48 metatile ids, row-major, row stride 256 bytes; \
             Read_Level_Data decompresses exactly $3000 bytes to $B000-$DFFF; \
             block pipelines address it as $B000 + row*$100 + col. \
             levelNN.decompressed.bin is relative to this index."
                .to_string(),
            "screen top-left = (camera_x - 80, camera_y - 72); live-verified against \
             PyBoy screenshots at 94-97% exact pixels (remainder = sprites)."
                .to_string(),
            "tileset maps 25-28 are not referenced by level headers; the boss/special \
             GFX path ($25AF, sLevelGfxIndex via the $26B5 pointer table) selects \
             them (see boss_gfx_indices)."
                .to_string(),
            "spawns: packed bank-$03 streams (UnknownData_0xE037, indexed by sCurLevel) \
             expanded by $EBBB into 6-byte records at $AB06: [0/1] world X (hi, 8-aligned \
             lo), [2] mode flag (0 = spawns always, 1 = skipped in Easy Mode; the game \
             writes 2 once spawned), [3/4] world Y (hi, 8-aligned lo), [5] object/anim \
             index. Records are sorted by X; the scan ($8C31) matches them against the \
             camera-driven cursor, and $EBB6 writes $FF $FF after the last record."
                .to_string(),
            "collision: each level's metatile_classes[128] gives every id's \
             engine collision class (solid < $38 blocks+stands; slope $38-$47 \
             stands, never blocks; coin $48/$49; exit $4A/$4B; water $4C-$4F; \
             hazard $50/$51; moon $52-$57; decor $58+). The ranges are global, \
             decoded from the home.asm probes and live-verified (see \
             sml2-core/src/collision.rs)."
                .to_string(),
            "objects: each spawn record carries sprite_id (anim -> sprite table \
             $8B61/$8BC1 by sprite_map_index) and its hitbox [x0,x1,y0,y1]; the \
             full tables (anim maps, extent records, OAM sprite mappings) live in \
             extracted/objects.json, and each level's whole sprite set is drawn to \
             renders/levelNN.sprites.png (16 cells per row, one per sprite id; \
             ids the level never spawns are omitted). objects.json also carries \
             anim_profiles_a/b: per-anim behavior handler ROM offsets (bank $16) \
             plus static scans of each handler's type byte(s) ($FFE2) and its \
             rst-dispatch sub-state code addresses (live-verified: the level-0 \
             walker's handler runs every other frame while active)."
                .to_string(),
            "renders: extracted/renders/levelNN.png is the full 4096x768 world render \
             (16px per metatile, header BGP palette; animated tiles show their static \
             base) and levelNN.metatiles.png the 128 definitions; raw per-file tile \
             sheets live under extracted/gfx/."
                .to_string(),
            "generated by: cargo run -p sml2-extract (from the sml2-rs repo root; \
             reads SML2_DECOMP_ROOT / ../sml2decomp)"
                .to_string(),
        ],
    };
    fs::write(
        out_dir.join("index.json"),
        serde_json::to_string_pretty(&index)? + "\n",
    )?;

    // ---- gfx tile sheets (every .2bpp under gfx/) ----
    let sheets_out = extracted.join("gfx");
    let gfx_root = decomp.join("gfx");
    let mut sheet_stack = vec![gfx_root.clone()];
    let mut sheet_count = 0usize;
    while let Some(dir) = sheet_stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                sheet_stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("2bpp") {
                continue;
            }
            let data = fs::read(&path)?;
            let rel = path
                .strip_prefix(&gfx_root)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let out = sheets_out.join(rel).with_extension("png");
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            render::write_tile_sheet(&out, &data, 16)?;
            sheet_count += 1;
        }
    }
    println!(
        "wrote {sheet_count} gfx tile sheets to {}",
        sheets_out.display()
    );

    println!(
        "wrote {} manifests + index.json to {}",
        headers.len(),
        out_dir.display()
    );
    println!(
        "wrote {} world renders + {} metatile sheets + {} sprite sheets to {}",
        headers.len(),
        headers.len(),
        headers.len(),
        renders_dir.display()
    );
    println!(
        "baserom cross-checks passed: {} headers (20 B each), {} metatile tables (512 B each)",
        headers.len(),
        verified_tables
    );
    Ok(())
}
