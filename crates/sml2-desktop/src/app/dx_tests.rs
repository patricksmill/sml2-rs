use super::*;
use crate::dx_import::Worker;
use sml2_dx::cache::CacheKey;

fn wait_for_dx(app: &mut DesktopApp) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while app.dx_worker.is_some() {
        assert!(Instant::now() < deadline, "DX worker did not finish");
        app.poll_dx();
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn system_picker_without_selection_opens_browser_and_preserves_installation() {
    let (mut app, ctx, path) = super::tests::app();
    let key = CacheKey::new("1".repeat(64));
    app.settings.dx_installation = Some(key.clone());
    app.dx_worker = Some(Worker::choose_file(|| None));
    wait_for_dx(&mut app);
    assert!(app.dx_picker.is_some());
    assert_eq!(app.settings.dx_installation, Some(key.clone()));
    assert!(!app.dx_message.contains("cancelled"));
    let output = super::tests::frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    assert!(!output.shapes.is_empty());
    assert!(
        app.screen == Screen::Home,
        "browser Enter must not activate Play behind it"
    );
    assert!(app.dx_picker.is_some());
    // Escape closes the browser instead of resuming the game behind it.
    app.input.event(crate::input::Button::key("Escape"), true);
    app.shortcuts(&ctx);
    assert!(app.dx_picker.is_none());
    assert!(app.screen == Screen::Home);
    assert_eq!(app.settings.dx_installation, Some(key));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn cancelling_system_picker_job_does_not_open_another_dialog() {
    let (mut app, _, path) = super::tests::app();
    let worker = Worker::choose_file(|| None);
    worker.cancel();
    app.dx_worker = Some(worker);
    wait_for_dx(&mut app);
    assert!(app.dx_picker.is_none());
    assert!(app.dx_message.contains("cancelled"));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn browser_selection_uses_existing_rom_validation_and_preserves_installation() {
    let (mut app, _, path) = super::tests::app();
    std::fs::create_dir_all(&path).unwrap();
    let rom = path.join("unsupported.gbc");
    std::fs::write(&rom, [0; 16]).unwrap();
    let key = CacheKey::new("1".repeat(64));
    app.settings.dx_installation = Some(key.clone());
    app.dx_picker = Some(super::file_picker::FilePicker::new(path.clone()));
    app.dx_worker = Some(Worker::choose_file(move || Some(rom)));
    wait_for_dx(&mut app);
    assert!(app.dx_picker.is_none());
    assert!(app.dx_message.contains("Unsupported DX ROM"));
    assert_eq!(app.settings.dx_installation, Some(key));
    assert!(!path.join("save.sav").exists());
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn selecting_rom_does_not_activate_quit_from_browser_keyboard_input() {
    for key in [egui::Key::Enter, egui::Key::Space] {
        let (mut app, ctx, path) = super::tests::app();
        std::fs::create_dir_all(&path).unwrap();
        let rom = path.join("unsupported.gbc");
        std::fs::write(&rom, [0; 16]).unwrap();
        app.screen = Screen::Settings;
        app.page = super::panels::Page::Mods;
        super::tests::frame(&mut app, &ctx, vec![]);
        app.menu.selected = 1; // Import SML2DX, after the tab strip.
        app.dx_picker = Some(super::file_picker::FilePicker::new(path.clone()));
        super::tests::frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
        app.import_dx_path(rom);
        let output = super::tests::frame(&mut app, &ctx, vec![]);
        assert!(
            output.viewport_output.values().all(|viewport| !viewport
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))),
            "browser key activated Quit after selection"
        );
        assert!(app.screen == Screen::Settings);
        wait_for_dx(&mut app);
        let _ = std::fs::remove_dir_all(path);
    }
}

#[test]
fn cancelled_and_failed_replacements_preserve_selected_installation() {
    let (mut app, _, path) = super::tests::app();
    let key = CacheKey::new("1".repeat(64));
    app.settings.dx_installation = Some(key.clone());
    app.dx_worker = Some(Worker::start(|_, _| Err("unsupported ROM".into())));
    for _ in 0..100 {
        app.poll_dx();
        if app.dx_worker.is_none() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(app.settings.dx_installation, Some(key.clone()));
    assert_eq!(app.dx_message, "unsupported ROM");
    app.dx_worker = Some(Worker::start(|_, _| Err("cancelled".into())));
    app.dx_worker.as_ref().unwrap().cancel();
    for _ in 0..100 {
        app.poll_dx();
        if app.dx_worker.is_none() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(app.settings.dx_installation, Some(key));
    assert!(app.dx_message.contains("previous installation retained"));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn leaving_preview_restores_normal_game_and_blocks_debug_warps() {
    let (mut app, ctx, path) = super::tests::app();
    let mut normal = app.game.take().unwrap();
    normal.session.coins_low = 0x42;
    app.dx_normal = Some(normal);
    app.game = Some(Game::preview(SessionAssets::load().unwrap(), false).unwrap());
    app.screen = Screen::Game;
    app.apply_debug(super::debug::DebugAction::WarpLevel(1));
    assert!(app.debug_note_error);
    assert!(matches!(&app.game.as_ref().unwrap().session.state,
        sml2_runtime::session::SessionState::InLevel(ctx) if ctx.manifest.level_id == 0));
    app.navigate(&ctx, Screen::Home);
    assert!(app.dx_normal.is_none());
    assert_eq!(app.game.as_ref().unwrap().session.coins_low, 0x42);
    assert!(!path.join("save.sav").exists());
    let _ = std::fs::remove_dir_all(path);
}
