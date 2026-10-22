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
