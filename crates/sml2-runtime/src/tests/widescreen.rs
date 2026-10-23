//! Enlarged views: native parity, sprite centring, tile window, status bar.

use crate::render::view::NATIVE;
use crate::session::{FrameInput, Session, SessionAssets, SessionState};
use sml2_core::player_sim::PlayerKeys;
use sml2_core::scenery::{SceneryOverlay, SceneryPaint};

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
fn the_biased_native_rect_matches_native_above_the_status_bar() {
    let session = level(1);
    let native = session.render_frame(NATIVE);
    for view in [
        (256, 144),
        (320, 180),
        (384, 216),
        (200, 180),
        (240, 216),
        (320, 288),
        (512, 288),
    ] {
        let wide = session.render_frame(view);
        let (mx, my) = crate::render::view::level_margin(view);
        assert_eq!(
            crop(&wide, view, (mx as usize, my as usize, 160, 136)),
            crop(&native, NATIVE, (0, 0, 160, 136)),
            "{view:?}"
        );
    }
}

#[test]
fn an_unpainted_margin_continues_the_playfield_edge() {
    let session = level(0);
    let native = session.render_frame(NATIVE);
    let view = (384, 216);
    let wide = session.render_frame(view);
    let (mx, my) = crate::render::view::level_margin(view);
    assert_eq!(
        crop(&wide, view, (mx as usize, my as usize, 160, 136)),
        crop(&native, NATIVE, (0, 0, 160, 136))
    );
    for row in 0..136 {
        assert_eq!(
            wide[(my as usize + row) * view.0],
            native[row * 160],
            "left edge row {row}"
        );
    }
    let below = (my + 144) as usize;
    assert_eq!(
        crop(&wide, view, (mx as usize, below, 160, 1)),
        crop(&native, NATIVE, (0, 135, 160, 1))
    );
}

#[test]
fn a_paint_shows_in_the_margin_and_a_native_frame_ignores_it() {
    let mut session = level(0);
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("in level")
    };
    ctx.cam.x = 200;
    ctx.cam.y = 448;
    if let Some(spr) = ctx.spr.as_mut() {
        spr.scroll_pan = 0;
    }
    let count = ctx.manifest.metatiles.metatiles.len();
    let native = session.render_frame(NATIVE);
    let view = (384, 216);
    let plain = session.render_frame(view);
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("in level")
    };
    ctx.scenery = SceneryOverlay::try_from_paints(
        vec![SceneryPaint {
            x: 7,
            y: 24,
            metatile: 1,
        }],
        count,
    )
    .unwrap();
    assert_eq!(session.render_frame(NATIVE), native);
    let wide = session.render_frame(view);
    let (mx, my) = crate::render::view::level_margin(view);
    assert_eq!(
        crop(&wide, view, (mx as usize, my as usize, 160, 136)),
        crop(&native, NATIVE, (0, 0, 160, 136))
    );
    // View (111, 76) is cell (7, 24), just left of the native picture.
    assert_ne!(wide[76 * view.0 + 111], plain[76 * view.0 + 111]);
    assert_eq!(wide[76 * view.0 + 112], plain[76 * view.0 + 112]);
}

#[test]
fn insert_level_carries_scenery_and_an_arena_ignores_it() {
    let mut assets = SessionAssets::load().unwrap();
    let mut content = assets.level(0).unwrap().clone();
    content.scenery = SceneryOverlay::try_from_paints(
        vec![SceneryPaint {
            x: 7,
            y: 24,
            metatile: 1,
        }],
        content.manifest.metatiles.metatiles.len(),
    )
    .unwrap();
    assets.insert_level(0, content);
    let mut session = Session::new(assets);
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("in level")
    };
    ctx.cam.x = 200;
    ctx.cam.y = 448;
    if let Some(spr) = ctx.spr.as_mut() {
        spr.scroll_pan = 0;
    }
    let painted = session.render_frame((384, 216));
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("in level")
    };
    ctx.arena_loaded = true;
    drop(ctx);
    let arena = session.render_frame((384, 216));
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("in level")
    };
    ctx.scenery = SceneryOverlay::empty();
    drop(ctx);
    let bare = session.render_frame((384, 216));
    assert_ne!(painted[76 * 384 + 111], arena[76 * 384 + 111]);
    assert_eq!(arena, bare);
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

#[test]
fn set_view_feeds_the_level_spawn_margin_but_not_demos() {
    let mut session = level(1);
    session.set_view((384, 216));
    session.advance(FrameInput::default());
    let SessionState::InLevel(ctx) = &session.state else {
        panic!("in level")
    };
    assert_eq!(ctx.obj.view_margin, (112, 36));
    assert_eq!(ctx.obj.view_bias_y, 18);
    session.set_view(NATIVE);
    session.advance(FrameInput::default());
    let SessionState::InLevel(ctx) = &session.state else {
        panic!("in level")
    };
    assert_eq!((ctx.obj.view_margin, ctx.obj.view_bias_y), ((0, 0), 0));
    session.set_view((384, 216));
    assert!(session.start_demo(0));
    session.advance(FrameInput::default());
    let SessionState::Demo(demo) = &session.state else {
        panic!("in demo")
    };
    assert_eq!(demo.ctx.obj.view_margin, (0, 0));
    assert_eq!(demo.ctx.obj.view_bias_y, 0);
}

#[test]
fn front_end_screens_keep_their_centre_and_continue_their_backdrops() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.enter_title();
    let native = session.render_frame(NATIVE);
    let view = (384, 216);
    let wide = session.render_frame(view);
    assert_eq!(crop(&wide, view, (112, 36, 160, 144)), native);
    // The wall checker repeats every 16 px, so it lines up across the seam.
    assert_eq!(
        crop(&wide, view, (96, 36, 16, 8)),
        crop(&native, NATIVE, (0, 0, 16, 8))
    );

    let step = |session: &mut Session, keys: PlayerKeys| {
        session.advance(FrameInput { keys, div: 0 });
    };
    for _ in 0..64 {
        step(&mut session, PlayerKeys::default());
    }
    step(
        &mut session,
        PlayerKeys {
            start: true,
            ..Default::default()
        },
    );
    for _ in 0..46 {
        step(&mut session, PlayerKeys::default());
    }
    assert!(matches!(session.state, SessionState::FileSelect(_)));
    let native = session.render_frame(NATIVE);
    let wide = session.render_frame(view);
    assert_eq!(crop(&wide, view, (112, 36, 160, 144)), native);
    // The 2×2 block checker repeats every 16 px across the seam.
    assert_eq!(
        crop(&wide, view, (96, 36, 16, 8)),
        crop(&native, NATIVE, (0, 0, 16, 8))
    );
    // The pipe in column 2 runs on above the screen.
    assert_eq!(
        crop(&wide, view, (112 + 16, 28, 8, 8)),
        crop(&native, NATIVE, (16, 0, 8, 8))
    );
}

#[test]
fn the_hub_view_stays_inside_the_map_and_keeps_the_native_centre() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    let SessionState::Overworld(map) = &mut session.state else {
        panic!("boots to map")
    };
    map.load_screen(0, 0, &session.assets.node_records);
    map.cam_x = 0xC0;
    map.cam_y = 0xC0;
    let native = session.render_frame(NATIVE);
    let view = (384, 216);
    let wide = session.render_frame(view);
    // The camera is far from the hub edges, so the view is centred on it.
    assert_eq!(
        crop(&wide, view, (112, 36, 160, 136)),
        crop(&native, NATIVE, (0, 0, 160, 136))
    );
    // At the corner the view clamps instead of wrapping.
    let SessionState::Overworld(map) = &mut session.state else {
        unreachable!()
    };
    map.cam_x = 0;
    map.cam_y = 0;
    let native = session.render_frame(NATIVE);
    let wide = session.render_frame(view);
    assert_eq!(
        crop(&wide, view, (0, 0, 160, 136)),
        crop(&native, NATIVE, (0, 0, 160, 136))
    );
}

#[test]
fn a_hub_view_wider_than_the_map_is_letterboxed() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    let SessionState::Overworld(map) = &mut session.state else {
        panic!("boots to map")
    };
    map.load_screen(0, 0, &session.assets.node_records);
    let full = session.render_frame((512, 144));
    let view = (600, 144);
    let wide = session.render_frame(view);
    assert_eq!(wide.len(), 600 * 144);
    assert_eq!(crop(&wide, view, (44, 0, 512, 144)), full);
}

#[test]
fn a_zone_screen_is_drawn_whole_and_centred() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    let SessionState::Overworld(map) = &mut session.state else {
        panic!("boots to map")
    };
    map.load_screen(1, 0, &session.assets.node_records);
    let full = session.render_frame((256, 144));
    let view = (384, 216);
    let wide = session.render_frame(view);
    assert_eq!(crop(&wide, view, (64, 36, 256, 144)), full);
}
