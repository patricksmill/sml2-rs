use crate::app::editor::state::Tool;
use crate::app::{tests::app, EditorApp, Screen};
use eframe::egui::{self, Context, Event, Key, Modifiers, PointerButton, Pos2, Rect};

pub(in crate::app) fn frame(
    app: &mut EditorApp,
    ctx: &Context,
    size: (f32, f32),
    events: Vec<Event>,
) -> egui::FullOutput {
    let modifiers = events
        .iter()
        .find_map(|event| match event {
            Event::Key { modifiers, .. }
            | Event::PointerButton { modifiers, .. }
            | Event::MouseWheel { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or_default();
    let mut raw = egui::RawInput {
        focused: true,
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(size.0, size.1))),
        events,
        modifiers,
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .inner_rect = raw.screen_rect;
    app.receive_input(&raw);
    ctx.run(raw, |ctx| app.ui(ctx))
}

fn draws_texture(shape: &egui::Shape, texture: egui::TextureId) -> bool {
    match shape {
        egui::Shape::Mesh(mesh) => mesh.texture_id == texture,
        egui::Shape::Rect(rect) => rect
            .brush
            .as_ref()
            .is_some_and(|brush| brush.fill_texture_id == texture),
        egui::Shape::Vec(shapes) => shapes.iter().any(|shape| draws_texture(shape, texture)),
        _ => false,
    }
}

fn pointer(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn playtest_window_aspect_follows_resizing_without_changing_saved_preferences() {
    let (mut app, ctx, path) = app();
    app.settings.widescreen = crate::settings::Widescreen::WindowSize;
    let preferences = app.settings.clone();
    app.open_editor(&ctx);
    app.play_editor(&ctx);
    app.paused = true;
    let mut widths = Vec::new();
    for size in [(800.0, 720.0), (1600.0, 720.0)] {
        frame(&mut app, &ctx, size, vec![]);
        assert_eq!(app.screen, Screen::Game);
        let [width, height] = app.texture.size();
        assert!(width >= 160);
        assert_eq!(height, 144);
        assert_eq!(app.settings, preferences);
        widths.push(width);
    }
    assert!(widths[1] > widths[0]);
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn layout_fits_small_normal_and_large_windows_and_reuses_the_atlas() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let game_view = app.settings.view();
    for size in [(480.0, 400.0), (800.0, 720.0), (1600.0, 1000.0)] {
        // Egui can request a sizing pass on the first frame after a resize.
        frame(&mut app, &ctx, size, vec![]);
        let output = frame(&mut app, &ctx, size, vec![]);
        let rect = app.editor.canvas_rect.unwrap();
        assert!(rect.width() >= size.0 - 20.0, "{size:?}: {rect:?}");
        assert!(rect.height() >= 48.0, "{size:?}: {rect:?}");
        assert!(
            rect.bottom() <= size.1 && rect.right() <= size.0,
            "{size:?}: {rect:?}"
        );
        assert!(!output.shapes.is_empty());
        let atlas = app.editor.atlas.as_ref().unwrap().texture.id();
        frame(&mut app, &ctx, size, vec![]);
        assert_eq!(app.editor.atlas.as_ref().unwrap().texture.id(), atlas);
        assert_eq!(app.settings.view(), game_view);
    }
    let old = app.editor.atlas.as_ref().unwrap().texture.id();
    app.cycle_filter(1);
    frame(&mut app, &ctx, (800.0, 720.0), vec![]);
    assert_ne!(app.editor.atlas.as_ref().unwrap().texture.id(), old);
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn pointer_drag_saves_on_release_and_command_keys_undo_after_button_focus() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 1;
    work.viewport.center = egui::pos2(0.0, 400.0);
    let size = (800.0, 720.0);
    frame(&mut app, &ctx, size, vec![]);
    let rect = app.editor.canvas_rect.unwrap();
    let viewport = app.editor.work.as_ref().unwrap().viewport;
    let start = viewport.screen(rect, egui::pos2(-88.0, 392.0));
    let end = viewport.screen(rect, egui::pos2(-8.0, 392.0));
    frame(
        &mut app,
        &ctx,
        size,
        vec![
            Event::PointerMoved(start),
            pointer(start, PointerButton::Primary, true),
        ],
    );
    frame(&mut app, &ctx, size, vec![Event::PointerMoved(end)]);
    assert_eq!(app.editor.work.as_ref().unwrap().scenery.len(), 6);
    assert!(!path.join("scenery-00.json").exists());
    frame(
        &mut app,
        &ctx,
        size,
        vec![pointer(end, PointerButton::Primary, false)],
    );
    let saved = std::fs::read(path.join("scenery-00.json")).unwrap();
    let overlay: sml2_core::scenery::SceneryOverlay = serde_json::from_slice(&saved).unwrap();
    assert_eq!(overlay.len(), 6);
    // A focused button must not disable editor shortcuts.
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("tool-button")));
    frame(
        &mut app,
        &ctx,
        size,
        vec![key(Key::Z, Modifiers::CTRL | Modifiers::COMMAND)],
    );
    assert!(app.editor.work.as_ref().unwrap().scenery.is_empty());
    frame(
        &mut app,
        &ctx,
        size,
        vec![Event::Key {
            key: Key::Z,
            physical_key: Some(Key::Z),
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    frame(
        &mut app,
        &ctx,
        size,
        vec![key(
            Key::Z,
            Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT,
        )],
    );
    assert_eq!(app.editor.work.as_ref().unwrap().scenery.len(), 6);
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn rectangle_escape_cancels_without_leaving_and_right_drag_temporarily_erases() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.tool = Tool::Rectangle;
    work.metatile = 1;
    work.viewport.center = egui::pos2(0.0, 400.0);
    let size = (800.0, 720.0);
    frame(&mut app, &ctx, size, vec![]);
    let rect = app.editor.canvas_rect.unwrap();
    let viewport = app.editor.work.as_ref().unwrap().viewport;
    let start = viewport.screen(rect, egui::pos2(-40.0, 392.0));
    let end = viewport.screen(rect, egui::pos2(24.0, 408.0));
    frame(
        &mut app,
        &ctx,
        size,
        vec![
            Event::PointerMoved(start),
            pointer(start, PointerButton::Primary, true),
        ],
    );
    frame(&mut app, &ctx, size, vec![Event::PointerMoved(end)]);
    frame(
        &mut app,
        &ctx,
        size,
        vec![key(Key::Escape, Modifiers::NONE)],
    );
    assert!(app.screen == Screen::Editor);
    assert!(app.editor.work.as_ref().unwrap().gesture.is_none());
    assert!(!path.join("scenery-00.json").exists());
    frame(
        &mut app,
        &ctx,
        size,
        vec![pointer(end, PointerButton::Primary, false)],
    );
    let work = app.editor.work.as_mut().unwrap();
    work.begin((-3, 24), false);
    work.extend(Some((-1, 24)));
    work.finish();
    frame(
        &mut app,
        &ctx,
        size,
        vec![
            Event::PointerMoved(start),
            pointer(start, PointerButton::Secondary, true),
        ],
    );
    let end = viewport.screen(rect, egui::pos2(-8.0, 392.0));
    frame(&mut app, &ctx, size, vec![Event::PointerMoved(end)]);
    frame(
        &mut app,
        &ctx,
        size,
        vec![pointer(end, PointerButton::Secondary, false)],
    );
    assert!(app.editor.work.as_ref().unwrap().scenery.is_empty());
    assert_eq!(app.editor.work.as_ref().unwrap().tool, Tool::Rectangle);
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn wheel_and_space_pan_do_not_paint_or_change_gameplay_zoom() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let size = (800.0, 720.0);
    frame(&mut app, &ctx, size, vec![]);
    let rect = app.editor.canvas_rect.unwrap();
    let pos = rect.center() - egui::vec2(80.0, 0.0);
    let view = app.settings.view();
    let window_zoom = app.settings.zoom;
    let center_before = app.editor.work.as_ref().unwrap().viewport.center;
    frame(
        &mut app,
        &ctx,
        size,
        vec![
            Event::PointerMoved(pos),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -64.0),
                modifiers: Modifiers::NONE,
            },
        ],
    );
    let work = app.editor.work.as_ref().unwrap();
    assert_eq!(work.viewport.zoom, 2);
    assert_eq!(work.viewport.center, center_before + egui::vec2(0.0, 32.0));
    frame(
        &mut app,
        &ctx,
        size,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(-64.0, 0.0),
            modifiers: Modifiers::SHIFT,
        }],
    );
    assert_eq!(
        app.editor.work.as_ref().unwrap().viewport.center.x,
        center_before.x + 32.0
    );
    let before = app.editor.work.as_ref().unwrap().viewport.world(rect, pos);
    frame(
        &mut app,
        &ctx,
        size,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            modifiers: Modifiers::CTRL | Modifiers::COMMAND,
        }],
    );
    let work = app.editor.work.as_ref().unwrap();
    assert_eq!(work.viewport.zoom, 3);
    assert_eq!(work.viewport.world(rect, pos), before);
    let center = work.viewport.center;
    frame(
        &mut app,
        &ctx,
        size,
        vec![
            key(Key::Space, Modifiers::NONE),
            pointer(pos, PointerButton::Primary, true),
        ],
    );
    frame(
        &mut app,
        &ctx,
        size,
        vec![Event::PointerMoved(pos + egui::vec2(96.0, 0.0))],
    );
    assert_eq!(
        app.editor.work.as_ref().unwrap().viewport.center,
        center - egui::vec2(32.0, 0.0)
    );
    frame(
        &mut app,
        &ctx,
        size,
        vec![
            pointer(pos, PointerButton::Primary, false),
            key(Key::Plus, Modifiers::NONE),
        ],
    );
    assert!(app.editor.work.as_ref().unwrap().scenery.is_empty());
    assert_eq!(app.settings.view(), view);
    assert_eq!(app.settings.zoom, window_zoom);
    assert_eq!(app.editor.work.as_ref().unwrap().viewport.zoom, 4);
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn playtest_restarts_with_scenery_and_escape_restores_the_workspace() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.viewport.center = egui::pos2(-200.0, 800.0);
    work.viewport.zoom = 4;
    work.metatile = 1;
    work.tool = Tool::Rectangle;
    work.begin((-1, 48), false);
    work.finish();
    let size = (800.0, 720.0);
    frame(&mut app, &ctx, size, vec![key(Key::Enter, Modifiers::NONE)]);
    let output = frame(&mut app, &ctx, size, vec![]);
    assert!(app.screen == Screen::Game);
    assert!(app.editor.playtesting);
    let playtest = app.texture.id();
    assert!(
        output
            .shapes
            .iter()
            .any(|shape| draws_texture(&shape.shape, playtest)),
        "playtesting from the launcher draws the game"
    );
    let sml2_runtime::session::SessionState::InLevel(level) =
        &app.game.as_ref().unwrap().session.state
    else {
        panic!("playtest starts level");
    };
    assert_eq!(level.scenery.get(-1, 48), Some(1));
    frame(
        &mut app,
        &ctx,
        size,
        vec![key(Key::Escape, Modifiers::NONE)],
    );
    assert!(app.screen == Screen::Editor);
    let work = app.editor.work.as_ref().unwrap();
    assert_eq!(work.viewport.center, egui::pos2(-200.0, 800.0));
    assert_eq!(work.viewport.zoom, 4);
    assert_eq!(work.metatile, 1);
    assert_eq!(work.tool, Tool::Rectangle);
    assert!(work.can_undo());
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn shortcut_events_are_consumed_once_even_when_the_frame_needs_another_pass() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 1;
    for x in -2..0 {
        work.begin((x, 0), false);
        work.finish();
    }
    let size = (800.0, 720.0);
    frame(&mut app, &ctx, size, vec![]);
    let mut raw = egui::RawInput {
        focused: true,
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(size.0, size.1))),
        modifiers: Modifiers::CTRL | Modifiers::COMMAND,
        events: vec![key(Key::Z, Modifiers::CTRL | Modifiers::COMMAND)],
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .inner_rect = raw.screen_rect;
    app.receive_input(&raw);
    let _ = ctx.run(raw, |ctx| {
        app.ui(ctx);
        ctx.request_discard("test an egui sizing pass");
    });
    assert_eq!(app.editor.work.as_ref().unwrap().scenery.len(), 1);
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn idle_editor_reuses_chunks_and_skips_hidden_game_uploads() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let size = (800.0, 720.0);
    frame(&mut app, &ctx, size, vec![]);
    frame(&mut app, &ctx, size, vec![]);
    for _ in 0..30 {
        frame(&mut app, &ctx, size, vec![]);
    }
    let builds = app.editor.canvas_cache.builds;
    let game_texture = app.texture.id();
    let idle = frame(&mut app, &ctx, size, vec![]);
    assert_eq!(app.editor.canvas_cache.builds, builds);
    assert!(idle
        .textures_delta
        .set
        .iter()
        .all(|(id, _)| *id != game_texture));
    assert!(
        idle.viewport_output[&egui::ViewportId::ROOT].repaint_delay
            > std::time::Duration::from_millis(8)
    );
    let work = app.editor.work.as_mut().unwrap();
    let cell = (
        (work.viewport.center.x / 16.0) as i16,
        (work.viewport.center.y / 16.0) as i16,
    );
    work.metatile = 1;
    work.begin(cell, false);
    frame(&mut app, &ctx, size, vec![]);
    assert_eq!(app.editor.canvas_cache.builds, builds + 1);
    app.editor.work.as_mut().unwrap().finish();
    app.editor.work.as_mut().unwrap().undo();
    frame(&mut app, &ctx, size, vec![]);
    assert_eq!(app.editor.canvas_cache.builds, builds + 2);
    std::fs::remove_dir_all(path).ok();
}

/// The Select tool through real input: drag an area, Ctrl+C (which desktop
/// backends report as a copy event), Ctrl+V (a paste event), then clicks
/// stamp copies until Escape.
#[test]
fn select_copy_and_paste_repeat_an_area_through_input_events() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 1;
    work.viewport.center = egui::pos2(0.0, 400.0);
    for cell in [(-4, 24), (-3, 25)] {
        work.begin(cell, false);
        work.finish();
    }
    let size = (800.0, 720.0);
    frame(&mut app, &ctx, size, vec![key(Key::S, Modifiers::NONE)]);
    assert_eq!(app.editor.work.as_ref().unwrap().tool, Tool::Select);
    let rect = app.editor.canvas_rect.unwrap();
    let viewport = app.editor.work.as_ref().unwrap().viewport;
    let at = |x: i16, y: i16| {
        viewport.screen(
            rect,
            egui::pos2(f32::from(x) * 16.0 + 8.0, f32::from(y) * 16.0 + 8.0),
        )
    };
    let (start, end) = (at(-4, 24), at(-3, 25));
    frame(
        &mut app,
        &ctx,
        size,
        vec![
            Event::PointerMoved(start),
            pointer(start, PointerButton::Primary, true),
        ],
    );
    frame(&mut app, &ctx, size, vec![Event::PointerMoved(end)]);
    frame(
        &mut app,
        &ctx,
        size,
        vec![pointer(end, PointerButton::Primary, false)],
    );
    assert_eq!(
        app.editor.work.as_ref().unwrap().selection,
        Some(((-4, 24), (-3, 25)))
    );

    let output = frame(&mut app, &ctx, size, vec![Event::Copy]);
    assert!(
        output
            .platform_output
            .commands
            .iter()
            .any(|command| matches!(
                command,
                egui::OutputCommand::CopyText(text) if !text.is_empty()
            )),
        "Copy leaves text so Ctrl+V reaches the editor"
    );
    frame(&mut app, &ctx, size, vec![Event::Paste("anything".into())]);
    assert!(app.editor.work.as_ref().unwrap().pasting);

    for (x, y) in [(-12, 24), (-8, 27)] {
        let p = at(x, y);
        frame(
            &mut app,
            &ctx,
            size,
            vec![
                Event::PointerMoved(p),
                pointer(p, PointerButton::Primary, true),
            ],
        );
        frame(
            &mut app,
            &ctx,
            size,
            vec![pointer(p, PointerButton::Primary, false)],
        );
    }
    let work = app.editor.work.as_ref().unwrap();
    for (x, y) in [(-12, 24), (-8, 27)] {
        assert_eq!(
            (work.paint((x, y)), work.paint((x + 1, y + 1))),
            (Some(1), Some(1))
        );
        assert_eq!(
            work.paint((x + 1, y)),
            None,
            "the copy's empty cell stays empty"
        );
    }
    frame(
        &mut app,
        &ctx,
        size,
        vec![key(Key::Escape, Modifiers::NONE)],
    );
    assert!(
        app.screen == Screen::Editor,
        "Escape stops stamping instead of leaving"
    );
    assert!(!app.editor.work.as_ref().unwrap().pasting);
    std::fs::remove_dir_all(path).ok();
}
