use crate::session::{FrameInput, Session, SessionAssets, SessionState, StepOutcome};
use sml2_core::player_sim::PlayerKeys;

fn step(session: &mut Session, keys: PlayerKeys) {
    assert_eq!(
        session.advance(FrameInput { keys, div: 0x3D }).outcome,
        StepOutcome::Continue
    );
}

#[test]
fn releasing_pan_blanks_stops_music_recenters_and_fades_back_before_input_resumes() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    step(&mut session, PlayerKeys::default());
    assert_eq!(session.sound_driver.active_song, 6);
    let initial_camera = match &session.state {
        SessionState::Overworld(map) => (map.cam_x, map.cam_y),
        _ => unreachable!(),
    };
    let progress = session.save_record();
    for _ in 0..8 {
        step(
            &mut session,
            PlayerKeys {
                b: true,
                right: true,
                ..Default::default()
            },
        );
    }
    match &session.state {
        SessionState::Overworld(map) => {
            assert_ne!((map.cam_x, map.cam_y), initial_camera);
            assert_eq!(map.scroll_only_latch, 0xFF);
        }
        _ => unreachable!(),
    }

    step(&mut session, PlayerKeys::default());
    assert!(
        session.map_pan_return.is_some(),
        "the loader must take time"
    );
    assert_eq!(session.sound_driver.active_song, 0, "$3F160 requests $FF");
    assert!(session.render_frame((160, 144)).iter().all(|&p| p == 255));
    let frozen_map = match &session.state {
        SessionState::Overworld(map) => {
            assert_eq!((map.cam_x, map.cam_y), initial_camera);
            assert_eq!(map.scroll_only_latch, 0);
            assert_eq!(map.cursor_anim, 0);
            map.clone()
        }
        _ => unreachable!(),
    };
    let frozen_wave = session.map_wave.clone();

    for frame in 1..=32 {
        // Held directions, B and A must neither pan nor enter a level during
        // the blocking transfer and four three-frame palette waits.
        step(
            &mut session,
            PlayerKeys {
                b: true,
                right: true,
                a: true,
                ..Default::default()
            },
        );
        match &session.state {
            SessionState::Overworld(map) => assert_eq!(map, &frozen_map),
            _ => panic!("input must not enter a level during the reload"),
        }
        assert!(session.pending_level.is_none());
        assert_eq!(
            session.map_wave.as_ref().unwrap().strips,
            frozen_wave.as_ref().unwrap().strips
        );
        assert_eq!(
            session.sound_driver.active_song,
            if frame < 20 { 0 } else { 6 }
        );
        let pixels = session.render_frame((160, 144));
        match frame {
            1..=25 => assert!(pixels.iter().all(|&p| p == 255)),
            26..=28 => {
                assert!(pixels.iter().all(|&p| p == 255 || p == 170), "BGP/OBP1 $40");
                assert!(pixels.contains(&170));
            }
            29..=31 => {
                assert!(
                    pixels.iter().all(|&p| p == 255 || p == 170 || p == 85),
                    "BGP $90, OBP1 $80"
                );
                assert!(pixels.contains(&85));
            }
            32 => assert!(pixels.contains(&0), "the normal palette returns"),
            _ => unreachable!(),
        }
    }
    assert!(session.map_pan_return.is_none());
    assert_eq!(
        session.save_record()[..0x40],
        progress[..0x40],
        "progress survives the reload"
    );

    step(&mut session, PlayerKeys::default());
    step(
        &mut session,
        PlayerKeys {
            a: true,
            ..Default::default()
        },
    );
    assert!(
        session.pending_level.is_some(),
        "fresh input works after the fade"
    );
}

#[test]
fn holding_b_without_directions_does_not_repeat_the_reload_until_release() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    for _ in 0..40 {
        step(
            &mut session,
            PlayerKeys {
                b: true,
                ..Default::default()
            },
        );
        assert!(session.map_pan_return.is_none());
        assert_eq!(session.sound_driver.active_song, 6);
    }
    step(&mut session, PlayerKeys::default());
    assert!(session.map_pan_return.is_some());
    for _ in 0..40 {
        step(&mut session, PlayerKeys::default());
    }
    assert!(session.map_pan_return.is_none());
    assert_eq!(session.sound_driver.active_song, 6);
}
