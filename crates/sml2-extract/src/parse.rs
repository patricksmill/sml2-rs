//! Parsers for the sml2decomp disassembly sources used to build the level
//! manifests: `levels/levelheaders.asm` (headers), `main.asm` (sections,
//! labels, INCBINs, metatile tables), `home.asm` (level property tables) and
//! `levels/tilesets.asm` (per-level tileset graphics table).
//!
//! Address conventions in this repo: a label comment `;$XXXXX` with 5+ hex
//! digits is an absolute ROM offset; a 4-digit `;$XXXX` is a CPU-window
//! address (`$4000-$7FFF`) that needs the label's bank to become a ROM
//! offset: `rom = bank * 0x4000 + (addr - 0x4000)`.

use std::collections::HashMap;

/// Strip a trailing `; comment` and surrounding whitespace.
fn code_part(line: &str) -> &str {
    line.split(';').next().unwrap_or("").trim()
}

/// Parse a `$hex` byte token.
fn hex_byte(tok: &str) -> Result<u8, String> {
    let hex = tok
        .trim()
        .strip_prefix('$')
        .ok_or_else(|| format!("expected $hex byte, got {tok:?}"))?;
    u8::from_str_radix(hex, 16).map_err(|e| format!("bad byte {tok:?}: {e}"))
}

/// Parse a `$hex` word token.
fn hex_word(tok: &str) -> Result<u16, String> {
    let hex = tok
        .trim()
        .strip_prefix('$')
        .ok_or_else(|| format!("expected $hex word, got {tok:?}"))?;
    u16::from_str_radix(hex, 16).map_err(|e| format!("bad word {tok:?}: {e}"))
}

/// Static facts about `main.asm` needed for resolution.
pub struct MainAsmFacts {
    /// Label -> `BANK[$NN]` of the enclosing `SECTION` (main.asm only).
    pub sections: HashMap<String, u8>,
    /// Label -> absolute ROM offset (computed from the `;$addr` comments +
    /// enclosing bank where needed).
    pub label_to_rom: HashMap<String, u32>,
    /// Absolute ROM offset -> first label at that address.
    pub rom_to_label: HashMap<u32, String>,
    /// Label -> INCBIN file path (labels immediately followed by `INCBIN`).
    pub incbin: HashMap<String, String>,
}

/// Scan `main.asm` once, collecting section banks, label addresses and
/// INCBIN associations.
pub fn parse_main_asm(src: &str) -> Result<MainAsmFacts, String> {
    let mut sections: HashMap<String, u8> = HashMap::new();
    let mut label_to_rom: HashMap<String, u32> = HashMap::new();
    let mut rom_to_label: HashMap<u32, String> = HashMap::new();
    let mut incbin: HashMap<String, String> = HashMap::new();

    let mut cur_bank: Option<u8> = None;
    let mut last_label: Option<String> = None;

    for line in src.lines() {
        let code = code_part(line);
        if code.is_empty() {
            continue;
        }

        if let Some(rest) = code.strip_prefix("SECTION ") {
            cur_bank = if let Some(pos) = rest.find("BANK[$") {
                let hex = &rest[pos + 6..pos + 8];
                Some(
                    u8::from_str_radix(hex, 16)
                        .map_err(|e| format!("bad BANK in {rest:?}: {e}"))?,
                )
            } else {
                None // ROM0
            };
            last_label = None;
            continue;
        }

        if let Some(rest) = code.strip_prefix("INCBIN ") {
            if let Some(l) = &last_label {
                incbin
                    .entry(l.clone())
                    .or_insert_with(|| rest.trim().trim_matches('"').to_string());
            }
            last_label = None;
            continue;
        }

        // Top-level label (no leading whitespace, starts with a letter/_).
        let indented = line.starts_with('\t') || line.starts_with(' ');
        if !indented {
            if let Some(label) = code.strip_suffix(':') {
                let label = label.trim();
                if !label.is_empty() && label != "MACRO" {
                    if let Some(b) = cur_bank {
                        sections.entry(label.to_string()).or_insert(b);
                    }
                    // Resolve the label's ROM offset from its ;$addr comment.
                    if let Some(cpos) = line.find(';') {
                        let comment = line[cpos + 1..].trim();
                        if let Some(hexstr) = comment.strip_prefix('$') {
                            let hexdigits: String = hexstr
                                .chars()
                                .take_while(|c| c.is_ascii_hexdigit())
                                .collect();
                            if hexdigits.len() >= 4 {
                                if let Ok(addr) = u32::from_str_radix(&hexdigits, 16) {
                                    let rom = if addr >= 0x10000 {
                                        addr // absolute ROM offset
                                    } else if addr >= 0x4000 {
                                        match cur_bank {
                                            Some(b) => b as u32 * 0x4000 + (addr - 0x4000),
                                            None => addr, // ROM0 window
                                        }
                                    } else {
                                        addr // ROM0 address
                                    };
                                    label_to_rom.entry(label.to_string()).or_insert(rom);
                                    rom_to_label.entry(rom).or_insert_with(|| label.to_string());
                                }
                            }
                        }
                    }
                }
                last_label = Some(label.trim().to_string());
                continue;
            }
        }

        last_label = None;
    }

    Ok(MainAsmFacts {
        sections,
        label_to_rom,
        rom_to_label,
        incbin,
    })
}

/// Resolve `Label: ;$addr` comments in an included file, using the bank of
/// the `INCLUDE` site (e.g. levelheaders.asm is included from bank $01).
pub fn parse_labels_in_file(src: &str, bank: Option<u8>) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    for line in src.lines() {
        let line_trim = line.trim_end();
        if let Some(cpos) = line_trim.find(';') {
            let head = line_trim[..cpos].trim();
            let rest = line_trim[cpos + 1..].trim();
            let label = match head.strip_suffix(':') {
                Some(l) if !l.is_empty() && !l.contains(' ') => l,
                _ => continue,
            };
            let hexstr = match rest.strip_prefix('$') {
                Some(h) => h,
                None => continue,
            };
            let hexdigits: String = hexstr
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            if hexdigits.len() < 4 {
                continue;
            }
            if let Ok(addr) = u32::from_str_radix(&hexdigits, 16) {
                let rom = if addr >= 0x10000 {
                    addr
                } else if addr >= 0x4000 {
                    match bank {
                        Some(b) => b as u32 * 0x4000 + (addr - 0x4000),
                        None => addr,
                    }
                } else {
                    addr
                };
                out.entry(label.to_string()).or_insert(rom);
            }
        }
    }
    out
}

/// Resolve a label's ROM offset: prefer the `;$addr` comment map, then fall
/// back to the `_0xNNNNN` suffix convention used by `Unknown*` labels.
pub fn label_rom(main: &MainAsmFacts, label: &str) -> Option<u32> {
    if let Some(rom) = main.label_to_rom.get(label) {
        return Some(*rom);
    }
    let idx = label.rfind("_0x")?;
    u32::from_str_radix(&label[idx + 3..], 16).ok()
}

/// One parsed level header block.
#[derive(Debug, Clone)]
pub struct RawHeader {
    /// e.g. `Level_Header_01`.
    pub label: String,
    /// The 20 raw bytes, fully resolved.
    pub bytes: [u8; 20],
    /// e.g. `Level_TileMap_01` (the `dw` operand).
    pub tileset_label: String,
    /// Symbol inside `db BANK(...)`, e.g. `Level_01`.
    pub data_bank_symbol: String,
}

/// Parsed header block before conversion to [`RawHeader`]:
/// `(label, bytes, tileset label, data-bank symbol)`.
type HeaderBuilder = (String, Vec<u8>, Option<String>, Option<String>);

/// Parse all `Level_Header_NN` blocks from `levels/levelheaders.asm`.
///
/// `main` supplies section banks (for `BANK(...)`) and label addresses (for
/// the `dw Level_TileMap_NN` pointer value).
pub fn parse_level_headers(
    src: &str,
    main: &MainAsmFacts,
    header_label_rom: &HashMap<String, u32>,
) -> Result<Vec<RawHeader>, String> {
    let mut out: Vec<RawHeader> = Vec::new();
    let mut cur: Option<HeaderBuilder> = None;

    let is_header_label = |l: &str| -> bool {
        l.starts_with("Level_Header_")
            && l["Level_Header_".len()..]
                .chars()
                .all(|c| c.is_ascii_digit())
    };

    for line in src.lines() {
        let code = code_part(line);
        if code.is_empty() {
            continue;
        }
        if let Some(label) = code.strip_suffix(':') {
            if is_header_label(label) {
                if let Some(prev) = cur.take() {
                    out.push(finish_header(prev)?);
                }
                cur = Some((label.to_string(), Vec::new(), None, None));
            }
            continue;
        }
        let Some((_, bytes, tileset, bank_sym)) = cur.as_mut() else {
            continue;
        };
        if let Some(rest) = code.strip_prefix("db ") {
            for tok in rest.split(',') {
                let tok = tok.trim();
                if let Some(sym) = tok.strip_prefix("BANK(").and_then(|s| s.strip_suffix(')')) {
                    let b = main
                        .sections
                        .get(sym)
                        .ok_or_else(|| format!("BANK({sym}): no section found"))?;
                    *bank_sym = Some(sym.to_string());
                    bytes.push(*b);
                } else {
                    bytes.push(hex_byte(tok)?);
                }
            }
        } else if let Some(rest) = code.strip_prefix("dw ") {
            let sym = rest.trim().to_string();
            let rom = main
                .label_to_rom
                .get(&sym)
                .ok_or_else(|| format!("dw {sym}: label address unknown"))?;
            let bank = main
                .sections
                .get(&sym)
                .ok_or_else(|| format!("dw {sym}: no section found"))?;
            if *bank == 0 {
                return Err(format!("dw {sym}: expected a banked label"));
            }
            let cpu = 0x4000u32 + (rom - *bank as u32 * 0x4000);
            *tileset = Some(sym);
            bytes.push(cpu as u8);
            bytes.push((cpu >> 8) as u8);
        } else {
            return Err(format!("unexpected directive in header block: {code:?}"));
        }
    }
    if let Some(prev) = cur.take() {
        out.push(finish_header(prev)?);
    }
    if out.len() != 32 {
        return Err(format!("expected 32 level headers, parsed {}", out.len()));
    }
    for h in &out {
        if !header_label_rom.contains_key(&h.label) {
            return Err(format!("{}: no ROM address comment found", h.label));
        }
    }
    Ok(out)
}

fn finish_header((label, bytes, tileset, bank_sym): HeaderBuilder) -> Result<RawHeader, String> {
    let bytes: [u8; 20] = bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("{label}: expected 20 bytes, got {}", v.len()))?;
    let tileset_label = tileset.ok_or_else(|| format!("{label}: missing tileset pointer"))?;
    let data_bank_symbol = bank_sym.ok_or_else(|| format!("{label}: missing BANK(...)"))?;
    Ok(RawHeader {
        label,
        bytes,
        tileset_label,
        data_bank_symbol,
    })
}

/// One entry of a per-level 4-byte GFX pointer table (bank, $00, dw ptr) —
/// used for both `LevelTileSets` (levels/tilesets.asm) and `LevelEnemySets`
/// (levels/enemysets.asm); both have 32 entries indexed by `sCurLevel * 4`.
#[derive(Debug, Clone)]
pub struct GfxTableEntry {
    /// e.g. `GFX_TilesetTreeZone1` / `GFX_EnemiesClassic`.
    pub gfx_label: String,
}

/// Parse a 32-entry per-level GFX pointer table introduced by `label`.
pub fn parse_gfx_pointer_table(src: &str, label: &str) -> Result<Vec<GfxTableEntry>, String> {
    let mut out = Vec::new();
    let mut in_table = false;
    let mut pending_bank_sym: Option<String> = None;
    let mut expect = 0u8; // 0 = bank, 1 = unused, 2 = ptr

    for line in src.lines() {
        let code = code_part(line);
        if code.is_empty() {
            continue;
        }
        if let Some(l) = code.strip_suffix(':') {
            if l == label {
                in_table = true;
            }
            continue;
        }
        if !in_table {
            continue;
        }
        if let Some(rest) = code.strip_prefix("db ") {
            match expect {
                0 => {
                    let sym = rest
                        .trim()
                        .strip_prefix("BANK(")
                        .and_then(|s| s.strip_suffix(')'))
                        .ok_or_else(|| format!("{label}: expected BANK(...), got {rest:?}"))?;
                    pending_bank_sym = Some(sym.to_string());
                    expect = 1;
                }
                1 => {
                    hex_byte(rest.trim())?; // the $00 "unused" byte
                    expect = 2;
                }
                _ => return Err(format!("{label}: unexpected db {rest:?}")),
            }
        } else if let Some(rest) = code.strip_prefix("dw ") {
            if expect != 2 {
                return Err(format!("{label}: unexpected dw {rest:?}"));
            }
            let sym = rest.trim().to_string();
            if Some(&sym) != pending_bank_sym.as_ref() {
                return Err(format!(
                    "{label}: dw {sym} does not match BANK symbol {:?}",
                    pending_bank_sym
                ));
            }
            out.push(GfxTableEntry { gfx_label: sym });
            pending_bank_sym = None;
            expect = 0;
        } else {
            return Err(format!("{label}: unexpected directive {code:?}"));
        }
    }
    if out.len() != 32 {
        return Err(format!(
            "{label}: expected 32 entries, parsed {}",
            out.len()
        ));
    }
    Ok(out)
}

/// Collect `db $xx, ...` values of a byte table introduced by `label`.
pub fn parse_byte_table(src: &str, label: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut in_table = false;
    for line in src.lines() {
        let code = code_part(line);
        if code.is_empty() {
            continue;
        }
        if code == format!("{label}:") {
            in_table = true;
            continue;
        }
        if !in_table {
            continue;
        }
        if let Some(rest) = code.strip_prefix("db ") {
            for tok in rest.split(',') {
                out.push(hex_byte(tok.trim())?);
            }
        } else {
            break; // next label / statement ends the table
        }
    }
    if out.is_empty() {
        return Err(format!("table {label}: not found"));
    }
    Ok(out)
}

/// Collect `dw $xxxx` values of a word table introduced by `label`.
pub fn parse_word_table(src: &str, label: &str) -> Result<Vec<u16>, String> {
    let mut out = Vec::new();
    let mut in_table = false;
    for line in src.lines() {
        let code = code_part(line);
        if code.is_empty() {
            continue;
        }
        if code == format!("{label}:") {
            in_table = true;
            continue;
        }
        if !in_table {
            continue;
        }
        if let Some(rest) = code.strip_prefix("dw ") {
            for tok in rest.split(',') {
                out.push(hex_word(tok.trim())?);
            }
        } else {
            break;
        }
    }
    if out.is_empty() {
        return Err(format!("table {label}: not found"));
    }
    Ok(out)
}
