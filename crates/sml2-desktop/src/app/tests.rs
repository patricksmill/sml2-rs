use super::debug::DebugAction;
use super::*;
use crate::{
    input::{Button, PadButton},
    settings::AppPaths,
};
use sml2_runtime::session::SessionState;
use std::sync::atomic::{AtomicUsize, Ordering};

fn app() -> (DesktopApp, Context, PathBuf) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sml2_desktop_ui_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let options = Options {
        assets: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"),
        paths: AppPaths {
            settings: directory.join("settings.json"),
            save: directory.join("save.sav"),
        },
        audio: false,
        filter: None,
        play: false,
        boot_title: false,
        boot_select: false,
        room: None,
        reveal: false,
        scale: None,
        fullscreen: false,
        smoke_frames: None,
        screenshot: None,
    };
    let ctx = Context::default();
    let mut app = DesktopApp::new(&ctx, options, Settings::default(), None);
    app.controllers = Controllers::disconnected();
    (app, ctx, directory)
}

fn frame(app: &mut DesktopApp, ctx: &Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut raw = RawInput {
        focused: true,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 720.0),
        )),
        events,
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .inner_rect = raw.screen_rect;
    app.receive_input(&raw);
    app.last_update = Instant::now() - Duration::from_millis(17);
    ctx.run(raw, |ctx| app.ui(ctx))
}

fn key(key: egui::Key, down: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: down,
        repeat: false,
        modifiers: Default::default(),
    }
}

#[test]
fn controller_can_open_settings_and_confirm_play_without_leaking_a_jump() {
    let (mut app, ctx, path) = app();
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), false);
    // Row 1 is Title / File Select; Settings is row 2.
    app.input.event(Button::Pad(PadButton::Down), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), false);
    app.input.event(Button::Pad(PadButton::South), true);
    frame(&mut app, &ctx, vec![]);
    assert!(app.screen == Screen::Settings);
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 0);
    app.input.event(Button::Pad(PadButton::South), false);
    app.input.event(Button::Pad(PadButton::Start), true);
    frame(&mut app, &ctx, vec![]);
    assert!(app.screen == Screen::Home);
    app.input.event(Button::Pad(PadButton::Start), false);
    app.input.event(Button::Pad(PadButton::South), true);
    frame(&mut app, &ctx, vec![]);
    assert!(app.screen == Screen::Game);
    let game = app.game.as_ref().unwrap();
    assert!(
        game.session.pending_level.is_none(),
        "menu confirmation must not enter a level"
    );
    assert!(game.session.frame_n > 0);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn debug_pause_and_step_advance_exactly_one_tick_after_a_stall() {
    let (mut app, ctx, path) = app();
    app.screen = Screen::Game;
    app.paused = true;
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 0);
    frame(&mut app, &ctx, vec![key(egui::Key::F3, true)]);
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 1);
    frame(&mut app, &ctx, vec![key(egui::Key::F3, false)]);
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 1);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn fullscreen_shortcut_requests_a_native_transition_without_ticking_the_menu() {
    let (mut app, ctx, path) = app();
    frame(&mut app, &ctx, vec![]);
    let output = frame(&mut app, &ctx, vec![key(egui::Key::F11, true)]);
    assert!(app.settings.fullscreen);
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .any(|c| matches!(c, ViewportCommand::Fullscreen(true))));
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 0);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn binding_capture_updates_controls_without_forwarding_the_capture_press() {
    let (mut app, ctx, path) = app();
    frame(&mut app, &ctx, vec![]);
    app.screen = Screen::Settings;
    app.capture = Some(Action::Jump);
    frame(&mut app, &ctx, vec![key(egui::Key::K, true)]);
    assert!(app.capture.is_none());
    assert!(app.settings.bindings.0[Action::Jump as usize].contains(&Button::key("K")));
    assert!(!app.input.sample_player(&app.settings.bindings).a);
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 0);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn focus_loss_freezes_ticks_and_clears_held_keys() {
    let (mut app, ctx, path) = app();
    frame(&mut app, &ctx, vec![]);
    app.screen = Screen::Game;
    frame(&mut app, &ctx, vec![]);
    let tick = app.game.as_ref().unwrap().session.frame_n;
    let mut raw = RawInput {
        focused: false,
        events: vec![key(egui::Key::Z, true)],
        ..Default::default()
    };
    app.receive_input(&raw);
    let _ = ctx.run(raw.take(), |ctx| app.ui(ctx));
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, tick);
    assert!(!app.input.sample_player(&app.settings.bindings).a);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn debug_panel_renders_with_a_game_and_lists_every_level() {
    let (mut app, ctx, path) = app();
    app.debug = true;
    app.screen = Screen::Game;
    // Two frames: the second reuses egui's memory, which is where duplicate
    // widget ids would panic.
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.levels.len(), 32, "every extracted level is offered");
    assert!(app
        .levels
        .iter()
        .any(|(id, label)| *id == 0 && label.contains("level1")));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn debug_actions_edit_the_session_and_persist_the_active_pipe() {
    let (mut app, ctx, path) = app();
    app.debug = true;
    app.screen = Screen::Game;
    frame(&mut app, &ctx, vec![]);
    app.game.as_mut().unwrap().session.active_slot = Some(0);

    app.apply_debug(DebugAction::ClearAllLevels);
    let game = app.game.as_ref().unwrap();
    assert!(game.session.debug_golden_coins().iter().all(|&held| held));
    assert!(
        game.session.slots.load_slot(0).is_none_or(|record| record[0x08 + 5] & 0x80 == 0),
        "an edit alone does not touch the pipe"
    );
    assert!(!path.join("save.sav").exists(), "nor the slot image on disk");

    app.apply_debug(DebugAction::Save);
    let game = app.game.as_ref().unwrap();
    let record = game
        .session
        .slots
        .load_slot(0)
        .expect("pipe A holds the record");
    assert_eq!(record[0x08 + 5] & 0x80, 0x80, "the clear reached the save");
    assert!(sml2_core::map::save_record_valid(&record));
    assert!(path.join("save.sav").exists(), "the slot image was written");

    app.apply_debug(DebugAction::SetGoldenCoin {
        index: 0,
        held: false,
    });
    assert_eq!(
        app.game.as_ref().unwrap().session.debug_golden_coins(),
        [false, true, true, true, true, true]
    );

    app.apply_debug(DebugAction::AddCoins(250));
    app.apply_debug(DebugAction::AddLives(2));
    assert_eq!(app.game.as_ref().unwrap().session.debug_coins(), 250);
    assert_eq!(app.game.as_ref().unwrap().session.debug_lives(), 7);

    app.apply_debug(DebugAction::WarpLevel(4));
    match &app.game.as_ref().unwrap().session.state {
        SessionState::InLevel(level) => assert_eq!(level.manifest.level_id, 4),
        _ => panic!("the level warp did not go live"),
    }
    // A warped level has to render and tick like a played one.
    let before = app.game.as_ref().unwrap().session.frame_n;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    assert!(app.game.as_ref().unwrap().session.frame_n > before);

    app.apply_debug(DebugAction::WarpMap {
        screen: 3,
        node: 0x32,
    });
    match &app.game.as_ref().unwrap().session.state {
        SessionState::Overworld(map) => {
            assert_eq!((map.screen_id, map.node), (3, 0x32));
            assert_eq!(
                map.level_progress[5] & 0x80,
                0,
                "the removed coin stayed removed"
            );
        }
        _ => panic!("the map warp did not land"),
    }

    // A zone button walks in through the hub door instead of teleporting.
    app.apply_debug(DebugAction::EnterZone(2));
    let session = &app.game.as_ref().unwrap().session;
    let transition = session
        .map_screen_transition
        .as_ref()
        .expect("the door's walk-in runs");
    assert_eq!(transition.source.screen_id, 0, "it starts on the hub door");
    match &session.state {
        SessionState::Overworld(map) => assert_eq!((map.screen_id, map.node), (2, 0x23)),
        _ => panic!("the zone entry left the map"),
    }
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn output_zoom_uses_physical_pixels_at_high_ui_dpi() {
    let (mut app, ctx, path) = app();
    app.screen = Screen::Game;
    app.paused = true;
    app.settings.zoom = Zoom::Integer(2);
    let mut raw = RawInput {
        focused: true,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 720.0),
        )),
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .native_pixels_per_point = Some(2.0);
    let _ = ctx.run(raw, |ctx| app.ui(ctx));
    assert_eq!(ctx.pixels_per_point(), 2.0);
    assert_eq!(&app.game_rect[2..], &[320, 288]);
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 0);
    let _ = std::fs::remove_dir_all(path);
}
