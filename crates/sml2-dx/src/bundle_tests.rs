//! Synthetic bundle tests: real ROM bytes are not needed to validate the
//! persisted format, reject malformed cache content, or pin how each level's
//! presentation is derived.
use super::*;
use sml2_core::{character::Character, object::MappingEntry};

/// Every mapping table points at one `$80` terminator.
const EMPTY: u16 = 0x7FF0;

fn put(bundle: &mut Bundle, bank: u8, address: u16, bytes: &[u8]) {
    let start = usize::from(address) - 0x4000;
    bundle.banks.get_mut(&bank).unwrap()[start..start + bytes.len()].copy_from_slice(bytes);
}

fn table(bundle: &mut Bundle, bank: u8, at: u16, count: u16, target: u16) {
    for i in 0..count {
        put(bundle, bank, at + i * 2, &target.to_le_bytes());
    }
}

fn fixture() -> Bundle {
    let mut bundle = Bundle {
        schema: BUNDLE_SCHEMA,
        source_sha256: ROM_SHA256.into(),
        level_config: vec![0; LEVELS],
        enemy_sets: [0x06, 0, 0x00, 0x58].repeat(LEVELS),
        tile_sets: [0x07, 0, 0x00, 0x40].repeat(LEVELS),
        extra_pointers: vec![0x00, 0x6A, 0x80, 0x6D, 0x00, 0x71, 0x80, 0x74],
        // Metatiles at `$08:$4000`, layout 0 of bank `$09`.
        level_headers: [&[0; 11][..], &[0x00, 0x40, 0x09], &[0; 4], &[0x00, 0]]
            .concat()
            .repeat(LEVELS),
        banks: BANKS.iter().map(|&b| (b, vec![0; BANK_LEN])).collect(),
    };
    // `$42F0[0]` -> `$5000`: runs of metatile 0, 257 bytes each.
    put(&mut bundle, 0x09, 0x42F0, &0x5000u16.to_le_bytes());
    put(&mut bundle, 0x09, 0x5000, &[0x80, 0x00].repeat(48));
    for bank in [0x23, 0x28, 0x3A, 0x2C, 0x2D] {
        put(&mut bundle, bank, EMPTY, &[0x80]);
    }
    for bank in [0x23, 0x28, 0x3A] {
        table(&mut bundle, bank, 0x40B1, 168, EMPTY);
        table(&mut bundle, bank, 0x4F11, 228, EMPTY);
    }
    for bank in [0x2C, 0x2D] {
        table(&mut bundle, bank, 0x4000, 242, EMPTY);
        table(&mut bundle, bank, 0x6000, 242, EMPTY);
    }
    table(&mut bundle, 0x24, 0x6000, LEVELS as u16, 0x6040);
    // The overworld's pointer tables, all at one empty record: a `$FF` first
    // byte ends a route patch, a `$FF` tile ends a decor metasprite. The
    // route-patch table `$63EA` is the zone decor table's last 58 entries.
    for bank in [0x0F, 0x18] {
        put(&mut bundle, bank, 0x7000, &[0xFF, 0, 0xFF, 0]);
    }
    table(&mut bundle, 0x18, 0x62E6, 188, 0x7000);
    table(&mut bundle, 0x0F, 0x4AD7, 16, 0x7000);
    table(&mut bundle, 0x0F, 0x4C4D, 17, 0x7000);
    bundle
}

fn decode(value: &serde_json::Value) -> Result<Bundle, ImportError> {
    Bundle::decode(&serde_json::to_vec(value).unwrap())
}

#[test]
fn valid_bundle_round_trips_through_hex_banks() {
    let mut bundle = fixture();
    put(&mut bundle, 0x1B, 0x7000, &[0x00, 0x7F, 0x80, 0xFF]);
    let bytes = bundle.encode().unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains(r#""1B":""#) && text.contains("007f80ff"));
    let decoded = Bundle::decode(&bytes).unwrap();
    assert_eq!(decoded.banks, bundle.banks);
    assert_eq!(decoded.encode().unwrap(), bytes);
}

#[test]
fn old_schemas_other_sources_and_unknown_fields_are_rejected() {
    let value = serde_json::to_value(fixture()).unwrap();
    for schema in [1, 2, 3, 4, 5, BUNDLE_SCHEMA + 1] {
        let mut bad = value.clone();
        bad["schema"] = schema.into();
        let error = decode(&bad).unwrap_err().to_string();
        assert!(error.contains("re-import"), "{error}");
    }
    for source in ["", &"0".repeat(64), &ROM_SHA256.to_uppercase()] {
        let mut bad = value.clone();
        bad["source_sha256"] = source.into();
        assert!(decode(&bad).is_err());
    }
    let mut bad = value.clone();
    bad["extra"] = 1.into();
    assert!(decode(&bad).is_err());
    for field in [
        "level_config",
        "enemy_sets",
        "tile_sets",
        "extra_pointers",
        "level_headers",
        "banks",
    ] {
        let mut bad = value.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(decode(&bad).is_err(), "{field}");
    }
}

#[test]
fn tables_and_banks_must_have_their_exact_shape() {
    for (field, length) in [
        ("level_headers", LEVELS * 20),
        ("level_config", LEVELS),
        ("enemy_sets", LEVELS * 4),
        ("tile_sets", LEVELS * 4),
        ("extra_pointers", 8),
    ] {
        for wrong in [length - 1, length + 1] {
            let mut value = serde_json::to_value(fixture()).unwrap();
            value[field] = vec![0u8; wrong].into();
            assert!(decode(&value).is_err(), "{field} {wrong}");
        }
    }
    let mut missing = fixture();
    missing.banks.remove(&0x26);
    assert!(missing.validate().is_err());
    let mut extra = fixture();
    extra.banks.insert(0x02, vec![0; BANK_LEN]);
    assert!(extra.validate().is_err());
    let mut short = fixture();
    short.banks.get_mut(&0x2A).unwrap().pop();
    assert!(short.validate().is_err());
    let value = serde_json::to_value(fixture()).unwrap();
    for (bank, hex) in [("1b", "00"), ("1B", "0"), ("1B", "zz"), ("100", "00")] {
        let mut bad = value.clone();
        bad["banks"][bank] = hex.into();
        assert!(decode(&bad).is_err(), "{bank} {hex}");
    }
    assert!(Bundle::decode(b"{").is_err());
    assert!(Bundle::decode(&vec![b' '; crate::cache::MAX_PAYLOAD_BYTES + 1]).is_err());
}

#[test]
fn malformed_level_and_mapping_tables_are_rejected() {
    let mut bundle = fixture();
    bundle.level_config[3] = 0x40;
    assert!(bundle.validate().is_err(), "graphics set past $0763");
    let mut bundle = fixture();
    bundle.tile_sets[4 * 5 + 3] = 0x7D; // $7D00 + $600 crosses the bank.
    assert!(bundle.validate().is_err());
    let mut bundle = fixture();
    put(&mut bundle, 0x3A, 0x4F11, &0x3FFFu16.to_le_bytes());
    assert!(bundle.validate().is_err(), "pointer below the bank");
    let mut bundle = fixture();
    put(&mut bundle, 0x2D, 0x6000, &0x7000u16.to_le_bytes());
    assert!(bundle.validate().is_err(), "65 cells without $80");
    let mut bundle = fixture();
    put(&mut bundle, 0x09, 0x42F0, &0x7FF0u16.to_le_bytes());
    assert!(bundle.validate().is_err(), "a layout running off its bank");
}

#[test]
fn each_level_uses_its_own_tables_and_the_dx_loader_sources() {
    let mut bundle = fixture();
    // Level 9 (config $31): set 3, Mario's normal theme, its own VRAM1 block.
    bundle.level_config[9] = 0x31;
    bundle.enemy_sets[9 * 4..9 * 4 + 4].copy_from_slice(&[0x1F, 0, 0x80, 0x61]);
    bundle.tile_sets[9 * 4..9 * 4 + 4].copy_from_slice(&[0x1D, 0, 0x00, 0x46]);
    put(&mut bundle, 0x07, 0x7480, &[0xE1]);
    put(&mut bundle, 0x1B, 0x7000 + 3 * 0x300, &[0xE2]);
    put(&mut bundle, 0x1F, 0x6180, &[0xE3]);
    put(&mut bundle, 0x1D, 0x4600, &[0xE4]);
    put(&mut bundle, 0x20, 0x5D00, &[0xE5]);
    put(&mut bundle, 0x24, 0x4D00, &[0xE6]);
    put(&mut bundle, 0x21, 0x4000 + 9 * 0x100, &[0xE7]);
    put(&mut bundle, 0x26, 0x4000 + 9 * 0x40, &[0x1F, 0]);
    put(&mut bundle, 0x26, 0x5000 + 9 * 0x40, &[0xE0, 3]);
    put(&mut bundle, 0x24, 0x6000 + 9 * 2, &0x6100u16.to_le_bytes());
    put(&mut bundle, 0x24, 0x6100, &[0x8F]);
    // Level $12 (config $35) selects the alternate theme.
    bundle.level_config[0x12] = 0x35;
    // Level $0D draws objects from bank $3A, mode B.
    put(&mut bundle, 0x3A, 0x7000, &[0xF8, 4, 12, 0x2B, 0x80]);
    put(&mut bundle, 0x3A, 0x4F11, &0x7000u16.to_le_bytes());
    // Level 9's own metatile table and layout, literal and run bytes.
    bundle.level_headers[9 * 20 + 0x0B..9 * 20 + 0x0E].copy_from_slice(&[0x00, 0x42, 0x0A]);
    bundle.level_headers[9 * 20 + 0x12] = 0x31;
    put(&mut bundle, 0x08, 0x4200, &[1, 2, 3, 4]);
    put(&mut bundle, 0x0A, 0x42F2, &0x6000u16.to_le_bytes());
    put(&mut bundle, 0x0A, 0x6000, &[0x05, 0x86, 0x02]);
    put(&mut bundle, 0x0A, 0x6003, &[0x80, 0x00].repeat(48));
    // Luigi's alternate-theme fire graphics.
    put(&mut bundle, 0x2B, 0x6200, &[0xE8]);

    let p = Presentation::new(&bundle).unwrap();
    let level = p.level(9).unwrap();
    let [vram0, vram1] = &level.background;
    assert_eq!(
        [vram0[0x680], vram0[0], vram0[0x300], vram0[0xA00]],
        [0xE1, 0xE2, 0xE3, 0xE4]
    );
    assert_eq!([vram1[0], vram1[0x300]], [0xE5, 0xE6]);
    assert_eq!(level.tile_attributes[0], 0xE7);
    assert_eq!(&level.bg_palettes[..2], &[0x1F, 0]);
    assert_eq!(&level.obj_palettes[..2], &[0xE0, 3]);
    assert_eq!(level.hud_attributes[0], 0x8F);
    assert_eq!(level.theme, 0);
    assert_eq!(level.metatiles.get(0).unwrap().tiles, [1, 2, 3, 4]);
    assert_eq!(
        &level.layout[..5],
        &[5, 6, 6, 6, 0],
        "a run is the byte plus its count"
    );
    assert_eq!(level.layout.len(), 0x3000);
    assert_eq!(
        p.level(0).unwrap().background[1][0x300],
        0,
        "other levels use $4000"
    );
    assert_eq!(p.level(0x12).unwrap().theme, 1);
    let level_d = p.level(0x0D).unwrap();
    assert_eq!(
        p.objects(level_d)[0],
        vec![MappingEntry {
            y: -8,
            x: 4,
            tile: 12,
            attr: 0x2B
        }]
    );
    assert_eq!(p.objects(level_d).len(), 228);
    assert_eq!(p.objects(p.level(0).unwrap()).len(), 168);
    assert!(p.objects(p.level(0x0E).unwrap()).iter().all(Vec::is_empty));
    assert_eq!(p.player(Character::Luigi, 1).fire[0], 0xE8);
    assert_eq!(p.player(Character::Mario, 1).fire[0], 0);
    assert!(p.level(32).is_none());
    assert_eq!(p.rom(0x1D, 0x4600, 1), Some(&[0xE4][..]));
    assert_eq!(p.rom(0x01, 0x4000, 1), None, "unstored bank");
    assert_eq!(p.map.screens.len(), 15, "hub, zones and both minigames");
    assert_eq!(
        p.map.screens[9].attributes.as_ref().map(Vec::len),
        Some(0x240)
    );
}
