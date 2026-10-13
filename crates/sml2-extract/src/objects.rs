//! Extraction of the object taxonomy tables from `baserom.gb`.
//!
//! Every table lives in ROM bank $02 ($8000-$BFFF) or bank $03
//! ($C000-$FFFF); addresses are documented with the `main.asm` references
//! that use them (see `sml2-core::object` for the full chain).

use sml2_core::object::{AnimProfile, Extents, MappingEntry, ObjectTables, OBJECTS_SCHEMA};

/// anim -> sprite id, 96 entries; `main.asm` `$8240`, CPU base `$4B61`
/// (mode A: `sSpriteMapIndex == 0`).
const ANIM_TO_SPRITE_A_ROM: usize = 0x8B61;
/// anim -> sprite id, 112 entries; CPU base `$4BC1` (mode B). Entry 96..111
/// sit directly before the spawn code at $8C31.
const ANIM_TO_SPRITE_B_ROM: usize = 0x8BC1;
/// sprite id -> extent record, 255 `dw` entries; CPU base `$4737`.
const HITBOX_PTR_A_ROM: usize = 0x8737;
/// sprite id -> extent record, 228 `dw` entries; CPU base `$4935`.
const HITBOX_PTR_B_ROM: usize = 0x8935;
/// sprite id -> extent record for the scene-state path (`$9048`), 255 `dw`
/// (ends at $9258; records begin $9259); CPU base `$505B`.
const DRAW_PTR_ROM: usize = 0x905B;
/// sprite id -> OAM mapping pointer, 168 `dw`; bank 3, CPU base `$40B1`.
const MAP_PTR_A_ROM: usize = 0xC0B1;
/// sprite id -> OAM mapping pointer, 228 `dw`; bank 3, CPU base `$4F11`.
const MAP_PTR_B_ROM: usize = 0xCF11;
/// anim -> behavior handler `dw`, 96 entries; bank $16 dispatcher `$58000`
/// (mode A), table begins right after its `rst $28`.
const ANIM_BEHAVIOR_A_ROM: usize = 0x58040;
/// anim -> behavior handler `dw`, 112 entries; bank $16 dispatcher at
/// `$59C04` (mode B).
const ANIM_BEHAVIOR_B_ROM: usize = 0x59C07;
/// Player draw frame -> OAM mapping pointer, 242 `dw`; bank 1, `main.asm`
/// label `UnknownData_0x4000` (region $4000-$41E3; blobs start $41E4).
/// Bank-1 CPU addresses map 1:1 to ROM offsets.
const PLAYER_MAP_PTR_ROM: usize = 0x4000;
const PLAYER_MAP_COUNT: usize = 242;
/// Bank-$16 CPU window ($4000-$7FFF): rom = cpu + ($58000 - $4000).
const BANK16_CPU_ADD: usize = 0x54000;
/// Bank-2 CPU window ($4000-$7FFF) to ROM offset.
const BANK2_CPU_ADD: usize = 0x4000;
/// Bank-3 CPU window to ROM offset.
const BANK3_CPU_ADD: usize = 0x8000;

fn dw(rom: &[u8], off: usize) -> Result<u16, String> {
    let b = rom
        .get(off..off + 2)
        .ok_or_else(|| format!("table read out of bounds at ${off:05X}"))?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

/// Resolve a bank-2 CPU pointer (as stored in the `dw` tables) to ROM.
fn resolve_bank2(v: u16) -> Result<usize, String> {
    if !(0x4000..0x8000).contains(&v) {
        return Err(format!("bank-2 pointer ${v:04X} out of window"));
    }
    Ok(v as usize + BANK2_CPU_ADD)
}

/// Read a 4-byte extent record at a bank-2 ROM offset.
fn extents_at(rom: &[u8], off: usize) -> Result<Extents, String> {
    let b = rom
        .get(off..off + 4)
        .ok_or_else(|| format!("extent record out of bounds at ${off:05X}"))?;
    Ok([b[0] as i8, b[1] as i8, b[2] as i8, b[3] as i8])
}

/// Build a deduplicated extent table: `(records, index)` where
/// `index[sprite]` selects the record.
fn extents_table(
    rom: &[u8],
    ptr_rom: usize,
    count: usize,
    label: &str,
) -> Result<(Vec<Extents>, Vec<u16>), String> {
    let mut records: Vec<Extents> = Vec::new();
    let mut index = Vec::with_capacity(count);
    for i in 0..count {
        let cpu = dw(rom, ptr_rom + 2 * i)?;
        let ext = extents_at(
            rom,
            resolve_bank2(cpu).map_err(|e| format!("{label}[{i}]: {e}"))?,
        )?;
        let idx = match records.iter().position(|r| *r == ext) {
            Some(pos) => pos,
            None => {
                records.push(ext);
                records.len() - 1
            }
        };
        index.push(idx as u16);
    }
    Ok((records, index))
}

/// Parse one OAM mapping blob: `[y, x, tile, attr]` groups until a group
/// starts with `$80` (the loader's terminator) — guarded to 16 entries.
fn parse_mapping(rom: &[u8], start: usize, label: &str) -> Result<Vec<MappingEntry>, String> {
    let mut out = Vec::new();
    let mut p = start;
    loop {
        let first = *rom
            .get(p)
            .ok_or_else(|| format!("{label}: mapping blob runs past the ROM"))?;
        if first == 0x80 {
            return Ok(out);
        }
        if out.len() >= 16 {
            return Err(format!(
                "{label}: mapping blob at ${start:05X} never terminates"
            ));
        }
        let b = rom
            .get(p..p + 4)
            .ok_or_else(|| format!("{label}: mapping entry past the ROM"))?;
        out.push(MappingEntry {
            y: b[0] as i8,
            x: b[1] as i8,
            tile: b[2],
            attr: b[3],
        });
        p += 4;
    }
}

/// sprite id -> OAM mapping via a bank-3 pointer table.
fn mappings_table(
    rom: &[u8],
    ptr_rom: usize,
    count: usize,
    label: &str,
) -> Result<Vec<Vec<MappingEntry>>, String> {
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let cpu = dw(rom, ptr_rom + 2 * i)?;
        if !(0x4000..0x8000).contains(&cpu) {
            return Err(format!(
                "{label}[{i}]: pointer ${cpu:04X} out of bank-3 window"
            ));
        }
        let rom_off = cpu as usize + BANK3_CPU_ADD;
        out.push(parse_mapping(rom, rom_off, &format!("{label}[{i}]"))?);
    }
    Ok(out)
}

/// The player draw's frame -> mapping table (bank 1, ROM $4000; bank-1 CPU
/// addresses are ROM offsets directly).
fn player_mappings_table(rom: &[u8]) -> Result<Vec<Vec<MappingEntry>>, String> {
    let mut out = Vec::with_capacity(PLAYER_MAP_COUNT);
    for i in 0..PLAYER_MAP_COUNT {
        let cpu = dw(rom, PLAYER_MAP_PTR_ROM + 2 * i)?;
        if !(0x4000..0x8000).contains(&cpu) {
            return Err(format!(
                "player mapping[{i}]: pointer ${cpu:04X} out of bank-1 window"
            ));
        }
        out.push(parse_mapping(
            rom,
            cpu as usize,
            &format!("player mapping[{i}]"),
        )?);
    }
    Ok(out)
}

/// anim -> behavior profile (bank $16): the per-anim handler entry plus a
/// static scan of the handler's code region (up to the next entry) for the
/// type byte(s) it sets (`$3E NN $E0 $E2`) and the targets of its inline
/// `rst $28` sub-dispatch tables (the `$EF` inline-table pattern).
fn behavior_profiles(
    rom: &[u8],
    ptr_rom: usize,
    count: usize,
    label: &str,
) -> Result<Vec<AnimProfile>, String> {
    let mut handlers = Vec::with_capacity(count);
    for i in 0..count {
        let cpu = dw(rom, ptr_rom + 2 * i)?;
        if !(0x4000..0x8000).contains(&cpu) {
            return Err(format!(
                "{label}[{i}]: handler ${cpu:04X} out of bank-16 window"
            ));
        }
        handlers.push((cpu as usize + BANK16_CPU_ADD) as u32);
    }
    let mut starts = handlers.clone();
    starts.sort_unstable();
    starts.dedup();
    let mut out = Vec::with_capacity(count);
    for &h in &handlers {
        let end = starts.iter().find(|&&s| s > h).copied().unwrap_or(0x5C000) as usize;
        let mut code = rom[h as usize..end].to_vec();
        let (mut types, mut states) = (Vec::new(), Vec::new());
        let mut i = 0;
        while i < code.len() {
            if code[i] == 0xEF {
                let mut k = i + 1;
                let mut ents = Vec::new();
                while k + 1 < code.len() {
                    let v = u16::from_le_bytes([code[k], code[k + 1]]);
                    if (0x4000..=0x7FFF).contains(&v) {
                        ents.push(v as u32 + BANK16_CPU_ADD as u32);
                        k += 2;
                    } else {
                        break;
                    }
                }
                if !ents.is_empty() {
                    code[i + 1..k].fill(0); // mask the table so it can't fake a type set
                    states.extend(ents);
                    i = k;
                    continue;
                }
            }
            if code[i] == 0x3E && i + 3 < code.len() && code[i + 2] == 0xE0 && code[i + 3] == 0xE2 {
                let t = code[i + 1];
                if !types.contains(&t) {
                    types.push(t);
                }
                i += 4;
                continue;
            }
            i += 1;
        }
        states.sort_unstable();
        states.dedup();
        out.push(AnimProfile {
            handler: h,
            type_sets: types,
            states,
        });
    }
    Ok(out)
}

/// Extract every object table from the ROM.
pub fn extract(rom: &[u8]) -> Result<ObjectTables, String> {
    let anim_to_sprite_a = rom
        .get(ANIM_TO_SPRITE_A_ROM..ANIM_TO_SPRITE_A_ROM + 96)
        .ok_or("anim table A out of bounds")?
        .to_vec();
    let anim_to_sprite_b = rom
        .get(ANIM_TO_SPRITE_B_ROM..ANIM_TO_SPRITE_B_ROM + 112)
        .ok_or("anim table B out of bounds")?
        .to_vec();
    let (hitbox_records_a, hitbox_index_a) = extents_table(rom, HITBOX_PTR_A_ROM, 255, "hitbox A")?;
    let (hitbox_records_b, hitbox_index_b) = extents_table(rom, HITBOX_PTR_B_ROM, 228, "hitbox B")?;
    let (draw_records, draw_index) = extents_table(rom, DRAW_PTR_ROM, 255, "draw")?;
    let sprite_mappings_a = mappings_table(rom, MAP_PTR_A_ROM, 168, "mapping A")?;
    let sprite_mappings_b = mappings_table(rom, MAP_PTR_B_ROM, 228, "mapping B")?;
    let anim_profiles_a = behavior_profiles(rom, ANIM_BEHAVIOR_A_ROM, 96, "behavior A")?;
    let anim_profiles_b = behavior_profiles(rom, ANIM_BEHAVIOR_B_ROM, 112, "behavior B")?;
    let player_mappings = player_mappings_table(rom)?;

    Ok(ObjectTables {
        schema: OBJECTS_SCHEMA.to_string(),
        notes: vec![
            "chain: spawn slot+5 (anim) -> $8240 anim->sprite table ($8B61 mode A / \
             $8BC1 mode B; sSpriteMapIndex = 1 when sCurLevel > $0A) -> staged $FFDB; \
             $81CF copies the sprite's 4-byte extents (tables $8737/$8935) to slot+19..22; \
             $9048 does the same from $905B on the scene-state path; $C000 (bank 3) \
             draws via $C0B1/$CF11 -> gfx/spritemappings*.asm blobs."
                .to_string(),
            "extents are 4 signed bytes, most consistent with [x0, x1, y0, y1] offsets \
             from the object position (sprite 0: -13..-3 x -6..+6)."
                .to_string(),
            "mapping blobs are [Yoff, Xoff, tile, attr] groups terminated by a first \
             byte of $80; offsets are signed (the engine adds them mod 256). The \
             attr byte carries the mapping's own flips ($20 X / $40 Y); the \
             OBP0/OBP1 select and runtime flips come from sSpriteFlipFlags \
             (slot+12 ^ slot+13 ^ slot+14, XORed into every OAM attr by $C000; \
             bit 4 = palette). The live-verified level-0 walker had flags $10 \
             -> OBP1, so sprite sheets render with OBP1."
                .to_string(),
            "anim tables: A = 96 entries (a normal level's max anim is <= 95); \
             B = 112 entries, anims 0..111 (Wario's Castle uses 96..111)."
                .to_string(),
            "behaviors: the bank-$16 update ($58000) reads the anim ($FFD5) and \
             does `rst $28`, an inline-table dispatch (handler: A*2, read dw at \
             the return address, jp there). Table A at $58040 (96 dw), table B \
             at $59C07 (112 dw) after the $59C04 dispatcher; entries are ROM \
             offsets of per-anim behavior code in bank $16. A handler typically \
             sets the type byte ($FFE2) and runs its own `rst $28` sub-state \
             machine (state in $FFDF). $4100 is a bare RET (inert anim). Each \
             anim_profiles_* entry carries that handler plus static scans of \
             its code region: type_sets (the $FFE2 immediates) and states \
             (sub-state code addresses)."
                .to_string(),
        ],
        anim_to_sprite_a,
        anim_to_sprite_b,
        hitbox_records_a,
        hitbox_records_b,
        hitbox_index_a,
        hitbox_index_b,
        draw_records,
        draw_index,
        sprite_mappings_a,
        sprite_mappings_b,
        anim_profiles_a,
        anim_profiles_b,
        player_mappings,
    })
}
