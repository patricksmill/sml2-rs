use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn boss_screen_input_starts_at_source_initial_value() {
    assert_eq!(ObjectSim::new(vec![], 80).player_screen_x, 64);
    assert_eq!(ObjectSim::new(vec![], 80).player_screen_y, 64);
}

#[test]
fn boss_screen_y_tracks_live_player_camera_and_wrap_without_draw_pan() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; crate::level::LAYOUT_SIZE]).unwrap();
    let mut obj = ObjectSim::new(vec![], 80);
    for (player_y, camera_y, expected) in [
        (100, 72, 126),
        (100, 80, 118),
        (104, 80, 122),
        (0x0104, 0x00FC, 106),
        (0x0004, 0x01FC, 106),
        (0x0010, 0x0090, 226),
        (0x0004, 0xFFFC, 106),
        (0xFFFC, 0x0004, 90),
    ] {
        for pan in [0,1,2,3,255] {
            let contact = MarioContact {x: 100,y: player_y,pan,scene_state: 0xF0,..Default::default()};
            obj.step(80,camera_y,&layout,&tables,&contact,true);
            assert_eq!(obj.player_screen_y,expected,"player={player_y}, camera={camera_y}, pan={pan}");
            assert_eq!(obj.mario.y,player_y);
        }
    }
}

#[test]
fn boss_screen_input_tracks_current_contact_and_camera_through_step() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; crate::level::LAYOUT_SIZE]).unwrap();
    let mut obj = ObjectSim::new(vec![], 80);
    // Each frame uses the live contact and the scene's boss-driver selection.
    // Literal results cover camera motion, player motion, low-byte crossings,
    // negative projection and the world-coordinate wrap.
    for (player_x, camera_x, expected) in [
        (100, 80, 116),
        (100, 88, 108),
        (104, 88, 112),
        (0x0104, 0x00FC, 104),
        (0x0004, 0x01FC, 104),
        (0x0010, 0x0090, 224),
        (0x0004, 0xFFFC, 104),
        (0xFFFC, 0x0004, 88),
    ] {
        let contact = MarioContact {
            x: player_x,
            y: 120,
            scene_state: 0xF0,
            ..Default::default()
        };
        obj.step(
            camera_x,
            72,
            &layout,
            &tables,
            &contact,
            contact.scene_state & 0xF0 != 0,
        );
        assert_eq!(
            obj.player_screen_x, expected,
            "player {player_x}, camera {camera_x}"
        );
        assert_eq!(obj.mario.x, player_x);
    }
    assert_eq!(obj.fff0, 8, "every sample reached the boss driver");
    assert_eq!(obj.prev_cam_x, 80, "the level driver did not run");
}
