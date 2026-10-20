use crate::session::{FrameInput, Session, SessionAssets, SessionState};
use sml2_core::player_sim::PlayerKeys;

fn tick(session: &mut Session, keys: PlayerKeys) {
    session.advance(FrameInput { keys, div: 0x3D });
}

#[test]
fn normal_doors_load_zones_walk_48_pixels_and_block_input() {
    // ASM door setup + 48 calls to $3E426, including wrapped starting bytes.
    for (node, screen, dest, x, y) in [
        (0x19, 2, 0x23, 48, 110),
        (0x1C, 1, 0x28, 83, 89),
        (0x1B, 3, 0x32, 18, 118),
        (0x12, 4, 0x3C, 64, 100),
        (0x0D, 5, 0x46, 75, 34),
        (0x67, 5, 0x4B, 146, 26),
        (0x1D, 6, 0x50, 60, 112),
        (0x11, 7, 0x5A, 81, 137),
        // $3D848 loads first, then lands the cursor at 212/131: 48 px right
        // wraps it to 4 (the ROM's own trace).
        (0x15, 8, 0x72, 4, 131),
    ] {
        let mut session = Session::new(SessionAssets::load().unwrap());
        if let SessionState::Overworld(map) = &mut session.state {
            map.node = node;
            map.load_node(node, &session.assets.node_records);
            map.cursor_move = 0;
        }
        tick(
            &mut session,
            PlayerKeys {
                a: true,
                ..Default::default()
            },
        );
        assert!(session.map_screen_transition.is_some(), "door {node:02X}");
        for _ in 0..160 {
            if session.map_screen_transition.is_none() {
                break;
            }
            tick(
                &mut session,
                PlayerKeys {
                    a: true,
                    b: true,
                    right: true,
                    ..Default::default()
                },
            );
            assert!(session.pending_level.is_none());
        }
        assert!(session.map_screen_transition.is_none());
        let SessionState::Overworld(map) = &session.state else {
            panic!()
        };
        assert_eq!(
            [map.screen_id, map.node, map.cursor_x, map.cursor_y],
            [screen, dest, x, y],
            "door {node:02X}"
        );
        // Zone loaders zero the camera; `$3D880` instead ends in `$3EEA4` on the
        // hub node it left, which the ROM leaves in `$FFB7-$FFBA` (209/282).
        let cam = if node == 0x15 { (209, 282) } else { (0, 0) };
        assert_eq!((map.cam_x, map.cam_y), cam, "door {node:02X}");
        assert_eq!(
            session.sound_driver.active_song,
            sml2_core::map_sim::screen_music(screen)
        );
        // The zone loader replaced the default slots. Visibility is not the
        // test: Macro's list ($0F) is all `$FF` ids until its stages clear.
        assert_ne!(
            map.zone_slots,
            sml2_core::map_sim::MapSim::new().zone_slots,
            "door {node:02X}: the zone sprite list was loaded"
        );
    }
}

#[test]
fn tree_door_walks_out_then_reloads_and_fades_back_to_hub() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    if let SessionState::Overworld(map) = &mut session.state {
        map.node = 0x23;
        map.load_screen(2, 0, &session.assets.node_records);
        map.load_node(0x23, &session.assets.node_records);
        map.mario_dead = 0xFF;
        map.cursor_move = 0;
    }
    tick(
        &mut session,
        PlayerKeys {
            down: true,
            ..Default::default()
        },
    );
    let transition = session.map_screen_transition.as_ref().unwrap();
    assert!(!transition.entering);
    assert!(transition.shows_source());
    let start_y = transition.source.cursor_y;
    for _ in 0..100 {
        tick(&mut session, PlayerKeys::default());
    }
    assert!(session.map_screen_transition.is_none());
    assert!(session.map_pan_return.is_some());
    assert!(session.render_frame((160, 144)).iter().all(|&p| p == 255));
    for _ in 0..32 {
        tick(&mut session, PlayerKeys::default());
    }
    assert!(session.map_pan_return.is_none());
    let SessionState::Overworld(map) = &session.state else {
        panic!()
    };
    let record = session.assets.node_records.record(25).unwrap();
    assert_eq!(
        [map.screen_id, map.node, map.cursor_x, map.cursor_y],
        [0, 25, record.cursor_x, record.cursor_y]
    );
    assert_ne!(start_y, map.cursor_y);
    assert_eq!((map.cam_x, map.cam_y), (record.cam_x, record.cam_y));
    assert_eq!(session.sound_driver.active_song, 6);
}

#[test]
fn screen_seven_holds_its_palettes_instead_of_fading_in() {
    // `$3E4C1` (node $11's door) against `$3E3BA`'s fade-in (node $19's).
    for (node, fixed) in [(0x11, true), (0x19, false)] {
        let mut session = Session::new(SessionAssets::load().unwrap());
        if let SessionState::Overworld(map) = &mut session.state {
            map.node = node;
            map.load_node(node, &session.assets.node_records);
            map.cursor_move = 0;
        }
        tick(
            &mut session,
            PlayerKeys {
                a: true,
                ..Default::default()
            },
        );
        for _ in 0..50 {
            tick(&mut session, PlayerKeys::default());
        }
        let SessionState::Overworld(map) = &session.state else {
            panic!()
        };
        let transition = session.map_screen_transition.as_ref().unwrap();
        assert!(
            !transition.shows_source(),
            "door {node:02X} is past the hub fade"
        );
        assert_eq!(
            transition.palettes(map.screen_id) == (0xE1, 0xD0, 0x39),
            fixed,
            "door {node:02X}"
        );
    }
}
