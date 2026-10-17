use super::*;

fn assets(seed: u8) -> EndingAssets {
    EndingAssets::new(
        (0..0x1800)
            .map(|i| seed.wrapping_add((i / 16) as u8))
            .collect(),
        (0..0x400).map(|i| i as u8).collect(),
    )
    .unwrap()
}

#[test]
fn assets_validate_schema_lengths_and_keep_raw_rendering_data() {
    let original = assets(7);
    let text = serde_json::to_string(&original).unwrap();
    assert_eq!(EndingAssets::from_json(&text).unwrap(), original);
    assert!(EndingAssets::from_json(&text.replace("sml2-ending/1", "sml2-ending/2")).is_err());
    assert!(EndingAssets::from_json("{}").is_err());
    assert!(EndingAssets::from_json(&text[..text.len() - 1]).is_err());
    for len in [0, 0x17ff, 0x1801] {
        assert!(EndingAssets::new(vec![0; len], vec![0; 0x400]).is_err());
    }
    for len in [0, 0x3ff, 0x401] {
        assert!(EndingAssets::new(vec![0; 0x1800], vec![0; len]).is_err());
    }
    let mut malformed: serde_json::Value = serde_json::from_str(&text).unwrap();
    malformed["tiles"].as_array_mut().unwrap().pop();
    assert!(EndingAssets::from_json(&malformed.to_string()).is_err());
    assert_eq!(original.tiles().len(), 384 * 16);
    assert_eq!(original.tilemap().len(), 32 * 32);
    for id in 0..=255 {
        let signed_offset = (0x1000i32 + i32::from(id as i8) * 16) as usize;
        assert_eq!(
            original.background_tile(id),
            &original.tiles()[signed_offset..signed_offset + 16]
        );
        assert_eq!(
            original.object_tile(id),
            &original.tiles()[usize::from(id) * 16..usize::from(id) * 16 + 16]
        );
    }
}

#[test]
fn helper_029dd_reloads_dirty_scene_and_wraps_only_ending_substate() {
    let mut scene = EndingScene {
        assets: Some(assets(0)),
        scroll_x: 97,
        scroll_y: 31,
        palettes: Some(EndingPalettes {
            background: 1,
            object0: 2,
            object1: 3,
        }),
        objects: [Cell {
            y: 0xff,
            x: 0xff,
            tile: 0xff,
            attr: 0xff,
        }; 40],
        credits_sprite_y: 9,
        display: EndingDisplayMode {
            window_enabled: true,
            tall_sprites: true,
            background_unsigned_tiles: true,
            background_upper_map: true,
            ..Default::default()
        },
    };
    // These are separate controller/world fields, not owned by the scene-load helper.
    let unrelated_world = [0xa5, 0x19, 0x77];
    let mut fade = 0xabcd;
    let mut state = 4;
    let new_assets = assets(91);
    scene.load(new_assets.clone(), &mut state, &mut fade);
    assert_eq!(scene.assets(), Some(&new_assets));
    assert_eq!([scene.scroll_y, scene.scroll_x], [0, 0]);
    assert_eq!(
        scene.palettes,
        Some(EndingPalettes {
            background: 0xe1,
            object0: 0xd2,
            object1: 0x39
        })
    );
    let oam_bytes: Vec<u8> = scene
        .objects
        .iter()
        .flat_map(|cell| [cell.y, cell.x, cell.tile, cell.attr])
        .collect();
    assert_eq!(oam_bytes, [0; 0xa0]);
    assert_eq!(
        scene.display,
        EndingDisplayMode {
            display_enabled: true,
            window_upper_map: true,
            objects_enabled: true,
            background_enabled: true,
            ..Default::default()
        }
    );
    assert_eq!(fade.to_le_bytes(), [0, 1]);
    assert_eq!(scene.credits_sprite_y, 0x2c);
    assert_eq!(state, 5);
    assert_eq!(unrelated_world, [0xa5, 0x19, 0x77]);
    state = 255;
    scene.load(assets(1), &mut state, &mut fade);
    assert_eq!(state, 0);
}
