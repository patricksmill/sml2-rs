//! Synthetic bundle shape tests: real ROM bytes are not needed to validate the
//! persisted format or prove malformed local cache content is rejected.
use super::*;

fn fixture() -> Bundle {
    Bundle {
        schema: BUNDLE_SCHEMA,
        source_sha256: ROM_SHA256.into(),
        background: [vec![0; 0x1000], vec![0; 0x1000]],
        mario_normal: vec![0; 0x800],
        mario_fire: vec![0; 0x800],
        mario_bank1: vec![0; 0x800],
        tile_attributes: vec![0; 256],
        hud_tiles: vec![0; 20],
        hud_attributes: vec![0; 20],
        bg_palettes: vec![0; 64],
        obj_palettes: vec![0; 64],
        animation: vec![0; 256],
        object_mappings: vec![vec![]; 168],
        player_mappings: vec![vec![]; 242],
        luigi: Some(CharacterGraphics {
            normal: vec![0; 0x800],
            fire: vec![0; 0x800],
            bank1: vec![0; 0x800],
            mappings: vec![vec![]; 242],
        }),
    }
}

fn decode(value: &serde_json::Value) -> Result<Bundle, ImportError> {
    Bundle::decode(&serde_json::to_vec(value).unwrap())
}

#[test]
fn valid_bundle_round_trip_preserves_literal_signed_mapping_bytes() {
    let mut bundle = fixture();
    bundle.player_mappings[0].push(MappingEntry {
        y: -128,
        x: 127,
        tile: 0xff,
        attr: 0xef,
    });
    let bytes = bundle.encode().unwrap();
    let decoded = Bundle::decode(&bytes).unwrap();
    assert_eq!(decoded.player_mappings[0][0], bundle.player_mappings[0][0]);
    assert_eq!(decoded.encode().unwrap(), bytes);
}

#[test]
fn schema_revision_unknown_and_missing_fields_are_rejected() {
    let value = serde_json::to_value(fixture()).unwrap();
    for schema in [0, BUNDLE_SCHEMA + 1] {
        let mut bad = value.clone();
        bad["schema"] = schema.into();
        assert!(decode(&bad).is_err());
    }
    for source in ["", &"0".repeat(64), &ROM_SHA256.to_uppercase()] {
        let mut bad = value.clone();
        bad["source_sha256"] = source.into();
        assert!(decode(&bad).is_err());
    }
    let mut bad = value.clone();
    bad["unrecognized"] = true.into();
    assert!(decode(&bad).is_err());
    let mut bad = value;
    bad.as_object_mut().unwrap().remove("schema");
    assert!(decode(&bad).is_err());
}

#[test]
fn every_fixed_byte_table_rejects_short_and_long_data() {
    let value = serde_json::to_value(fixture()).unwrap();
    for field in [
        "mario_normal",
        "mario_fire",
        "mario_bank1",
        "tile_attributes",
        "hud_tiles",
        "hud_attributes",
        "bg_palettes",
        "obj_palettes",
        "animation",
    ] {
        let expected = value[field].as_array().unwrap().len();
        for length in [expected - 1, expected + 1] {
            let mut bad = value.clone();
            bad[field] = serde_json::json!(vec![0u8; length]);
            assert!(decode(&bad).is_err(), "accepted {field} length {length}");
        }
    }
    for bank in 0..2 {
        for length in [0xfff, 0x1001] {
            let mut bad = value.clone();
            bad["background"][bank] = serde_json::json!(vec![0u8; length]);
            assert!(decode(&bad).is_err());
        }
    }
    let mut bad = value;
    bad["background"] = serde_json::json!([vec![0u8; 0x1000]]);
    assert!(decode(&bad).is_err());
}

#[test]
fn sprite_mapping_counts_and_lengths_are_bounded() {
    for players in [false, true] {
        for delta in [-1, 1] {
            let mut bundle = fixture();
            let maps = if players {
                &mut bundle.player_mappings
            } else {
                &mut bundle.object_mappings
            };
            maps.resize((maps.len() as isize + delta) as usize, vec![]);
            assert!(bundle.encode().is_err());
        }
        let mut bundle = fixture();
        let maps = if players {
            &mut bundle.player_mappings
        } else {
            &mut bundle.object_mappings
        };
        maps[0] = vec![
            MappingEntry {
                y: 0,
                x: 0,
                tile: 0,
                attr: 0
            };
            64
        ];
        assert!(bundle.validate().is_ok());
        let maps = if players {
            &mut bundle.player_mappings
        } else {
            &mut bundle.object_mappings
        };
        maps[0].push(MappingEntry {
            y: 0,
            x: 0,
            tile: 0,
            attr: 0,
        });
        assert!(bundle.validate().is_err());
    }
}

#[test]
fn invalid_json_byte_values_mapping_offsets_and_payload_size_are_rejected() {
    let value = serde_json::to_value(fixture()).unwrap();
    for number in [-1, 256] {
        let mut bad = value.clone();
        bad["hud_tiles"][0] = number.into();
        assert!(decode(&bad).is_err());
    }
    for offset in [-129, 128] {
        let mut bad = value.clone();
        bad["player_mappings"][0] = serde_json::json!([{"y":offset,"x":0,"tile":0,"attr":0}]);
        assert!(decode(&bad).is_err());
    }
    assert!(Bundle::decode(b"{").is_err());
    assert!(Bundle::decode(&vec![b' '; crate::cache::MAX_PAYLOAD_BYTES + 1]).is_err());
}

#[cfg(unix)]
#[test]
fn fingerprint_rejects_nested_and_required_file_symlinks() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("levels")).unwrap();
    fs::write(root.path().join("levels/level01.manifest.json"), b"{}").unwrap();
    std::os::unix::fs::symlink("levels/level01.manifest.json", root.path().join("alias")).unwrap();
    assert!(base_fingerprint(root.path()).is_err());
    fs::remove_file(root.path().join("alias")).unwrap();
    fs::remove_file(root.path().join("levels/level01.manifest.json")).unwrap();
    fs::write(root.path().join("manifest"), b"{}").unwrap();
    std::os::unix::fs::symlink(
        "../manifest",
        root.path().join("levels/level01.manifest.json"),
    )
    .unwrap();
    assert!(base_fingerprint(root.path()).is_err());
}

#[test]
fn fingerprint_streaming_and_resource_limits() {
    let root = tempfile::tempdir().unwrap();
    assert!(base_fingerprint(root.path()).is_err());
    fs::create_dir(root.path().join("levels")).unwrap();
    fs::write(root.path().join("levels/level01.manifest.json"), b"{}").unwrap();
    assert!(fingerprint_with_limits(root.path(), 0, 100, 1024).is_err());
    assert!(fingerprint_with_limits(root.path(), 32, 0, 1024).is_err());
    assert!(fingerprint_with_limits(root.path(), 32, 100, 1).is_err());
    assert!(fingerprint_with_limits(root.path(), 1, 1, 2).is_ok());
    let large = root.path().join("large.bin");
    let bytes = vec![0x5au8; 64 * 1024 + 17];
    fs::write(&large, &bytes).unwrap();
    let original = base_fingerprint(root.path()).unwrap();
    let mut bytes = bytes;
    bytes[64 * 1024 + 16] ^= 1;
    fs::write(&large, &bytes).unwrap();
    assert_ne!(base_fingerprint(root.path()).unwrap(), original);
    fs::write(&large, vec![0x5au8; 64 * 1024 + 17]).unwrap();
    fs::rename(&large, root.path().join("renamed.bin")).unwrap();
    assert_ne!(base_fingerprint(root.path()).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn fingerprint_distinguishes_literal_backslashes_from_path_separators() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("levels")).unwrap();
    fs::write(root.path().join("levels/level01.manifest.json"), b"{}").unwrap();
    fs::write(root.path().join("levels\\extra"), b"a").unwrap();
    let literal = base_fingerprint(root.path()).unwrap();
    fs::rename(
        root.path().join("levels\\extra"),
        root.path().join("levels/extra"),
    )
    .unwrap();
    assert_ne!(base_fingerprint(root.path()).unwrap(), literal);
}

#[test]
fn legacy_schema_without_character_field_round_trips_and_keeps_mario() {
    let mut value = serde_json::to_value(fixture()).unwrap();
    value["schema"] = 1.into();
    value.as_object_mut().unwrap().remove("luigi");
    let legacy = decode(&value).unwrap();
    assert!(legacy.supports_character(Character::Mario));
    assert!(!legacy.supports_character(Character::Luigi));
    assert_eq!(legacy.schema, 1);
    assert!(legacy.luigi.is_none());
    let encoded = legacy.encode().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&encoded).unwrap(),
        value
    );
    assert_eq!(Bundle::decode(&encoded).unwrap().encode().unwrap(), encoded);
}

#[test]
fn character_presence_matches_schema_and_new_bundle_supports_both() {
    let bundle = fixture();
    assert!(bundle.supports_character(Character::Mario));
    assert!(bundle.supports_character(Character::Luigi));
    let value = serde_json::to_value(bundle).unwrap();
    for schema in [1, BUNDLE_SCHEMA] {
        let mut bad = value.clone();
        bad["schema"] = schema.into();
        if schema == BUNDLE_SCHEMA {
            bad.as_object_mut().unwrap().remove("luigi");
        }
        assert!(decode(&bad).is_err());
    }
    let mut bad = value;
    bad["luigi"] = serde_json::Value::Null;
    assert!(decode(&bad).is_err());
}

#[test]
fn luigi_graphics_reject_malformed_lengths_counts_and_unknown_fields() {
    let value = serde_json::to_value(fixture()).unwrap();
    for field in ["normal", "fire", "bank1"] {
        for size in [0x7ff, 0x801] {
            let mut bad = value.clone();
            bad["luigi"][field] = serde_json::json!(vec![0u8; size]);
            assert!(
                decode(&bad).is_err(),
                "accepted Luigi {field} length {size}"
            );
        }
        let mut bad = value.clone();
        bad["luigi"].as_object_mut().unwrap().remove(field);
        assert!(decode(&bad).is_err());
    }
    for count in [241, 243] {
        let mut bundle = fixture();
        bundle
            .luigi
            .as_mut()
            .unwrap()
            .mappings
            .resize(count, vec![]);
        assert!(bundle.validate().is_err());
    }
    let mut bundle = fixture();
    bundle.luigi.as_mut().unwrap().mappings[0] = vec![
        MappingEntry {
            y: -12,
            x: -7,
            tile: 1,
            attr: 0x22
        };
        64
    ];
    assert!(bundle.validate().is_ok());
    bundle.luigi.as_mut().unwrap().mappings[0].push(MappingEntry {
        y: 0,
        x: 0,
        tile: 0,
        attr: 0,
    });
    assert!(bundle.validate().is_err());
    let mut bad = value;
    bad["luigi"]["unrecognized"] = true.into();
    assert!(decode(&bad).is_err());
}

#[test]
fn luigi_signed_mapping_cells_round_trip_without_losing_attributes() {
    let mut bundle = fixture();
    let cell = MappingEntry {
        y: -12,
        x: -7,
        tile: 1,
        attr: 0x22,
    };
    bundle.luigi.as_mut().unwrap().mappings[0].push(cell.clone());
    let decoded = Bundle::decode(&bundle.encode().unwrap()).unwrap();
    assert_eq!(decoded.luigi.unwrap().mappings[0][0], cell);
}
