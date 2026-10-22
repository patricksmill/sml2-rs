//! Local-ROM acceptance. No copyrighted fixture is stored in the repository.
use sml2_core::{object::MappingEntry, video::cgb::CgbPalettes};
use sml2_dx::{extract, identify, Bundle, REVISION};

#[test]
#[ignore = "requires SML2_DX_ROM pointing to the user-supplied audited ROM"]
fn audited_opening_tables_and_bundle_round_trip() {
    let path = std::env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(identify(&bytes).unwrap(), REVISION);
    let bundle = extract(&bytes).unwrap();
    let palettes = CgbPalettes::from_bytes(&bundle.bg_palettes, &bundle.obj_palettes).unwrap();
    assert_eq!(palettes.background()[0], [0x7fff, 0x7a89, 0x72e6, 0x0064]);
    assert_eq!(palettes.background()[7], [0x7fff, 0x2a9a, 0x05b4, 0x0064]);
    assert_eq!(palettes.objects()[0], [0x7777, 0x7fff, 0x107c, 0]);
    assert_eq!(palettes.objects()[6], [0x7777, 0x7fff, 0x0137, 0]);
    assert_eq!(bundle.hud_attributes[5], 0x89);
    assert!(bundle
        .hud_attributes
        .iter()
        .enumerate()
        .all(|(i, &a)| a == if i == 5 { 0x89 } else { 0x8f }));
    assert_eq!(bundle.player_mappings[0].len(), 11);
    assert_eq!(
        bundle.player_mappings[0][0],
        MappingEntry {
            y: -12,
            x: -7,
            tile: 1,
            attr: 0x20
        }
    );
    assert_eq!(
        bundle.player_mappings[0][3],
        MappingEntry {
            y: -2,
            x: -7,
            tile: 0x11,
            attr: 0x2c
        }
    );
    assert_eq!(
        bundle.object_mappings[0],
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
            &bundle.background[0][dest..dest + length],
            &bytes[source..source + length]
        );
    }
    let encoded = bundle.encode().unwrap();
    assert_eq!(Bundle::decode(&encoded).unwrap().encode().unwrap(), encoded);
    let mut value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    value["bg_palettes"] = serde_json::json!([0]);
    assert!(Bundle::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut mutation = bytes;
    mutation[0x98001] ^= 1;
    assert!(extract(&mutation).is_err());
}

#[test]
#[ignore = "requires SML2_DX_ROM pointing to the user-supplied audited ROM"]
fn audited_luigi_graphics_and_mapping_literals() {
    use sml2_core::character::Character;
    let bytes = std::fs::read(std::env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM")).unwrap();
    let bundle = extract(&bytes).unwrap();
    assert!(bundle.supports_character(Character::Luigi));
    let luigi = bundle.luigi.as_ref().unwrap();
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
    let encoded = bundle.encode().unwrap();
    assert_eq!(
        Bundle::decode(&encoded).unwrap().luigi.unwrap().mappings,
        luigi.mappings
    );
}
