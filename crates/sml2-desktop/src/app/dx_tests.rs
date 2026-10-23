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
        app.menu.selected = 3; // Import, after the tab strip, DX Mode and Character.
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

/// A valid bundle whose every BG color is pure red.
fn red_bundle() -> sml2_dx::Bundle {
    sml2_dx::Bundle {
        schema: sml2_dx::BUNDLE_SCHEMA,
        source_sha256: sml2_dx::ROM_SHA256.into(),
        background: [vec![0; 0x1000], vec![0; 0x1000]],
        mario_normal: vec![0; 0x800],
        mario_fire: vec![0; 0x800],
        mario_bank1: vec![0; 0x800],
        tile_attributes: vec![0; 256],
        hud_tiles: vec![0; 20],
        hud_attributes: vec![0; 20],
        bg_palettes: [0x1f, 0].repeat(32),
        obj_palettes: [0xe0, 3].repeat(32),
        animation: vec![0; 256],
        object_mappings: vec![vec![]; 168],
        player_mappings: vec![vec![]; 242],
        luigi: Some(sml2_dx::CharacterGraphics {
            normal: vec![0; 0x800],
            fire: vec![0; 0x800],
            bank1: vec![0; 0x800],
            mappings: vec![vec![]; 242],
        }),
    }
}

fn is_red(image: &ColorImage) -> bool {
    image
        .pixels
        .iter()
        .all(|p| *p == Color32::from_rgb(255, 0, 0))
}

#[test]
fn dx_mode_colors_the_normal_game_only_where_dx_data_covers_it() {
    let (mut app, _, path) = super::tests::app();
    app.dx_bundle = Some(red_bundle());
    let game = app.game.as_mut().unwrap();
    game.session.debug_warp_level(0).unwrap();
    // Off: the filter's native frame, even in the covered level.
    assert!(!is_red(&app.frame_image().unwrap()));
    app.settings.dx_mode = true;
    assert!(is_red(&app.frame_image().unwrap()));
    // Luigi uses the same session; a Mario-only bundle quietly draws Mario.
    app.settings.character = sml2_core::character::Character::Luigi;
    assert!(is_red(&app.frame_image().unwrap()));
    let bundle = app.dx_bundle.as_mut().unwrap();
    bundle.schema = 1;
    bundle.luigi = None;
    assert!(is_red(&app.frame_image().unwrap()));
    // Another level keeps the native picture, and so does a DX failure.
    let game = app.game.as_mut().unwrap();
    game.session.debug_warp_level(1).unwrap();
    assert!(!is_red(&app.frame_image().unwrap()));
    let game = app.game.as_mut().unwrap();
    game.session.debug_warp_level(0).unwrap();
    app.dx_bundle.as_mut().unwrap().bg_palettes.clear();
    assert!(!is_red(&app.frame_image().unwrap()));
    assert!(app.dx_message.contains("BG palettes"));
    // The normal game keeps its save slot: DX Mode is presentation only.
    assert!(!path.join("save.sav").exists());
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn enabling_dx_mode_without_graphics_starts_the_import() {
    let (mut app, ctx, path) = super::tests::app();
    app.screen = Screen::Settings;
    app.page = super::panels::Page::Mods;
    super::tests::frame(&mut app, &ctx, vec![]);
    app.menu.selected = 1; // DX Mode, after the tab strip.
    app.input.event(crate::input::Button::key("Enter"), true);
    super::tests::frame(&mut app, &ctx, vec![]);
    assert!(app.settings.dx_mode);
    assert!(app.dx_worker.is_some(), "the ROM chooser opens");
    // The test chooser returns no file, so the in-app browser takes over.
    wait_for_dx(&mut app);
    assert!(app.dx_picker.is_some());
    let _ = std::fs::remove_dir_all(path);
}
