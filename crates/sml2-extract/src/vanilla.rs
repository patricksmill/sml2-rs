use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use sml2_core::collision;
use sml2_core::level::{
    GraphicsRef, HudStrip, IndexLevel, IndexTilesetMap, LayoutSummary, LevelHeader, LevelLayout,
    LevelManifest, LevelProperties, LevelStart, LevelsIndex, MetatileTable, ObjectStream,
    SpawnRecord, TileAnim, TilesetMapRef, INDEX_SCHEMA, LAYOUT_HEIGHT, LAYOUT_SIZE, LAYOUT_WIDTH,
    MANIFEST_SCHEMA, METATILE_COUNT,
};
use sml2_core::object;
use sml2_core::tileset::{TilesetEntry, Tilesets, TILESET_SCHEMA};

pub const MAX_READ: usize = 524_288 + 1;
pub const IMPORTER_VERSION: u32 = 1;
pub const SCHEMA: u32 = 1;

#[derive(Debug)]
pub struct Revision {
    pub id: &'static str,
    pub name: &'static str,
    pub len: usize,
    pub sha256: &'static str,
    pub offsets: &'static str,
}

pub const REVISIONS: &[Revision] = &[Revision {
    id: "ue-v1.0",
    name: "Super Mario Land 2",
    len: 524_288,
    sha256: "5450dce1bd0c073964c374b5b5b5729dce8d00f2e807892c34af32b8bce1392e",
    offsets: "ue-v1.0",
}];

#[derive(Debug)]
pub enum IdentifyError {
    TooLong,
    Unsupported { len: usize, sha256: String },
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn identify(bytes: &[u8]) -> Result<&'static Revision, IdentifyError> {
    if bytes.len() > 524_288 {
        return Err(IdentifyError::TooLong);
    }
    let digest = sha256(bytes);
    REVISIONS
        .iter()
        .find(|revision| revision.len == bytes.len() && revision.sha256 == digest)
        .ok_or(IdentifyError::Unsupported {
            len: bytes.len(),
            sha256: digest,
        })
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub revision: String,
    pub sha256: String,
    pub importer_version: u32,
    pub schema: u32,
}

#[derive(Debug)]
pub enum PublishError {
    Prepare(String),
    Validate(String),
    Io(String),
}

#[derive(Debug)]
pub struct Commit {
    pub backup: Option<PathBuf>,
}

pub fn publish_directory(
    dest: &Path,
    manifest: &Manifest,
    prepare: impl FnOnce(&Path) -> Result<(), PublishError>,
    validate: impl FnOnce(&Path) -> Result<(), PublishError>,
) -> Result<Commit, PublishError> {
    let parent = dest
        .parent()
        .ok_or_else(|| PublishError::Io("vanilla cache has no parent".into()))?;
    fs::create_dir_all(parent).map_err(|e| PublishError::Io(e.to_string()))?;
    let stage = parent.join(format!("vanilla.stage.{}", std::process::id()));
    let _ = fs::remove_dir_all(&stage);
    fs::create_dir_all(&stage).map_err(|e| PublishError::Io(e.to_string()))?;
    if let Err(error) = prepare(&stage) {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|e| PublishError::Io(e.to_string()))?;
    if let Err(error) = fs::write(stage.join("manifest.json"), bytes) {
        let _ = fs::remove_dir_all(&stage);
        return Err(PublishError::Io(error.to_string()));
    }
    if let Err(error) = validate(&stage) {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    let backup = parent.join(format!("vanilla.bak.{}", std::process::id()));
    let _ = fs::remove_dir_all(&backup);
    let had_dest = dest.exists();
    if had_dest {
        fs::rename(dest, &backup).map_err(|e| PublishError::Io(e.to_string()))?;
    }
    if let Err(error) = fs::rename(&stage, dest) {
        if had_dest {
            let _ = fs::rename(&backup, dest);
        }
        let _ = fs::remove_dir_all(&stage);
        return Err(PublishError::Io(error.to_string()));
    }
    Ok(Commit {
        backup: had_dest.then_some(backup),
    })
}

pub fn rollback(dest: &Path, commit: &Commit) -> Result<(), PublishError> {
    let discarded = dest.with_extension("discarded");
    let _ = fs::remove_dir_all(&discarded);
    if dest.exists() {
        fs::rename(dest, &discarded).map_err(|e| PublishError::Io(e.to_string()))?;
    }
    if let Some(backup) = &commit.backup {
        fs::rename(backup, dest).map_err(|e| PublishError::Io(e.to_string()))?;
    }
    let _ = fs::remove_dir_all(&discarded);
    Ok(())
}

pub fn discard_backup(commit: &Commit) {
    if let Some(backup) = &commit.backup {
        let _ = fs::remove_dir_all(backup);
    }
}

const LEVEL_COUNT: usize = 32;
const METATILE_TABLE_COUNT: usize = 28;
const HEADER_ROM: usize = 0x560B;
const HEADER_LEN: usize = 20;
/// `UnknownData_0xE037`, 32 bank-3 spawn-stream pointers.
const SPAWN_TABLE_ROM: usize = 0xE037;
/// Bank 5 `$4125`: `UnknownData_0x14121` plus 4.
const SCRIPT_ROM: usize = 0x14125;
const SCRIPT_LEN: usize = 192;
const TILEANIM_TABLE_ROM: usize = 0x35EA;
const TILEANIM_ROWS_ROM: usize = 0x76680;
const GFX_CONFIG_ROM: usize = 0x360A;
const ENEMY_SETS_ROM: usize = 0x362A;
const TILE_SETS_ROM: usize = 0x36AA;
const PROPS_UNK_ROM: usize = 0x1F51;
const PROPS_SCROLL_ROM: usize = 0x1F71;
const PROPS_SPACE_ROM: usize = 0x1F91;
const SECRET_ROM: usize = 0x377B;
const NODES_ROM: usize = 0x3C218;
const SLOTS_ROM: usize = 0x3E50E;
const WALK_ROM: usize = 0x6163A;
const WALK_ANCHOR: usize = 0x61A40;

/// Compressed `levels/levelNN.bin` blobs, `Level_01` .. `Level_32` in `main.asm`.
const LEVEL_DATA_ROM: [usize; LEVEL_COUNT] = [
    0x24500, 0x28500, 0x2919B, 0x2AA87, 0x2C500, 0x2CCA7, 0x48500, 0x48E29, 0x494D9, 0x49FF2,
    0x5C500, 0x5CE21, 0x5D980, 0x5E4F4, 0x50500, 0x50AB4, 0x51EC9, 0x4C500, 0x4CE19, 0x4DAFF,
    0x54500, 0x54B41, 0x55242, 0x55C4C, 0x4E33A, 0x24E5B, 0x25105, 0x25526, 0x2D8D7, 0x2D40B,
    0x2D192, 0x2E0F5,
];

/// ue-v1.0 graphics labels. The address is the `main.asm` `;$` comment and the
/// path is that label's `INCBIN`, so a level table's bank/pointer can name the
/// same file the disassembly extractor records.
const GFX_CATALOG: &[(usize, &str, &str)] = &[
    (0x18000, "GFX_Mario", "gfx/mario.2bpp"),
    (0x18800, "GFX_MarioDark", "gfx/mariodark.2bpp"),
    (0x19000, "GFX_MarioMoon", "gfx/mariomoon.2bpp"),
    (0x19800, "GFX_EnemiesClassic", "gfx/enemies/classic.2bpp"),
    (
        0x19B80,
        "GFX_EnemiesTreeZone1",
        "gfx/enemies/treezone1.2bpp",
    ),
    (
        0x19F00,
        "GFX_EnemiesTreeZone2",
        "gfx/enemies/treezone2.2bpp",
    ),
    (
        0x1A280,
        "GFX_EnemiesTreeZone3",
        "gfx/enemies/treezone3.2bpp",
    ),
    (
        0x1A600,
        "GFX_EnemiesTreeZone4",
        "gfx/enemies/treezone4.2bpp",
    ),
    (
        0x1A980,
        "GFX_EnemiesTreeZoneBoss",
        "gfx/enemies/treezoneboss.2bpp",
    ),
    (0x1C000, "GFX_TilesetLevel1", "gfx/tilesets/level1.2bpp"),
    (
        0x1C600,
        "GFX_TilesetTreeZone1",
        "gfx/tilesets/treezone1.2bpp",
    ),
    (
        0x1CC00,
        "GFX_TilesetTreeZone2",
        "gfx/tilesets/treezone2.2bpp",
    ),
    (
        0x1D200,
        "GFX_TilesetTreeZone3",
        "gfx/tilesets/treezone3.2bpp",
    ),
    (
        0x1D800,
        "GFX_TilesetTreeZone4",
        "gfx/tilesets/treezone4.2bpp",
    ),
    (
        0x1DE00,
        "GFX_TilesetTreeZone5",
        "gfx/tilesets/treezone5.2bpp",
    ),
    (
        0x1E400,
        "GFX_TilesetTreeZoneBoss",
        "gfx/tilesets/treezoneboss.2bpp",
    ),
    (
        0x1EA00,
        "GFX_TilesetTreeZoneExtra",
        "gfx/tilesets/treezone_extra.2bpp",
    ),
    (
        0x6C000,
        "GFX_TilesetMarioZone1",
        "gfx/tilesets/mariozone1.2bpp",
    ),
    (
        0x6C600,
        "GFX_TilesetMarioZone2",
        "gfx/tilesets/mariozone2.2bpp",
    ),
    (
        0x6CC00,
        "GFX_TilesetMarioZone3",
        "gfx/tilesets/mariozone3.2bpp",
    ),
    (
        0x6D200,
        "GFX_TilesetMarioZone4",
        "gfx/tilesets/mariozone4.2bpp",
    ),
    (
        0x6D800,
        "GFX_TilesetMarioZoneBoss",
        "gfx/tilesets/mariozoneboss.2bpp",
    ),
    (
        0x6DE00,
        "GFX_EnemiesMarioZone1",
        "gfx/enemies/mariozone1.2bpp",
    ),
    (
        0x6E180,
        "GFX_EnemiesMarioZone2",
        "gfx/enemies/mariozone2.2bpp",
    ),
    (
        0x6E500,
        "GFX_EnemiesMarioZone3",
        "gfx/enemies/mariozone3.2bpp",
    ),
    (
        0x6E880,
        "GFX_EnemiesMarioZone4",
        "gfx/enemies/mariozone4.2bpp",
    ),
    (
        0x6EC00,
        "GFX_EnemiesMarioZoneBoss",
        "gfx/enemies/mariozoneboss.2bpp",
    ),
    (
        0x6EF80,
        "GFX_TilesetMarioZoneExtra",
        "gfx/tilesets/mariozone_extra.2bpp",
    ),
    (
        0x70000,
        "GFX_TilesetMacroZone1",
        "gfx/tilesets/macrozone1.2bpp",
    ),
    (
        0x70600,
        "GFX_TilesetMacroZone2",
        "gfx/tilesets/macrozone2.2bpp",
    ),
    (
        0x70C00,
        "GFX_TilesetMacroZone3",
        "gfx/tilesets/macrozone3.2bpp",
    ),
    (
        0x71200,
        "GFX_TilesetMacroZone4",
        "gfx/tilesets/macrozone4.2bpp",
    ),
    (
        0x71800,
        "GFX_TilesetMacroZoneBoss",
        "gfx/tilesets/macrozoneboss.2bpp",
    ),
    (
        0x71E00,
        "GFX_EnemiesMacroZone1",
        "gfx/enemies/macrozone1.2bpp",
    ),
    (
        0x72180,
        "GFX_EnemiesMacroZone2",
        "gfx/enemies/macrozone2.2bpp",
    ),
    (
        0x72500,
        "GFX_EnemiesMacroZone3",
        "gfx/enemies/macrozone3.2bpp",
    ),
    (
        0x72880,
        "GFX_EnemiesMacroZone4",
        "gfx/enemies/macrozone4.2bpp",
    ),
    (
        0x72C00,
        "GFX_EnemiesMacroZoneBoss",
        "gfx/enemies/macrozoneboss.2bpp",
    ),
    (
        0x74000,
        "GFX_TilesetTurtleZone1",
        "gfx/tilesets/turtlezone1.2bpp",
    ),
    (
        0x74600,
        "GFX_TilesetTurtleZone2",
        "gfx/tilesets/turtlezone2.2bpp",
    ),
    (
        0x74C00,
        "GFX_TilesetTurtleZone3",
        "gfx/tilesets/turtlezone3.2bpp",
    ),
    (
        0x75200,
        "GFX_TilesetTurtleZoneBoss",
        "gfx/tilesets/turtlezoneboss.2bpp",
    ),
    (
        0x75800,
        "GFX_EnemiesTurtleZone1",
        "gfx/enemies/turtlezone1.2bpp",
    ),
    (
        0x75B80,
        "GFX_EnemiesTurtleZone2",
        "gfx/enemies/turtlezone2.2bpp",
    ),
    (
        0x75F00,
        "GFX_EnemiesTurtleZone3",
        "gfx/enemies/turtlezone3.2bpp",
    ),
    (
        0x76280,
        "GFX_EnemiesTurtleZoneBoss",
        "gfx/enemies/turtlezoneboss.2bpp",
    ),
    (
        0x78000,
        "GFX_TilesetSpaceZone",
        "gfx/tilesets/spacezone.2bpp",
    ),
    (
        0x78600,
        "GFX_TilesetPumpkinZoneBoss2",
        "gfx/tilesets/pumpkinzoneboss2.2bpp",
    ),
    (
        0x78C00,
        "GFX_TilesetWarioCastle",
        "gfx/tilesets/wariocastle.2bpp",
    ),
    (
        0x79200,
        "GFX_TilesetWarioRoom",
        "gfx/tilesets/warioroom.2bpp",
    ),
    (
        0x79800,
        "GFX_TilesetWarioRoom2",
        "gfx/tilesets/warioroom2.2bpp",
    ),
    (
        0x79A00,
        "GFX_EnemiesHippoLevel",
        "gfx/enemies/hippolevel.2bpp",
    ),
    (
        0x79D80,
        "GFX_EnemiesSpaceZone1",
        "gfx/enemies/spacezone1.2bpp",
    ),
    (
        0x7A100,
        "GFX_EnemiesSpaceZone2",
        "gfx/enemies/spacezone2.2bpp",
    ),
    (
        0x7A480,
        "GFX_EnemiesSpaceZoneBoss",
        "gfx/enemies/spacezoneboss.2bpp",
    ),
    (
        0x7A800,
        "GFX_EnemiesSpaceZoneBoss2",
        "gfx/enemies/spacezoneboss2.2bpp",
    ),
    (
        0x7A880,
        "GFX_EnemiesWarioCastle1",
        "gfx/enemies/wariocastle1.2bpp",
    ),
    (
        0x7AC00,
        "GFX_EnemiesWarioCastle2",
        "gfx/enemies/wariocastle2.2bpp",
    ),
    (0x7AF80, "GFX_EnemiesWario1", "gfx/enemies/wario1.2bpp"),
    (0x7B300, "GFX_EnemiesWario2", "gfx/enemies/wario2.2bpp"),
    (
        0x7C000,
        "GFX_TilesetPumpkinZone1",
        "gfx/tilesets/pumpkinzone1.2bpp",
    ),
    (
        0x7C600,
        "GFX_TilesetPumpkinZone2",
        "gfx/tilesets/pumpkinzone2.2bpp",
    ),
    (
        0x7CC00,
        "GFX_TilesetPumpkinZone3",
        "gfx/tilesets/pumpkinzone3.2bpp",
    ),
    (
        0x7D200,
        "GFX_TilesetPumpkinZone4",
        "gfx/tilesets/pumpkinzone4.2bpp",
    ),
    (
        0x7D800,
        "GFX_TilesetPumpkinZoneBoss",
        "gfx/tilesets/pumpkinzoneboss.2bpp",
    ),
    (
        0x7DE00,
        "GFX_EnemiesPumpkinZone1",
        "gfx/enemies/pumpkinzone1.2bpp",
    ),
    (
        0x7E180,
        "GFX_EnemiesPumpkinZone2",
        "gfx/enemies/pumpkinzone2.2bpp",
    ),
    (
        0x7E500,
        "GFX_EnemiesPumpkinZone3",
        "gfx/enemies/pumpkinzone3.2bpp",
    ),
    (
        0x7E880,
        "GFX_EnemiesPumpkinZone4",
        "gfx/enemies/pumpkinzone4.2bpp",
    ),
    (
        0x7EC00,
        "GFX_EnemiesPumpkinZoneBoss",
        "gfx/enemies/pumpkinzoneboss.2bpp",
    ),
];

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

/// Read one packed spawn stream from the ROM: 3-byte packs, terminated by a
/// `$FF` control byte (returned inclusive).
pub fn read_packed_stream(rom: &[u8], start: usize) -> Result<Vec<u8>, String> {
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
pub fn decode_object_stream(packed: &[u8]) -> Result<Vec<SpawnRecord>, String> {
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

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn rom_slice<'a>(rom: &'a [u8], start: usize, len: usize, what: &str) -> Result<&'a [u8], String> {
    let end = start
        .checked_add(len)
        .ok_or_else(|| format!("{what} truncated at ROM ${start:05X}"))?;
    rom.get(start..end)
        .ok_or_else(|| format!("{what} truncated at ROM ${start:05X}"))
}

fn rom_word(rom: &[u8], off: usize) -> Result<usize, String> {
    let bytes = rom_slice(rom, off, 2, "word")?;
    Ok(bytes[0] as usize | ((bytes[1] as usize) << 8))
}

fn bank_rom(bank: usize, cpu: usize) -> Result<usize, String> {
    if !(0x4000..0x8000).contains(&cpu) {
        return Err(format!("${cpu:04X} is outside $4000-$7FFF"));
    }
    Ok(bank * 0x4000 + (cpu - 0x4000))
}

/// Bank 8 `Level_TileMap_NN` at `$20000 + (NN - 1) * $200`.
fn tilemap_of(cpu: usize) -> Result<(u8, usize), String> {
    let rom = bank_rom(8, cpu)?;
    if rom < 0x20000 || (rom - 0x20000) % 0x200 != 0 {
        return Err(format!("${cpu:04X} is not a Level_TileMap"));
    }
    let index = (rom - 0x20000) / 0x200 + 1;
    if !(1..=METATILE_TABLE_COUNT).contains(&index) {
        return Err(format!("Level_TileMap index {index} is out of range"));
    }
    Ok((index as u8, rom))
}

fn gfx_named(addr: usize) -> Result<(&'static str, &'static str), String> {
    GFX_CATALOG
        .iter()
        .find(|(at, _, _)| *at == addr)
        .map(|(_, label, file)| (*label, *file))
        .ok_or_else(|| format!("no graphics label at ROM ${addr:05X}"))
}

fn gfx_entry(
    rom: &[u8],
    table: usize,
    index: usize,
) -> Result<(u8, String, String, usize), String> {
    let at = table + index * 4;
    let bytes = rom_slice(rom, at, 4, "gfx table")?;
    let bank = bytes[0];
    let cpu = bytes[2] as usize | ((bytes[3] as usize) << 8);
    let addr = bank_rom(bank as usize, cpu)?;
    let (label, file) = gfx_named(addr)?;
    Ok((bank, label.to_string(), file.to_string(), addr))
}

fn write_out(path: &Path, bytes: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_json(
    path: &Path,
    value: &impl serde::Serialize,
    trailing_newline: bool,
) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    if trailing_newline {
        text.push('\n');
    }
    write_out(path, text)
}

/// `castle::emit` also writes the bell-room reference PNGs. The ROM extract
/// keeps the binaries and drops every `.png`.
fn remove_pngs(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("png") {
                fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

pub fn extract(rom: &[u8], dest: &Path) -> Result<&'static Revision, String> {
    let revision = identify(rom).map_err(|error| match error {
        IdentifyError::TooLong => "ROM is longer than the allow-list".into(),
        IdentifyError::Unsupported { len, sha256 } => {
            format!("Unsupported vanilla ROM ({len} bytes, SHA-256 {sha256})")
        }
    })?;
    if revision.offsets != "ue-v1.0" {
        return Err(format!("no offset table for {}", revision.offsets));
    }
    crate::cartridge::emit(rom, dest).map_err(|e| e.to_string())?;
    crate::ending::emit(rom, dest).map_err(|e| e.to_string())?;
    crate::map::emit(rom, dest).map_err(|e| e.to_string())?;
    crate::castle::emit(rom, dest).map_err(|e| e.to_string())?;
    crate::front_end::emit_rom(rom, dest).map_err(|e| e.to_string())?;
    emit_game(rom, dest)?;
    remove_pngs(dest)?;
    Ok(revision)
}

fn emit_game(rom: &[u8], dest: &Path) -> Result<(), String> {
    let secret_bytes = rom_slice(rom, SECRET_ROM, LEVEL_COUNT * 8, "secret entries")?;
    let secret_entries: [LevelStart; LEVEL_COUNT] = std::array::from_fn(|i| {
        let mut row = [0u8; 8];
        row.copy_from_slice(&secret_bytes[i * 8..i * 8 + 8]);
        LevelStart::from_bytes(row)
    });

    let script = rom_slice(rom, SCRIPT_ROM, SCRIPT_LEN, "scene scripts")?;
    write_out(&dest.join("scenes/script_table.bin"), script)?;
    let scripts =
        sml2_core::scene::SceneScripts::from_bytes(script).ok_or("invalid scene script table")?;
    let transfers = sml2_core::scene::SceneTransferAssets::extract(rom, &scripts)?;
    write_json(&dest.join("scenes/transfers.json"), &transfers, false)?;

    let mut tileset_entries = Vec::new();
    let mut boss_map_usage: Vec<(u8, u8)> = Vec::new();
    for idx in 0..7usize {
        let banks = rom_slice(rom, 0x268B + idx * 2, 2, "$268B")?;
        let bg_bank = banks[0] as usize;
        let obj_bank = banks[1] as usize;
        let bg_addr = rom_word(rom, 0x2699 + idx * 4)?;
        let obj_addr = rom_word(rom, 0x2699 + idx * 4 + 2)?;
        let map_addr = rom_word(rom, 0x26B5 + idx * 2)?;
        if !(0x4000..0x8000).contains(&bg_addr)
            || !(0x4000..0x8000).contains(&obj_addr)
            || !(0x4000..0x8000).contains(&map_addr)
        {
            return Err(format!(
                "$25AF index {idx}: a source address is outside $4000-$7FFF"
            ));
        }
        let bg_rom = bank_rom(bg_bank, bg_addr)?;
        let obj_rom = bank_rom(obj_bank, obj_addr)?;
        let (map_index, map_rom) = tilemap_of(map_addr)?;
        boss_map_usage.push((map_index, idx as u8));
        tileset_entries.push(TilesetEntry {
            index: idx as u8,
            bg_bank: bg_bank as u8,
            bg_addr: bg_addr as u16,
            obj_bank: obj_bank as u8,
            obj_addr: obj_addr as u16,
            map_index,
            map_label: format!("Level_TileMap_{map_index:02}"),
            bg_tiles: rom_slice(rom, bg_rom, 0x600, "bg tiles")?.to_vec(),
            obj_tiles: rom_slice(rom, obj_rom, 0x400, "obj tiles")?.to_vec(),
            metatiles: rom_slice(rom, map_rom, 512, "metatiles")?.to_vec(),
        });
    }
    let extra_rom = 7 * 0x4000 + (0x7800 - 0x4000);
    let gfx_tilesets = Tilesets {
        schema: TILESET_SCHEMA.to_string(),
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
        extra_8e80: rom_slice(rom, extra_rom, 0x50, "extra $8E80")?.to_vec(),
        entries: tileset_entries,
    };
    write_json(&dest.join("scenes/tilesets.json"), &gfx_tilesets, false)?;

    let arena_src = 19 * 0x4000 + (0x42F0 - 0x4000) + 3 * 2;
    let arena_ptr = rom_word(rom, arena_src)?;
    if !(0x4000..0x8000).contains(&arena_ptr) {
        return Err(format!(
            "$42F0[3] in bank 19 = ${arena_ptr:04X}, outside $4000-$7FFF"
        ));
    }
    let arena_rom = 19 * 0x4000 + (arena_ptr - 0x4000);
    let arena_layout = decompress_level(
        rom.get(arena_rom..)
            .ok_or_else(|| format!("arena layout truncated at ROM ${arena_rom:05X}"))?,
    )?;
    write_out(&dest.join("scenes/arena_layout.bin"), &arena_layout)?;

    let mut boss_spawns = Vec::new();
    for idx in 0..9usize {
        let rom_bank2 = |addr: usize| 2 * 0x4000 + (addr - 0x4000);
        let t0_addr = rom_word(rom, rom_bank2(0x4E3F + idx * 2))?;
        let t0_off = rom_bank2(t0_addr);
        let triple = rom_slice(rom, t0_off, 3, "boss spawn")?;
        let count = triple[0];
        let hits = triple[1];
        let sprite = triple[2];

        let obj0_addr = rom_word(rom, rom_bank2(0x4E9A + idx * 2))?;
        let obj0_off = rom_bank2(obj0_addr);
        let obj0_def = rom_slice(rom, obj0_off, 18, "boss obj0")?.to_vec();

        let mut subs = Vec::new();
        if count > 0 {
            let obj1_addr = rom_word(rom, rom_bank2(0x4EAC + idx * 2))?;
            let obj1_off = rom_bank2(obj1_addr);
            for c in 0..(count as usize) {
                let start = obj1_off + c * 18;
                subs.push(rom_slice(rom, start, 18, "boss sub-object")?.to_vec());
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
    write_json(&dest.join("scenes/boss_spawns.json"), &boss_spawns, false)?;

    let mut nodes = Vec::with_capacity(2 * 130 + 42);
    nodes.extend_from_slice(rom_slice(rom, NODES_ROM, 130, "map nodes")?);
    nodes.extend_from_slice(rom_slice(rom, SLOTS_ROM, 130, "map slots")?);
    nodes.extend_from_slice(rom_slice(rom, SLOTS_ROM + 131, 42, "map masks")?);
    write_out(&dest.join("map/nodes.bin"), &nodes)?;

    const WALK_NODES: usize = 128;
    if WALK_ANCHOR < WALK_ROM + WALK_NODES * 8 {
        return Err(format!(
            "walk table base ${WALK_ROM:05X} + {WALK_NODES} nodes runs past \
             UnknownData_0x61A40 (${WALK_ANCHOR:05X})"
        ));
    }
    let mut walk_end = WALK_ROM + WALK_NODES * 8;
    for node in 0..WALK_NODES {
        for dir in 0..4 {
            let at = WALK_ROM + node * 8 + dir * 2;
            let ptr = rom_word(rom, at)?;
            if ptr == 0xFFFF {
                continue;
            }
            let mut end = 24 * 0x4000 + (ptr - 0x4000);
            let mut guard = 0;
            loop {
                let byte = *rom.get(end).ok_or_else(|| {
                    format!("walk stream at ${ptr:04X} ran past the end of the ROM")
                })?;
                if byte == 0 {
                    break;
                }
                end += 1;
                guard += 1;
                if guard > 0x400 {
                    return Err(format!("walk stream at ${ptr:04X} has no terminator"));
                }
            }
            walk_end = walk_end.max(end + 2);
        }
    }
    if walk_end > rom.len() {
        return Err(format!(
            "walk stream region ends at ${walk_end:05X}, past the ROM"
        ));
    }
    write_out(&dest.join("map/walk.bin"), &rom[WALK_ROM..walk_end])?;

    const NODE_RECORD_LEN: usize = 8;
    let records_rom = 24 * 0x4000 + (0x5EE6 - 0x4000);
    let records = rom_slice(
        rom,
        records_rom,
        WALK_NODES * NODE_RECORD_LEN,
        "map node records",
    )?;
    if records
        .as_chunks::<NODE_RECORD_LEN>()
        .0
        .iter()
        .any(|row| row[6] != 0 || row[7] != 0)
    {
        return Err(format!(
            "map node record at ${records_rom:05X}: bytes 6/7 are not zero inside \
             the {WALK_NODES}-node table"
        ));
    }
    let past = rom_slice(
        rom,
        records_rom + WALK_NODES * NODE_RECORD_LEN,
        NODE_RECORD_LEN,
        "map node record past the table",
    )?;
    if past[6] == 0 && past[7] == 0 {
        return Err(format!(
            "map node record table: node $80 at ${:05X} still looks like a record",
            records_rom + WALK_NODES * NODE_RECORD_LEN
        ));
    }
    write_out(&dest.join("map/node_records.bin"), records)?;

    let obj_tables = crate::objects::extract(rom)?;
    write_json(&dest.join("objects.json"), &obj_tables, true)?;

    let anim_table = rom_slice(rom, TILEANIM_TABLE_ROM, LEVEL_COUNT, "tile-anim config")?;
    let anim_rows = rom_slice(rom, TILEANIM_ROWS_ROM, 16 * 0x100, "tile-anim rows")?;
    write_out(&dest.join("tileanim.bin"), anim_rows)?;
    let hud = HudStrip {
        template: rom_slice(rom, 0x07EA, 20, "HUD template")?.to_vec(),
        window_x: 7,
        window_y: 136,
        digit_columns: vec![2, 3, 7, 8, 9, 13, 14, 17, 18, 19],
    };
    write_json(&dest.join("hud.json"), &hud, true)?;

    let gfx_config = rom_slice(rom, GFX_CONFIG_ROM, LEVEL_COUNT, "UnknownData_0x360A")?;
    let props_unk = rom_slice(rom, PROPS_UNK_ROM, LEVEL_COUNT, "LevelPropertiesUNK")?;
    let props_scroll = rom_slice(rom, PROPS_SCROLL_ROM, LEVEL_COUNT, "LevelPropertiesScroll")?;
    let props_space = rom_slice(rom, PROPS_SPACE_ROM, LEVEL_COUNT, "LevelPropertiesSpace")?;
    let headers = rom_slice(rom, HEADER_ROM, LEVEL_COUNT * HEADER_LEN, "level headers")?;
    let out_dir = dest.join("levels");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let mut index_levels: Vec<IndexLevel> = Vec::new();
    let mut map_used_by: Vec<Vec<u8>> = vec![Vec::new(); METATILE_TABLE_COUNT + 1];

    for i in 0..LEVEL_COUNT {
        let level_id = i as u8;
        let nn = i + 1;
        let label = format!("Level_Header_{nn:02}");
        let mut raw = [0u8; HEADER_LEN];
        raw.copy_from_slice(&headers[i * HEADER_LEN..i * HEADER_LEN + HEADER_LEN]);
        if raw[10] as usize != i {
            return Err(format!(
                "{label}: level number byte ${:02X} != expected {i}",
                raw[10]
            ));
        }
        let header = LevelHeader::from_bytes(&raw);
        let cpu = raw[11] as usize | ((raw[12] as usize) << 8);
        let (map_nn, map_rom) = tilemap_of(cpu).map_err(|e| format!("{label}: {e}"))?;
        let map_file = format!("tilemaps/mappings/{map_nn:02}.bin");
        let map_bytes = rom_slice(rom, map_rom, 512, &map_file)?;
        let metatiles = MetatileTable::from_bytes(map_bytes)?;
        let level_rom = LEVEL_DATA_ROM[i];
        let decompressed = decompress_level(
            rom.get(level_rom..)
                .ok_or_else(|| format!("{label}: level data truncated at ROM ${level_rom:05X}"))?,
        )
        .map_err(|e| format!("levels/level{nn:02}.bin: {e}"))?;
        let layout_file = format!("level{nn:02}.decompressed.bin");
        write_out(&out_dir.join(&layout_file), &decompressed)?;
        let layout = LevelLayout::from_bytes(&decompressed)?;
        if header.level_data_bank == 19 && header.layout_index == 3 && decompressed != arena_layout
        {
            return Err(format!(
                "{label} claims (bank 19, layout 3) but differs from scenes/arena_layout.bin"
            ));
        }

        let map_bank = header.level_data_bank as usize;
        for (ptr_at, suffix, what) in [
            (0usize, "edge", "sScreenEdgeMap"),
            (2, "scene", "sSceneMap"),
        ] {
            let base = rom_word(rom, map_bank * 0x4000 + ptr_at)?;
            if !(0x4000..0x8000).contains(&base) {
                return Err(format!(
                    "{label}: {what} pointer ${base:04X} (header +{ptr_at}) is outside bank {map_bank}"
                ));
            }
            let off =
                map_bank * 0x4000 + (base - 0x4000) + (header.layout_index & 0x0F) as usize * 48;
            write_out(
                &out_dir.join(format!("level{nn:02}.{suffix}.bin")),
                rom_slice(rom, off, 48, what)?,
            )?;
        }

        let (gfx_bank, gfx_label, gfx_file, gfx_rom) = gfx_entry(rom, TILE_SETS_ROM, i)?;
        let (enemy_bank, enemy_label, enemy_file, enemy_rom) = gfx_entry(rom, ENEMY_SETS_ROM, i)?;
        let stream_cpu = rom_word(rom, SPAWN_TABLE_ROM + i * 2)?;
        if !(0x4000..0x8000).contains(&stream_cpu) {
            return Err(format!(
                "{label}: stream pointer ${stream_cpu:04X} outside the $4000-$7FFF window"
            ));
        }
        let stream_rom = 0x03 * 0x4000 + (stream_cpu - 0x4000);
        let packed = read_packed_stream(rom, stream_rom)?;
        let mut records = decode_object_stream(&packed)?;
        for record in &mut records {
            let sprite = obj_tables
                .sprite_for(level_id, record.anim_index)
                .ok_or_else(|| {
                    format!(
                        "level {level_id}: anim ${:02X} has no anim->sprite entry",
                        record.anim_index
                    )
                })?;
            record.sprite_id = sprite;
            record.hitbox = obj_tables.hitbox_for(level_id, sprite).ok_or_else(|| {
                format!("level {level_id}: sprite ${sprite:02X} has no extent record")
            })?;
        }
        if !records.windows(2).all(|w| w[0].x <= w[1].x) {
            return Err(format!("{label}: spawn stream is not sorted by X"));
        }
        let spawn_count = records.len() as u32;

        let nibble = (gfx_config[i] >> 4) as usize;
        let mut bg_vram = vec![0u8; crate::render::BG_VRAM_LEN];
        bg_vram[0x000..0x300].copy_from_slice(rom_slice(
            rom,
            0x6F000 + nibble * 0x300,
            0x300,
            "$8800 block",
        )?);
        bg_vram[0x300..0x680].copy_from_slice(rom_slice(rom, enemy_rom, 0x380, &enemy_file)?);
        bg_vram[0x680..0xA00].copy_from_slice(rom_slice(
            rom,
            0x1EA00 + nibble * 0x380,
            0x380,
            "$8E80 block",
        )?);
        bg_vram[0xA00..0x1000].copy_from_slice(rom_slice(rom, gfx_rom, 0x600, &gfx_file)?);
        write_out(&out_dir.join(format!("level{nn:02}.bgblock.bin")), &bg_vram)?;

        let mario_rom = match gfx_config[i] & 0x0F {
            0 => 0x18000,
            1 => 0x18800,
            _ => 0x19000,
        };
        let mut objblock = rom_slice(rom, mario_rom, 0x800, "GFX_Mario")?.to_vec();
        objblock.extend_from_slice(&bg_vram[0..0x800]);
        write_out(
            &out_dir.join(format!("level{nn:02}.objblock.bin")),
            &objblock,
        )?;

        let manifest = LevelManifest {
            schema: MANIFEST_SCHEMA.to_string(),
            level_id,
            header_label: label.clone(),
            header_raw_hex: hex_encode(&raw),
            header,
            secret_entry: Some(secret_entries[i]),
            level_data_label: format!("Level_{nn:02}"),
            tileset_map: TilesetMapRef {
                index: map_nn,
                label: format!("Level_TileMap_{map_nn:02}"),
                file: map_file,
            },
            graphics: GraphicsRef {
                label: gfx_label,
                bank: gfx_bank,
                file: gfx_file.clone(),
            },
            enemy_graphics: GraphicsRef {
                label: enemy_label,
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
                stream_rom: stream_rom as u32,
                packed_len: packed.len() as u32,
                records,
            },
        };
        write_json(
            &out_dir.join(format!("level{nn:02}.manifest.json")),
            &manifest,
            true,
        )?;
        index_levels.push(IndexLevel {
            level_id,
            header_label: label,
            tileset_map_index: map_nn,
            graphics_file: gfx_file,
            layout_file,
            spawn_count,
        });
        map_used_by[map_nn as usize].push(level_id);
    }

    let mut tileset_maps = Vec::new();
    for (n, used_by) in map_used_by.iter().enumerate().skip(1) {
        tileset_maps.push(IndexTilesetMap {
            index: n as u8,
            file: format!("tilemaps/mappings/{n:02}.bin"),
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
    write_json(&out_dir.join("index.json"), &index, true)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_short_buffer_and_a_different_digest() {
        let err = identify(&[0; 16]).unwrap_err();
        assert!(matches!(err, IdentifyError::Unsupported { len: 16, .. }));
        let mut rom = vec![0; 524_288];
        let err = identify(&rom).unwrap_err();
        assert!(matches!(
            err,
            IdentifyError::Unsupported { len: 524_288, .. }
        ));
        rom.push(0);
        assert!(matches!(identify(&rom), Err(IdentifyError::TooLong)));
    }

    #[test]
    fn accepts_only_the_ue_v1_0_digest() {
        let mut rom = vec![0; 524_288];
        // The real digest is pinned by the constant. A buffer of zeros must not match it.
        assert!(identify(&rom).is_err());
        let revision = Revision {
            id: "ue-v1.0",
            name: "Super Mario Land 2",
            len: 524_288,
            sha256: "5450dce1bd0c073964c374b5b5b5729dce8d00f2e807892c34af32b8bce1392e",
            offsets: "ue-v1.0",
        };
        assert_eq!(REVISIONS[0].sha256, revision.sha256);
        assert_eq!(REVISIONS[0].name, "Super Mario Land 2");
        assert_eq!(MAX_READ, 524_289);
    }

    #[test]
    fn a_failed_prepare_leaves_the_previous_install() {
        let root = std::env::temp_dir().join(format!("sml2_pub_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dest = root.join("vanilla");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("keep.txt"), b"old").unwrap();
        let manifest = Manifest {
            revision: "ue-v1.0".into(),
            sha256: "ab".into(),
            importer_version: IMPORTER_VERSION,
            schema: SCHEMA,
        };
        let err = publish_directory(
            &dest,
            &manifest,
            |_| Err(PublishError::Prepare("nope".into())),
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(matches!(err, PublishError::Prepare(_)));
        assert_eq!(std::fs::read(dest.join("keep.txt")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_failed_validate_leaves_the_previous_install() {
        let root = std::env::temp_dir().join(format!("sml2_pub_val_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dest = root.join("vanilla");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("keep.txt"), b"old").unwrap();
        let manifest = Manifest {
            revision: "ue-v1.0".into(),
            sha256: "ef".into(),
            importer_version: IMPORTER_VERSION,
            schema: SCHEMA,
        };
        let err = publish_directory(
            &dest,
            &manifest,
            |stage| {
                std::fs::write(stage.join("keep.txt"), b"new").unwrap();
                Ok(())
            },
            |_| Err(PublishError::Validate("bad".into())),
        )
        .unwrap_err();
        assert!(matches!(err, PublishError::Validate(_)));
        assert_eq!(std::fs::read(dest.join("keep.txt")).unwrap(), b"old");
        assert!(!dest.join("manifest.json").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn settings_failure_rolls_the_new_install_back() {
        let root = std::env::temp_dir().join(format!("sml2_roll_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dest = root.join("vanilla");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("keep.txt"), b"old").unwrap();
        let manifest = Manifest {
            revision: "ue-v1.0".into(),
            sha256: "cd".into(),
            importer_version: IMPORTER_VERSION,
            schema: SCHEMA,
        };
        let commit = publish_directory(
            &dest,
            &manifest,
            |stage| {
                std::fs::write(stage.join("keep.txt"), b"new").unwrap();
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(std::fs::read(dest.join("keep.txt")).unwrap(), b"new");
        assert!(dest.join("manifest.json").is_file());
        rollback(&dest, &commit).unwrap();
        assert_eq!(std::fs::read(dest.join("keep.txt")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_random_buffer_is_not_extracted() {
        let dir = std::env::temp_dir().join(format!("sml2_extract_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let err = extract(&[0; 32], &dir).unwrap_err();
        assert!(err.contains("524288") || err.contains("Unsupported"));
        assert!(!dir.exists());
    }

    #[test]
    #[ignore = "requires a local developer baserom.gb; do not commit the ROM"]
    fn rom_only_extract_matches_the_disassembly_extractor() {
        let rom_path =
            std::env::var("SML2_ROM").unwrap_or_else(|_| "../sml2decomp/baserom.gb".into());
        let extracted = std::env::var("SML2_EXTRACTED").unwrap_or_else(|_| "extracted".into());
        let rom = std::fs::read(rom_path).unwrap();
        let out = std::env::temp_dir().join(format!("sml2_parity_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        extract(&rom, &out).unwrap();
        let mut files = Vec::new();
        walk(&out, &out, &mut files);
        assert!(files
            .iter()
            .any(|path| path.ends_with("levels/level01.manifest.json")));
        for relative in files {
            assert!(!relative.ends_with(".png"), "{relative}");
            assert!(!relative.starts_with("renders/"), "{relative}");
            let left = std::fs::read(out.join(&relative)).unwrap();
            let right = std::fs::read(std::path::Path::new(&extracted).join(&relative))
                .unwrap_or_else(|error| panic!("{relative}: {error}"));
            assert_eq!(left, right, "{relative}");
        }
    }

    fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
}
