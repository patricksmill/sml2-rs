//! Enlarged views: native parity, sprite centring, tile window, status bar.

use crate::render::view::NATIVE;
use crate::session::{FrameInput, Session, SessionAssets, SessionState};

fn level(id: u8) -> Session {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    session.begin_level(id, 0).expect("level loads");
    for _ in 0..90 {
        session.advance(FrameInput::default());
    }
    assert!(matches!(session.state, SessionState::InLevel(_)));
    session
}

fn crop(frame: &[u8], view: (usize, usize), rect: (usize, usize, usize, usize)) -> Vec<u8> {
    let (x0, y0, w, h) = rect;
    (y0..y0 + h)
        .flat_map(|y| frame[y * view.0 + x0..y * view.0 + x0 + w].to_vec())
        .collect()
}

#[test]
fn the_centre_of_an_enlarged_level_view_matches_native_above_the_status_bar() {
    let session = level(1);
    let native = session.render_frame(NATIVE);
    for view in [(256, 144), (320, 180), (384, 216), (200, 180), (240, 216)] {
        let wide = session.render_frame(view);
        let (mx, my) = crate::render::view::margin(view);
        assert_eq!(
            crop(&wide, view, (mx as usize, my as usize, 160, 136)),
            crop(&native, NATIVE, (0, 0, 160, 136)),
            "{view:?}"
        );
    }
}

#[test]
fn the_status_bar_sits_at_the_bottom_centre_of_an_enlarged_view() {
    let session = level(1);
    let native = session.render_frame(NATIVE);
    let view = (384, 216);
    let wide = session.render_frame(view);
    assert_eq!(
        crop(&wide, view, (112, 208, 160, 8)),
        crop(&native, NATIVE, (0, 136, 160, 8))
    );
}

#[test]
fn a_384_px_level_view_does_not_repeat_columns() {
    let session = level(1);
    let view = (384, 216);
    let frame = session.render_frame(view);
    let column = |x: usize| (0..200).map(|y| frame[y * view.0 + x]).collect::<Vec<_>>();
    let repeats = (0..128).filter(|&x| column(x) == column(x + 256)).count();
    // A 256-px wrap repeats every column but those Mario and the HUD cover.
    assert!(repeats < 64, "{repeats} of 128 columns repeat 256 px apart");
}
