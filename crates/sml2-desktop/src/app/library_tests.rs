use super::*;
use sml2_runtime::session::SessionState;

fn visible_text(output: egui::FullOutput) -> String {
    fn collect(shape: &egui::Shape, text: &mut String) {
        match shape {
            egui::Shape::Text(shape) => {
                text.push_str(shape.galley.text());
                text.push('\n');
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
            _ => {}
        }
    }
    let mut text = String::new();
    for shape in output.shapes {
        collect(&shape.shape, &mut text);
    }
    text
}

fn launcher() -> (DesktopApp, egui::Context, std::path::PathBuf) {
    let (mut app, ctx, path) = super::tests::app();
    app.game = None;
    app.screen = Screen::Home;
    app.options.launch_library = true;
    app.load_launcher();
    super::tests::frame(&mut app, &ctx, vec![]);
    (app, ctx, path)
}

fn confirm(app: &mut DesktopApp, ctx: &egui::Context, row: usize) -> egui::FullOutput {
    app.menu.selected = row;
    app.input.event(Button::key("Enter"), true);
    let output = super::tests::frame(app, ctx, vec![]);
    app.input.event(Button::key("Enter"), false);
    super::tests::frame(app, ctx, vec![]);
    output
}

#[test]
fn title_screen_action_bar_play_starts_mario_settings_opens_menu_and_quit_closes() {
    let (mut app, ctx, path) = super::tests::app();
    assert_eq!(app.screen, Screen::Game);
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));

    super::tests::frame(&mut app, &ctx, vec![]);
    let output = super::tests::frame(&mut app, &ctx, vec![]);
    let text = visible_text(output);
    assert!(text.contains("Play"));
    assert!(text.contains("Settings"));
    assert!(text.contains("Quit"));

    // Navigating to Settings and hitting Enter opens Settings without advancing Mario.
    app.menu.selected = 1;
    app.input.event(Button::key("Enter"), true);
    super::tests::frame(&mut app, &ctx, vec![]);
    app.input.event(Button::key("Enter"), false);
    super::tests::frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Settings);
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));

    // Return to title screen.
    app.navigate(&ctx, Screen::Game);
    super::tests::frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Game);
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));

    // Navigating to Quit and hitting Enter sends Close command.
    app.menu.selected = 2;
    app.input.event(Button::key("Enter"), true);
    let output = super::tests::frame(&mut app, &ctx, vec![]);
    app.input.event(Button::key("Enter"), false);
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .any(|command| matches!(command, ViewportCommand::Close)));

    // Advance past the initial opening hold (64 frames) where the ROM title ignores Start.
    if let SessionState::Title(title) = &mut app.game.as_mut().unwrap().session.state {
        while !super::pointer::title_accepts_start(title.timer()) {
            title.step(0, 0);
        }
    }

    // Hitting Play proceeds Mario toward file select.
    app.menu.selected = 0;
    app.input.event(Button::key("Enter"), true);
    super::tests::frame(&mut app, &ctx, vec![]);
    app.input.event(Button::key("Enter"), false);
    super::tests::frame(&mut app, &ctx, vec![]);
    assert!(
        !matches!(
            app.game.as_ref().unwrap().session.state,
            SessionState::Title(_)
        ),
        "Mario must proceed to file select loading after Play is hit"
    );

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn launcher_shows_the_title_and_bottom_actions_before_and_after_rom_verification() {
    let (mut app, ctx, path) = launcher();
    for ready in [false, true] {
        app.library.vanilla_ready = ready;
        app.library.dx_installed = ready;
        app.settings.dx_mode = ready;
        let text = visible_text(super::tests::frame(&mut app, &ctx, vec![]));
        assert_eq!(text, "Play\nSettings\nQuit\n");
    }
    assert!(app.launcher.is_some());
    assert!(ctx
        .memory(|memory| memory.area_rect(egui::Id::new("pause-menu")))
        .is_none());
    let output = confirm(&mut app, &ctx, 2);
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .any(|command| matches!(command, ViewportCommand::Close)));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn settings_opens_usual_options_and_escape_returns_to_the_launch_menu() {
    let (mut app, ctx, path) = launcher();
    confirm(&mut app, &ctx, 1);
    assert_eq!(app.screen, Screen::Settings);
    assert_eq!(app.page, Page::Display);
    let text = visible_text(super::tests::frame(&mut app, &ctx, vec![]));
    assert!(text.contains("Fit to window"));
    assert!(text.contains("Fullscreen"));
    assert!(text.contains("GAME"));
    app.input.event(Button::key("Escape"), true);
    let output = super::tests::frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Home);
    assert!(!output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .any(|command| matches!(command, ViewportCommand::Close)));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn play_without_a_usable_rom_opens_settings_for_import_or_repair() {
    for corrupted in [false, true] {
        let (mut app, ctx, path) = launcher();
        if corrupted {
            app.settings.vanilla = Some(VanillaInstall::default());
            app.library.rom_error = "The ROM installation is damaged.".into();
        }
        confirm(&mut app, &ctx, 0);
        assert_eq!(app.screen, Screen::Settings);
        assert_eq!(app.page, Page::Game);
        assert!(app.game.is_none());
        let text = visible_text(super::tests::frame(&mut app, &ctx, vec![]));
        assert!(text.contains(if corrupted {
            "Replace ROM"
        } else {
            "Import ROM"
        }));
        if corrupted {
            assert!(text.contains(&app.library.rom_error));
        }
        assert!(text.contains("Add DX ROM / IPS"));
        app.start_library_import(LibraryPick::Dx);
        assert!(app.dx_worker.is_none(), "DX requires a verified base ROM");
        confirm(&mut app, &ctx, 1); // Original ROM, after tabs.
        assert!(matches!(app.library_pick, Some(LibraryPick::Rom)));
        assert!(app.dx_worker.is_some() || app.dx_picker.is_some());
        if let Some(worker) = &app.dx_worker {
            worker.cancel();
        }
        let _ = std::fs::remove_dir_all(path);
    }
}

#[test]
fn escape_exits_the_launcher_after_a_game_has_loaded() {
    let (mut app, ctx, path) = super::tests::app();
    app.options.launch_library = true;
    app.screen = Screen::Home;
    super::tests::frame(&mut app, &ctx, vec![]);
    app.input.event(Button::key("Escape"), true);
    let output = super::tests::frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Home);
    assert!(output.viewport_output[&egui::ViewportId::ROOT]
        .commands
        .iter()
        .any(|command| matches!(command, ViewportCommand::Close)));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn settings_game_has_dx_import_and_persists_the_mode_choice() {
    let (mut app, ctx, path) = launcher();
    app.library.vanilla_ready = true;
    app.library.revision_name = "Super Mario Land 2".into();
    app.settings.vanilla = Some(VanillaInstall::default());
    app.settings.dx_installation = Some(sml2_dx::cache::CacheKey::new("1".repeat(64)));
    app.library.dx_installed = true;
    app.navigate(&ctx, Screen::Settings);
    app.page = Page::Game;
    super::tests::frame(&mut app, &ctx, vec![]);
    let text = visible_text(super::tests::frame(&mut app, &ctx, vec![]));
    assert!(text.contains("Replace ROM"));
    assert!(text.contains("DX Mode"));
    assert!(text.contains("Replace DX ROM / IPS"));
    confirm(&mut app, &ctx, 2); // DX Mode, after tabs and ROM.
    assert!(app.settings.dx_mode);
    assert!(app.library.dx_mode);
    app.persist();
    assert!(Settings::load(&app.options.paths.settings).unwrap().dx_mode);
    confirm(&mut app, &ctx, 3); // DX importer.
    assert!(matches!(app.library_pick, Some(LibraryPick::Dx)));
    assert!(app.dx_worker.is_some() || app.dx_picker.is_some());
    let text = visible_text(super::tests::frame(&mut app, &ctx, vec![]));
    assert!(text.contains("Cancel import") || app.dx_picker.is_some());
    if let Some(worker) = &app.dx_worker {
        worker.cancel();
    }
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn display_replaces_scaling_explanations_with_a_saved_integer_scaling_toggle() {
    let (mut app, ctx, path) = launcher();
    confirm(&mut app, &ctx, 1);
    let text = visible_text(super::tests::frame(&mut app, &ctx, vec![]));
    assert!(text.contains("Integer scaling"));
    assert!(!text.contains("Fit scales the same game view"));
    assert!(!text.contains("Whole-number scaling keeps pixels crisp"));
    assert!(app.settings.integer_scaling);
    confirm(&mut app, &ctx, 2); // Integer scaling, after tabs and Fit to window.
    assert!(!app.settings.integer_scaling);
    assert_eq!(app.settings.zoom, Zoom::Auto);
    app.persist();
    assert!(
        !Settings::load(&app.options.paths.settings)
            .unwrap()
            .integer_scaling
    );
    let _ = std::fs::remove_dir_all(path);
}
