use crate::session::{FrameInput, Session, SessionAssets, SessionState};
use sml2_core::player_sim::PlayerKeys;

fn zone(screen: u8) -> Session {
    let mut session = Session::new(SessionAssets::load().unwrap());
    if let SessionState::Overworld(map) = &mut session.state {
        map.node = 255;
        map.load_screen(screen, 0, &session.assets.node_records);
        map.cursor_x = 0;
        map.cursor_y = 0;
    }
    session
}

#[test]
fn zone_animation_frames_reach_the_normal_session_renderer() {
    for screen in [1, 2, 4, 5, 6, 7, 8] {
        let mut session = zone(screen);
        let initial = session.render_frame((160, 144));
        for frame in 0..64 {
            session.advance(FrameInput {
                keys: PlayerKeys::default(),
                div: frame,
            });
        }
        let frame = session.render_frame((160, 144));
        assert_ne!(frame, initial, "zone {screen} scenery must animate");
        if let SessionState::Overworld(map) = &session.state {
            assert_eq!(map.frame_counter, 64);
        } else {
            panic!("zone animation must stay in the overworld");
        }
    }
}

#[test]
fn repeated_zone_renders_do_not_advance_sprites_or_background_animation() {
    let mut session = zone(7);
    for _ in 0..16 {
        session.advance(FrameInput {
            keys: PlayerKeys::default(),
            div: 0,
        });
    }
    let before = match &session.state {
        SessionState::Overworld(map) => map.clone(),
        _ => unreachable!(),
    };
    let frame = session.render_frame((160, 144));
    assert_eq!(session.render_frame((160, 144)), frame);
    let _ = session.render_frame((320, 288));
    let after = match &session.state {
        SessionState::Overworld(map) => map,
        _ => unreachable!(),
    };
    assert_eq!(&before, after);
}

#[test]
fn mario_route_animation_changes_background_pixels_without_sprites() {
    let mut session = zone(6);
    if let SessionState::Overworld(map) = &mut session.state {
        map.zone_slots.fill((255, 255, 255));
    }
    let initial = session.render_frame((160, 144));
    for _ in 0..32 {
        session.advance(FrameInput {
            keys: PlayerKeys::default(),
            div: 0,
        });
    }
    if let SessionState::Overworld(map) = &mut session.state {
        map.zone_slots.fill((255, 255, 255));
    }
    assert_ne!(
        session.render_frame((160, 144)),
        initial,
        "animated route tiles reach the BG pass"
    );
}
