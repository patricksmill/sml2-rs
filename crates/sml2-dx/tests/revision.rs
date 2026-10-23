//! Local-ROM acceptance. No copyrighted fixture is stored in the repository.
use sml2_core::character::Character;
use sml2_core::{object::MappingEntry, video::cgb::CgbPalettes};
use sml2_dx::{extract, identify, Bundle, Presentation, REVISION};

#[test]
#[ignore = "requires SML2_DX_ROM pointing to the user-supplied audited ROM"]
fn audited_opening_tables_and_bundle_round_trip() {
    let path = std::env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(identify(&bytes).unwrap(), REVISION);
    let bundle = extract(&bytes).unwrap();
    let presentation = Presentation::new(&bundle).unwrap();
    let level = presentation.level(0).unwrap();
    let mario = presentation.player(Character::Mario, 0);
    let palettes = CgbPalettes::from_bytes(&level.bg_palettes, &level.obj_palettes).unwrap();
    assert_eq!(palettes.background()[0], [0x7fff, 0x7a89, 0x72e6, 0x0064]);
    assert_eq!(palettes.background()[7], [0x7fff, 0x2a9a, 0x05b4, 0x0064]);
    assert_eq!(palettes.objects()[0], [0x7777, 0x7fff, 0x107c, 0]);
    assert_eq!(palettes.objects()[6], [0x7777, 0x7fff, 0x0137, 0]);
    assert_eq!(level.hud_attributes[5], 0x89);
    assert!(level
        .hud_attributes
        .iter()
        .enumerate()
        .all(|(i, &a)| a == if i == 5 { 0x89 } else { 0x8f }));
    assert_eq!(mario.mappings[0].len(), 11);
    assert_eq!(
        mario.mappings[0][0],
        MappingEntry {
            y: -12,
            x: -7,
            tile: 1,
            attr: 0x20
        }
    );
    assert_eq!(
        mario.mappings[0][3],
        MappingEntry {
            y: -2,
            x: -7,
            tile: 0x11,
            attr: 0x2c
        }
    );
    assert_eq!(
        presentation.objects(level)[0],
        vec![
            MappingEntry {
                y: -16,
                x: -8,
                tile: 0xb0,
                attr: 6
            },
            MappingEntry {
                y: -16,
                x: 0,
                tile: 0xb0,
                attr: 0x26
            },
            MappingEntry {
                y: -8,
                x: -8,
                tile: 0xb4,
                attr: 7
            },
            MappingEntry {
                y: -8,
                x: 0,
                tile: 0xb2,
                attr: 0x27
            },
        ]
    );
    for (source, dest, length) in [
        (0x6f600, 0, 0x300),
        (0x19800, 0x300, 0x380),
        (0x1f100, 0x680, 0x380),
        (0x1c000, 0xa00, 0x600),
    ] {
        assert_eq!(
            &level.background[0][dest..dest + length],
            &bytes[source..source + length]
        );
    }
    // The opening level's VRAM1, attributes, animation and Mario forms.
    assert_eq!(&level.background[1][..0x300], &bytes[0x81d00..0x82000]);
    assert_eq!(&level.background[1][0x300..], &bytes[0x90000..0x90d00]);
    assert_eq!(level.tile_attributes, &bytes[0x84000..0x84100]);
    assert_eq!(
        &presentation.animation_rows[0xd00..0xe00],
        &bytes[0x77380..0x77480]
    );
    assert_eq!(mario.normal, &bytes[0xa8200..0xa8a00]);
    assert_eq!(mario.fire, &bytes[0xa9200..0xa9a00]);
    assert_eq!(mario.bank1, &bytes[0xa8a00..0xa9200]);
    assert_eq!(presentation.hud_tiles, &bytes[0x93c24..0x93c38]);
    let encoded = bundle.encode().unwrap();
    assert_eq!(Bundle::decode(&encoded).unwrap().encode().unwrap(), encoded);
    let mut value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    value["level_config"] = serde_json::json!([0]);
    assert!(Bundle::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut mutation = bytes;
    mutation[0x98001] ^= 1;
    assert!(extract(&mutation).is_err());
}

#[test]
#[ignore = "requires SML2_DX_ROM pointing to the user-supplied audited ROM"]
fn audited_luigi_graphics_and_mapping_literals() {
    let bytes = std::fs::read(std::env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM")).unwrap();
    let bundle = extract(&bytes).unwrap();
    let presentation = Presentation::new(&bundle).unwrap();
    let luigi = presentation.player(Character::Luigi, 0);
    for (graphics, offset, hash) in [
        (
            &luigi.normal,
            0xac200,
            "12ca97a4c770daab7b1bfee26554adb3d002f4021ecc8bc1bb61a2480472ddf7",
        ),
        (
            &luigi.fire,
            0xad200,
            "4acc7c89c4e997143403deba64d55f4e4788fe5e5d72b1503a2e3a36a5e0b517",
        ),
        (
            &luigi.bank1,
            0xaca00,
            "3dc821c8d8e1f92d98f61c2f9923b920eb071d074d9dd9f28cbe3dfac5c9e909",
        ),
    ] {
        assert_eq!(graphics, &bytes[offset..offset + 0x800]);
        assert_eq!(sml2_dx::sha256(graphics), hash);
    }
    assert_eq!(luigi.mappings.len(), 242);
    assert_eq!(luigi.mappings[0].len(), 11);
    assert_eq!(
        luigi.mappings[0][0],
        MappingEntry {
            y: -12,
            x: -7,
            tile: 1,
            attr: 0x22
        }
    );
    assert_eq!(
        luigi.mappings[0][4],
        MappingEntry {
            y: -9,
            x: -3,
            tile: 0x10,
            attr: 0x2a
        }
    );
    assert_eq!(luigi.mappings[32].len(), 10);
    assert_eq!(
        luigi.mappings[32][3],
        MappingEntry {
            y: 3,
            x: -8,
            tile: 0x51,
            attr: 0x2c
        }
    );
    assert_eq!(luigi.mappings[69].len(), 11);
    assert_eq!(luigi.mappings[112].len(), 13);
    let decoded = Presentation::new(&Bundle::decode(&bundle.encode().unwrap()).unwrap()).unwrap();
    assert_eq!(decoded.player(Character::Luigi, 0).mappings, luigi.mappings);
}

#[test]
#[ignore = "requires SML2_DX_ROM pointing to the user-supplied audited ROM"]
fn every_level_derives_from_the_dx_loader_tables() {
    let bytes = std::fs::read(std::env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM")).unwrap();
    let bundle = extract(&bytes).unwrap();
    let encoded = bundle.encode().unwrap();
    assert!(encoded.len() < sml2_dx::cache::MAX_PAYLOAD_BYTES / 4);
    let presentation = Presentation::new(&bundle).unwrap();
    assert_eq!(presentation.levels.len(), 32);
    let rom = |bank: usize, address: usize, length: usize| {
        let start = bank * 0x4000 + address - 0x4000;
        &bytes[start..start + length]
    };
    for (id, level) in presentation.levels.iter().enumerate() {
        // $360A, $362A and $36AA read straight from bank 0 of the DX ROM.
        let config = bytes[0x360a + id];
        let set = usize::from(config >> 4);
        let enemy = &bytes[0x362a + id * 4..0x362a + id * 4 + 4];
        let tiles = &bytes[0x36aa + id * 4..0x36aa + id * 4 + 4];
        let extra = usize::from(u16::from_le_bytes([
            bytes[0x763 + set * 2],
            bytes[0x764 + set * 2],
        ]));
        let word = |b: &[u8]| usize::from(u16::from_le_bytes([b[2], b[3]]));
        let vram0 = &level.background[0];
        assert_eq!(
            &vram0[..0x300],
            rom(0x1b, 0x7000 + set * 0x300, 0x300),
            "level {id}"
        );
        assert_eq!(
            &vram0[0x300..0x680],
            rom(enemy[0].into(), word(enemy), 0x380)
        );
        assert_eq!(&vram0[0x680..0xa00], rom(7, extra, 0x380));
        assert_eq!(&vram0[0xa00..], rom(tiles[0].into(), word(tiles), 0x600));
        let extra1 = match id {
            9 => 0x4d00,
            0xc => 0x5a00,
            0xd => 0x6700,
            _ => 0x4000,
        };
        assert_eq!(&level.background[1][0x300..], rom(0x24, extra1, 0xd00));
        assert_eq!(level.bg_palettes, rom(0x26, 0x4000 + id * 0x40, 0x40));
        assert_eq!(level.obj_palettes, rom(0x26, 0x5000 + id * 0x40, 0x40));
        assert_eq!(level.tile_attributes, rom(0x21, 0x4000 + id * 0x100, 0x100));
        assert_eq!(level.theme, u8::from(config & 0x0f == 5), "level {id}");
        let objects = presentation.objects(level).len();
        assert_eq!(objects, if id >= 0x0a { 228 } else { 168 });
    }
    // Config $35 marks the three alternate-theme (space) levels.
    let themed: Vec<usize> = (0..32)
        .filter(|&i| presentation.levels[i].theme == 1)
        .collect();
    assert_eq!(themed, [0x12, 0x13, 0x1c]);
    for character in [Character::Mario, Character::Luigi] {
        let normal = presentation.player(character, 0);
        let alternate = presentation.player(character, 1);
        assert_ne!(normal.normal, alternate.normal);
        assert_ne!(normal.mappings, alternate.mappings);
        assert_eq!(normal.bank1, alternate.bank1);
    }
}
