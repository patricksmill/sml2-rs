use super::*;
use crate::dx_import::Worker;
use sml2_dx::cache::CacheKey;
use sml2_runtime::session::SessionState;

fn game_tick(app: &mut DesktopApp, ctx: &Context) {
    app.step = true;
    super::tests::frame(app, ctx, vec![]);
}

#[test]
fn character_door_requires_dx_mode_and_installed_graphics() {
    use sml2_core::front_end::{keys::RIGHT, FileSelect};
    use sml2_runtime::session::SessionState;

    let (mut app, _, path) = super::tests::app();
    for (enabled, installed, expected_timer) in
        [(false, true, 0), (true, false, 0), (true, true, 64)]
    {
        app.settings.dx_mode = enabled;
        let session = &mut app.game.as_mut().unwrap().session;
        let mut file_select = FileSelect::new(&mut session.slots);
        for _ in 0..64 {
            file_select.step(&mut session.slots, 0, 0);
        }
        for _ in 0..3 {
            file_select.step(&mut session.slots, RIGHT, 0);
            for _ in 0..32 {
                file_select.step(&mut session.slots, 0, 0);
            }
        }
        session.state = SessionState::FileSelect(file_select);
        super::dx::prepare_tick(session, &app.settings, installed);
        let SessionState::FileSelect(file_select) = &mut session.state else {
            unreachable!()
        };
        file_select.step(&mut session.slots, RIGHT, 0);
        assert_eq!(file_select.timer(), expected_timer);
        assert_eq!(file_select.cursor(), (0x88, 0x7C));
    }
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn character_door_ticks_render_and_persist_the_new_character_without_a_save() {
    use crate::input::Button;
    use sml2_core::character::Character;
    use sml2_runtime::session::SessionState;

    let (mut app, ctx, path) = super::tests::app();
    let mut bundle = red_bundle();
    // Keep this synthetic cursor visible even while it reaches the offscreen door.
    for character in &mut bundle.players {
        for player in character {
            player.mappings = vec![
                vec![sml2_core::object::MappingEntry {
                    y: 0,
                    x: -80,
                    tile: 0,
                    attr: 0,
                }];
                242
            ];
        }
    }
    bundle.players[1][0].normal.fill(0xFF);
    app.dx_bundle = Some(bundle);
    app.settings.dx_mode = true;
    app.screen = Screen::Game;
    app.game.as_mut().unwrap().session.enter_file_select();
    let slots = app.game.as_ref().unwrap().session.slots.image();
    let menu = |app: &DesktopApp| {
        let SessionState::FileSelect(file_select) = &app.game.as_ref().unwrap().session.state
        else {
            panic!("file select")
        };
        (
            file_select.menu_state(),
            file_select.cursor(),
            file_select.timer(),
        )
    };
    for _ in 0..64 {
        if menu(&app).0 == 1 {
            break;
        }
        game_tick(&mut app, &ctx);
    }
    app.input.event(Button::key("ArrowRight"), true);
    for _ in 0..99 {
        game_tick(&mut app, &ctx);
    }
    assert_eq!(menu(&app), (1, (0x88, 0x7C), 0));
    app.input.event(Button::key("ArrowRight"), false);
    game_tick(&mut app, &ctx);
    app.input.event(Button::key("ArrowRight"), true);
    game_tick(&mut app, &ctx);
    app.input.event(Button::key("ArrowRight"), false);
    for _ in 0..64 {
        game_tick(&mut app, &ctx);
    }
    assert_eq!(app.settings.character, Character::Mario);
    game_tick(&mut app, &ctx);
    assert_eq!(menu(&app), (3, (0xC8, 0x7C), 64));
    assert_eq!(app.settings.character, Character::Luigi);
    assert_eq!(
        app.game.as_ref().unwrap().session.character,
        Character::Luigi
    );
    let luigi = app.frame_image(Smooth::OFF).unwrap();
    app.settings.character = Character::Mario;
    let mario = app.frame_image(Smooth::OFF).unwrap();
    assert_ne!(
        luigi.pixels, mario.pixels,
        "the switched frame uses Luigi graphics"
    );
    app.settings.character = Character::Luigi;
    app.persist();
    assert_eq!(
        Settings::load(&app.options.paths.settings)
            .unwrap()
            .character,
        Character::Luigi
    );
    assert_eq!(app.game.as_ref().unwrap().session.slots.image(), slots);
    assert!(!path.join("save.sav").exists());

    // A later menu choice must seed the session instead of being overwritten.
    app.settings.character = Character::Mario;
    game_tick(&mut app, &ctx);
    assert_eq!(
        app.game.as_ref().unwrap().session.character,
        Character::Mario
    );
    assert_eq!(app.settings.character, Character::Mario);
    let _ = std::fs::remove_dir_all(path);
}

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
    assert_eq!(app.screen, Screen::Game, "browser Enter must not leave the title");
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));
    assert!(app.dx_picker.is_some());
    // Escape closes the browser instead of resuming the game behind it.
    app.input.event(crate::input::Button::key("Escape"), true);
    app.shortcuts(&ctx);
    assert!(app.dx_picker.is_none());
    assert_eq!(app.screen, Screen::Game);
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
        app.menu.selected = 5; // Import, after the tab strip, view rows, DX Mode and Character.
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

/// Every level's BG colors are pure red; every tile is blank.
pub(super) fn red_bundle() -> sml2_dx::Presentation {
    let level = sml2_dx::LevelGraphics {
        background: [vec![0; 0x1000], vec![0; 0x1000]],
        tile_attributes: vec![0; 256],
        bg_palettes: [0x1f, 0].repeat(32),
        obj_palettes: [0xe0, 3].repeat(32),
        hud_attributes: vec![0; 20],
        theme: 0,
        object_table: 0,
        metatiles: sml2_core::level::MetatileTable::from_bytes(&[0; 512]).unwrap(),
        layout: vec![0; 0x3000],
    };
    let player = sml2_dx::PlayerGraphics {
        normal: vec![0; 0x800],
        fire: vec![0; 0x800],
        bank1: vec![0; 0x800],
        mappings: vec![vec![]; 242],
    };
    sml2_dx::Presentation {
        levels: vec![level; 32],
        players: [[player.clone(), player.clone()], [player.clone(), player]],
        object_tables: vec![vec![vec![]; 228]],
        hud_tiles: vec![0; 20],
        animation_rows: vec![0; 0x1000],
        banks: Default::default(),
        title: screen(),
        file_select: screen(),
        map: sml2_dx::MapGraphics {
            tilemaps: Default::default(),
            manifest: Default::default(),
            tiles: Default::default(),
            zone_animation_tiles: Vec::new(),
            hub_attributes: vec![0; 0x100],
            screens: vec![
                sml2_dx::MapScreen {
                    bg_palettes: [0x1f, 0].repeat(32),
                    alternate_bg_palettes: None,
                    obj_palettes: [[0xe0, 3].repeat(32), [0xe0, 3].repeat(32)],
                    attributes: Some(vec![0; 0x240]),
                    alternate_attributes: None,
                };
                9
            ],
            sprites: sml2_dx::MapSprites {
                hub_decor: vec![vec![]; 17],
                zone_decor: vec![vec![]; 188],
                status_bar: [(0, 0, 0, 0); 9],
            },
        },
    }
}

/// A blank screen whose background is palette 0 color 0.
fn screen() -> sml2_dx::Screen {
    sml2_dx::Screen {
        vram: [vec![0; 0x1800], vec![0; 0x1800]],
        tilemap: vec![0; 0x400],
        attributes: vec![0; 0x400],
        bg_palettes: [0x1f, 0].repeat(32),
        obj_palettes: [0xe0, 3].repeat(32),
    }
}

fn is_red(image: &ColorImage) -> bool {
    image
        .pixels
        .iter()
        .all(|p| *p == Color32::from_rgb(255, 0, 0))
}

#[test]
fn dx_mode_frames_follow_the_view_settings() {
    let (mut app, _, path) = super::tests::app();
    app.dx_bundle = Some(red_bundle());
    app.game.as_mut().unwrap().session.enter_title();
    app.settings.dx_mode = true;
    app.settings.widescreen = crate::settings::Widescreen::Wide16x9;
    let image = app
        .frame_image(sml2_runtime::render::motion::Smooth::OFF)
        .unwrap();
    assert_eq!(image.size, [256, 144]);
    // The title backdrop fills the view with the screen's own color.
    assert_eq!(image.pixels[72 * 256 + 128], Color32::from_rgb(255, 0, 0));
    assert_eq!(image.pixels[72 * 256], Color32::from_rgb(255, 0, 0));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn dx_mode_colors_every_level_of_the_normal_game() {
    let (mut app, _, path) = super::tests::app();
    app.dx_bundle = Some(red_bundle());
    // The title has its own DX screen.
    app.game.as_mut().unwrap().session.enter_title();
    app.settings.dx_mode = true;
    assert!(is_red(
        &app.frame_image(sml2_runtime::render::motion::Smooth::OFF)
            .unwrap()
    ));
    for level in [0, 1, 0x1F] {
        let game = app.game.as_mut().unwrap();
        game.session.debug_warp_level(level).unwrap();
        app.settings.dx_mode = false;
        assert!(
            !is_red(
                &app.frame_image(sml2_runtime::render::motion::Smooth::OFF)
                    .unwrap()
            ),
            "off: the filter, level {level}"
        );
        app.settings.dx_mode = true;
        assert!(
            is_red(
                &app.frame_image(sml2_runtime::render::motion::Smooth::OFF)
                    .unwrap()
            ),
            "level {level}"
        );
    }
    // Luigi draws through the same session.
    app.settings.character = sml2_core::character::Character::Luigi;
    assert!(is_red(
        &app.frame_image(sml2_runtime::render::motion::Smooth::OFF)
            .unwrap()
    ));
    // A boss arena, and any DX failure, keep the native picture.
    let game = app.game.as_mut().unwrap();
    if let sml2_runtime::session::SessionState::InLevel(ctx) = &mut game.session.state {
        ctx.arena_loaded = true;
    }
    assert!(!is_red(
        &app.frame_image(sml2_runtime::render::motion::Smooth::OFF)
            .unwrap()
    ));
    assert!(app.dx_message.is_empty(), "an arena is not a failure");
    let game = app.game.as_mut().unwrap();
    game.session.debug_warp_level(0).unwrap();
    app.dx_bundle.as_mut().unwrap().levels[0]
        .bg_palettes
        .clear();
    assert!(!is_red(
        &app.frame_image(sml2_runtime::render::motion::Smooth::OFF)
            .unwrap()
    ));
    assert!(!app.dx_message.is_empty());
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
    app.menu.selected = 4; // DX Mode, after the tab strip and the view rows.
    app.input.event(crate::input::Button::key("Enter"), true);
    super::tests::frame(&mut app, &ctx, vec![]);
    assert!(app.settings.dx_mode);
    assert!(app.dx_worker.is_some(), "the ROM chooser opens");
    // The test chooser returns no file, so the in-app browser takes over.
    wait_for_dx(&mut app);
    assert!(app.dx_picker.is_some());
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn dx_mode_colors_the_settings_hall() {
    use sml2_core::hardware_palettes::rgb555;
    let (mut app, _, path) = super::tests::app();
    let mut dx = red_bundle();
    dx.file_select.bg_palettes[6] = 0;
    dx.file_select.bg_palettes[7] = 0x7c; // dark sign ink is blue
    dx.file_select.bg_palettes[8] = 0xe0;
    dx.file_select.bg_palettes[9] = 0x03;
    dx.file_select.attributes[0] = 1;
    dx.file_select.attributes[1] = 1;
    dx.file_select.attributes[32] = 1;
    dx.file_select.attributes[33] = 1;
    app.dx_bundle = Some(dx);
    app.settings.dx_mode = true;
    app.settings.widescreen = crate::settings::Widescreen::Off;
    app.settings.camera_zoom = crate::settings::CameraZoom::Native;
    let mut hall = hall::Hall::enter();
    hall.mario = hall::pedestal_x(1);
    hall.camera = -72;
    app.hall = Some(hall);
    let image = app.frame_image(Smooth::OFF).unwrap();
    assert_eq!(
        image.pixels[0],
        Color32::from_rgb(rgb555(0x03e0)[0], rgb555(0x03e0)[1], rgb555(0x03e0)[2])
    );
    assert_eq!(
        image.pixels[32 * 160],
        Color32::from_rgb(rgb555(0x001f)[0], rgb555(0x001f)[1], rgb555(0x001f)[2])
    );
    assert!(
        image.pixels[48 * 160..72 * 160]
            .iter()
            .any(|&pixel| pixel == Color32::BLUE),
        "the option sign uses DX background color 3"
    );
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn resting_on_the_hall_door_keeps_the_live_file_select_picture() {
    let (mut app, _, path) = super::tests::app();
    app.dx_bundle = Some(red_bundle());
    app.game.as_mut().unwrap().session.enter_file_select();
    for _ in 0..64 {
        app.game
            .as_mut()
            .unwrap()
            .session
            .advance(Default::default());
    }
    for dx_mode in [false, true] {
        app.settings.dx_mode = dx_mode;
        app.hall = None;
        let file_select = app.frame_image(Smooth::OFF).unwrap();
        app.hall = Some(hall::Hall::enter());
        let doorway = app.frame_image(Smooth::OFF).unwrap();
        assert_eq!(doorway.size, file_select.size);
        assert!(doorway.pixels == file_select.pixels, "DX mode {dx_mode}");
    }
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn file_select_and_hall_keep_the_same_native_framing_in_enlarged_views() {
    use crate::settings::{CameraZoom, Widescreen};
    let (mut app, _, path) = super::tests::app();
    app.dx_bundle = Some(red_bundle());
    app.game.as_mut().unwrap().session.enter_file_select();
    for _ in 0..64 {
        app.game
            .as_mut()
            .unwrap()
            .session
            .advance(Default::default());
    }
    for dx_mode in [false, true] {
        app.settings.dx_mode = dx_mode;
        app.settings.widescreen = Widescreen::Off;
        app.settings.camera_zoom = CameraZoom::Native;
        app.hall = None;
        let native = app.frame_image(Smooth::OFF).unwrap();
        app.hall = Some(hall::Hall::enter());
        hall::step(
            app.hall.as_mut().unwrap(),
            true,
            false,
            false,
            &mut app.settings,
            &mut app.debug,
        );
        let hall_native = app.frame_image(Smooth::OFF).unwrap();
        for (wide, zoom) in [
            (Widescreen::Wide16x9, CameraZoom::Native),
            (Widescreen::Off, CameraZoom::Zoom125),
            (Widescreen::Wide16x9, CameraZoom::Zoom150),
            (Widescreen::Wide16x9, CameraZoom::Zoom200),
        ] {
            app.settings.widescreen = wide;
            app.settings.camera_zoom = zoom;
            let view = app.settings.view();
            let expected = sml2_runtime::render::view::letterbox(
                &hall_native.pixels,
                (160, 144),
                view,
                Color32::BLACK,
            );
            assert!(
                app.frame_image(Smooth::OFF).unwrap().pixels == expected,
                "hall {view:?}"
            );
            let hall = app.hall.take();
            let expected = sml2_runtime::render::view::letterbox(
                &native.pixels,
                (160, 144),
                view,
                Color32::BLACK,
            );
            assert!(
                app.frame_image(Smooth::OFF).unwrap().pixels == expected,
                "file select {view:?}, DX {dx_mode}"
            );
            app.hall = Some(hall::Hall::enter());
            assert!(
                app.frame_image(Smooth::OFF).unwrap().pixels == expected,
                "doorway {view:?}"
            );
            app.hall = hall;
        }
    }
    let _ = std::fs::remove_dir_all(path);
}
