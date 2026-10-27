use super::{tests::app, *};
use crate::{input::PadButton, settings::Widescreen};

fn frame(app: &mut DesktopApp, ctx: &Context, size: [f32; 2], dpi: f32) {
    let mut raw = RawInput {
        focused: true,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(size[0], size[1]),
        )),
        ..Default::default()
    };
    let viewport = raw.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
    viewport.inner_rect = raw.screen_rect;
    viewport.native_pixels_per_point = Some(dpi);
    viewport.fullscreen = Some(app.settings.fullscreen);
    let _ = ctx.run(raw, |ctx| app.ui(ctx));
}

#[test]
fn widescreen_selector_moves_both_ways_through_the_new_options() {
    let (mut app, ctx, path) = app();
    app.navigate(&ctx, Screen::Settings);
    app.page = Page::Mods;
    super::tests::frame(&mut app, &ctx, vec![]);
    app.menu.selected = 1;
    for (button, expected) in [
        (Button::key("ArrowLeft"), Widescreen::WindowSize),
        (Button::key("ArrowLeft"), Widescreen::Wide21x9),
        (Button::Pad(PadButton::Left), Widescreen::Wide16x9),
        (Button::Pad(PadButton::Right), Widescreen::Wide21x9),
        (Button::key("ArrowRight"), Widescreen::WindowSize),
        (Button::key("Enter"), Widescreen::Off),
    ] {
        app.input.event(button.clone(), true);
        super::tests::frame(&mut app, &ctx, vec![]);
        app.input.event(button, false);
        super::tests::frame(&mut app, &ctx, vec![]);
        assert_eq!(app.settings.widescreen, expected);
    }
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn live_window_aspects_drive_native_and_dx_frames_and_spawn_margins() {
    for dx in [false, true] {
        let (mut app, ctx, path) = app();
        app.apply_debug(debug::DebugAction::WarpLevel(4));
        app.paused = true;
        app.settings.show_toolbar = false;
        app.settings.fullscreen = true;
        app.settings.dx_mode = dx;
        app.dx_bundle = Some((super::dx_tests::red_bundle()).into());
        let saved_window = app.settings.window_size;
        for (wide, size, dpi, expected) in [
            (Widescreen::Wide21x9, [800.0, 720.0], 1.0, (336, 144)),
            (Widescreen::Wide21x9, [1200.0, 720.0], 1.0, (336, 144)),
            (Widescreen::WindowSize, [1920.0, 1080.0], 1.0, (256, 144)),
            (Widescreen::WindowSize, [2520.0, 1080.0], 2.0, (336, 144)),
            (Widescreen::WindowSize, [800.0, 800.0], 1.0, (160, 160)),
            (Widescreen::WindowSize, [480.0, 960.0], 1.0, (160, 320)),
        ] {
            app.settings.widescreen = wide;
            frame(&mut app, &ctx, size, dpi);
            assert_eq!(app.view(), expected);
            assert_eq!(app.texture.size(), [expected.0, expected.1]);
            assert_eq!(app.settings.window_size, saved_window);
            assert!(app.game_rect[2] > 0 && app.game_rect[3] > 0);

            let game = app.game.as_mut().unwrap();
            game.session.advance(Default::default());
            let sml2_runtime::session::SessionState::InLevel(level) = &game.session.state else {
                panic!("expected a level");
            };
            assert_eq!(
                level.obj.view_margin,
                (
                    ((expected.0 - 160) / 2) as u16,
                    ((expected.1 - 144) / 2) as u16
                ),
                "spawning and rendering share the live view, DX {dx}"
            );
        }
        std::fs::remove_dir_all(path).ok();
    }
}

#[test]
fn window_size_excludes_the_toolbar_and_tracks_camera_zoom() {
    let (mut app, ctx, path) = app();
    app.paused = true;
    app.settings.widescreen = Widescreen::WindowSize;
    app.settings.show_toolbar = false;
    frame(&mut app, &ctx, [1920.0, 1080.0], 1.0);
    assert_eq!(app.view(), (256, 144));

    app.settings.show_toolbar = true;
    frame(&mut app, &ctx, [1920.0, 1080.0], 1.0);
    let [width, height] = app.viewport_size.unwrap();
    assert_eq!(width, 1920);
    assert!(height < 1080);
    assert_eq!(app.view(), (144 * width as usize / height as usize, 144));
    assert!(app.view().0 > 256);

    app.settings.camera_zoom = crate::settings::CameraZoom::Zoom200;
    frame(&mut app, &ctx, [1920.0, 1080.0], 1.0);
    assert_eq!(app.view(), (288 * width as usize / height as usize, 288));
    assert_eq!(app.texture.size(), [app.view().0, app.view().1]);
    std::fs::remove_dir_all(path).ok();
}
