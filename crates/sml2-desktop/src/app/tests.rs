use super::debug::DebugAction;
use super::*;
use crate::{
    input::{Button, PadButton},
    settings::AppPaths,
};
use sml2_runtime::session::SessionState;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) fn app() -> (DesktopApp, Context, PathBuf) {
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

pub(super) fn frame(
    app: &mut DesktopApp,
    ctx: &Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
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
        game.session
            .slots
            .load_slot(0)
            .is_none_or(|record| record[0x08 + 5] & 0x80 == 0),
        "an edit alone does not touch the pipe"
    );
    assert!(
        !path.join("save.sav").exists(),
        "nor the slot image on disk"
    );

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

    app.apply_debug(DebugAction::SetPowerup(3));
    assert_eq!(app.game.as_ref().unwrap().session.debug_powerup(), 3);

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

#[test]
fn every_filter_colorizes_the_native_source_and_shade_frame() {
    use sml2_core::{
        hardware_palettes::{gbc_auto, rgb555, sgb_preset},
        video::PaletteSource,
    };
    let (app, _, path) = app();
    let header = &app.game.as_ref().unwrap().session.assets.cartridge_header;
    let sources = [PaletteSource::Bg, PaletteSource::Obj0, PaletteSource::Obj1];
    let native: Vec<_> = (0..160 * 144)
        .map(|i| PalettePixel {
            source: sources[i % 3],
            shade: ((i / 3) % 4) as u8,
        })
        .collect();
    for filter in Filter::ALL {
        let image = colorized(&native, filter, header, (160, 144));
        assert_eq!(image.size, [160, 144]);
        let hardware = match filter {
            Filter::Gbc => Some(gbc_auto(header)),
            Filter::Sgb(index) => Some(sgb_preset(index as usize).unwrap()),
            _ => None,
        };
        if let Some(hardware) = hardware {
            for (pixel, actual) in native.iter().zip(&image.pixels) {
                let ramp = match pixel.source {
                    PaletteSource::Bg => hardware.bg,
                    PaletteSource::Obj0 => hardware.obj0,
                    PaletteSource::Obj1 => hardware.obj1,
                };
                let [r, g, b] = rgb555(ramp[pixel.shade as usize]);
                assert_eq!(
                    *actual,
                    Color32::from_rgb(r, g, b),
                    "{} {:?}",
                    filter.label(),
                    pixel
                );
            }
        }
        // Verify the actual expanded native image, including the OBJ sources.
        let expanded = ColorImage::new(
            [320, 288],
            (0..288)
                .flat_map(|y| {
                    let image = &image;
                    (0..320).map(move |x| image.pixels[(y / 2) * 160 + x / 2])
                })
                .collect(),
        );
        assert!(screenshot::verify(&expanded, &image, [0, 0, 320, 288]).is_ok());
    }
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn the_frame_follows_the_view_settings() {
    let (mut app, _, path) = app();
    app.settings.widescreen = crate::settings::Widescreen::Wide16x9;
    app.settings.camera_zoom = crate::settings::CameraZoom::Zoom150;
    let image = app
        .frame_image(sml2_runtime::render::motion::Smooth::OFF)
        .expect("a game frame");
    assert_eq!(image.size, [384, 216]);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn mods_rows_cycle_the_view_settings() {
    let (mut app, ctx, path) = app();
    app.screen = Screen::Settings;
    app.page = super::panels::Page::Mods;
    frame(&mut app, &ctx, vec![]);
    for (row, expected) in [(1, (256, 144)), (2, (320, 180))] {
        app.menu.selected = row; // Widescreen, then Camera zoom, after the tab strip.
        app.input.event(Button::key("Enter"), true);
        frame(&mut app, &ctx, vec![]);
        app.input.event(Button::key("Enter"), false);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.settings.view(), expected);
    }
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn actual_session_frames_use_the_extracted_cartridge_palette_for_every_filter() {
    let (mut app, _, path) = app();
    app.apply_debug(DebugAction::WarpLevel(4));
    let game = app.game.as_ref().unwrap();
    let native = game.session.render_palette_frame((160, 144));
    assert_eq!(native.len(), 160 * 144);
    assert!(native
        .iter()
        .any(|p| p.source != sml2_core::video::PaletteSource::Bg));
    for filter in Filter::ALL {
        let image = colorized(
            &native,
            filter,
            &game.session.assets.cartridge_header,
            (160, 144),
        );
        assert!(screenshot::verify(&image, &image, [0, 0, 160, 144]).is_ok());
        // A changed channel at any native pixel must invalidate screenshot verification.
        let mut changed = image.clone();
        let [r, g, b, _] = changed.pixels[123].to_array();
        changed.pixels[123] = Color32::from_rgb(if r < 128 { r + 2 } else { r - 2 }, g, b);
        assert!(screenshot::verify(&changed, &image, [0, 0, 160, 144]).is_err());
    }
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn motion_smoothing_submits_a_frame_at_the_chosen_scale() {
    let (mut app, _, path) = app();
    app.apply_debug(DebugAction::WarpLevel(4));
    let smooth = sml2_runtime::render::motion::Smooth {
        alpha: 1.0,
        scale: 2,
    };
    let image = app.frame_image(smooth).unwrap();
    assert_eq!(image.size, [320, 288]);
    let off = app
        .frame_image(sml2_runtime::render::motion::Smooth::OFF)
        .unwrap();
    assert_eq!(off.size, [160, 144]);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn screenshot_verification_keeps_the_submitted_filter_when_settings_change() {
    let (mut app, ctx, path) = app();
    app.screen = Screen::Game;
    app.settings.filter = Filter::Sgb(0);
    app.options.smoke_frames = Some(1);
    app.options.screenshot = Some(path.join("frame.png"));
    frame(&mut app, &ctx, vec![]);
    let (submitted, rect) = app.screenshot_check.clone().expect("submitted RGB frame");
    let game = app.game.as_ref().unwrap();
    let expected = colorized(
        &game.session.render_palette_frame((160, 144)),
        Filter::Sgb(0),
        &game.session.assets.cartridge_header,
        (160, 144),
    );
    assert_eq!(
        submitted, expected,
        "verification keeps the exact submitted colors"
    );
    app.settings.filter = Filter::Gray;
    app.options.screenshot = None;
    // Build the native screenshot using the exact image submitted before the change.
    let [x, y, w, h] = rect.map(|v| v as usize);
    let mut screenshot_image =
        ColorImage::new([x + w, y + h], vec![Color32::BLACK; (x + w) * (y + h)]);
    let scale = w / 160;
    for py in 0..h {
        for px in 0..w {
            screenshot_image.pixels[(y + py) * (x + w) + x + px] =
                submitted.pixels[(py / scale) * 160 + px / scale];
        }
    }
    app.receive_input(&RawInput {
        events: vec![egui::Event::Screenshot {
            viewport_id: egui::ViewportId::ROOT,
            user_data: Default::default(),
            image: std::sync::Arc::new(screenshot_image),
        }],
        ..Default::default()
    });
    assert!(app.validation_error.lock().unwrap().is_none());
    assert!(app.screenshot_check.is_none());
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn sml2_gbc_colors_keep_three_distinct_sources_and_sgb_1a_uses_the_hardware_ramp() {
    use sml2_core::video::PaletteSource;
    let (app, _, path) = app();
    let header = &app.game.as_ref().unwrap().session.assets.cartridge_header;
    let sources = [PaletteSource::Bg, PaletteSource::Obj0, PaletteSource::Obj1];
    // Independent RGB888 expectations from the boot RGB555 triples, with bit replication.
    let expected = [
        [
            [255, 255, 206],
            [99, 239, 239],
            [156, 132, 49],
            [90, 90, 90],
        ],
        [[255, 255, 255], [255, 115, 0], [148, 66, 0], [0, 0, 0]],
        [[255, 255, 255], [99, 165, 255], [0, 0, 255], [0, 0, 0]],
    ];
    let palette = Filter::Gbc.palette(header);
    for (source, colors) in sources.into_iter().zip(expected) {
        for shade in 0..4 {
            assert_eq!(
                palette.rgb(PalettePixel { source, shade }),
                colors[shade as usize]
            );
        }
    }
    let palette = Filter::Sgb(0).palette(header);
    let expected = [[255, 239, 206], [222, 148, 74], [173, 41, 33], [49, 24, 82]];
    for source in sources {
        for shade in 0..4 {
            assert_eq!(
                palette.rgb(PalettePixel { source, shade }),
                expected[shade as usize]
            );
        }
    }
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn settings_pages_cycle_with_shoulders_and_the_tab_row() {
    let (mut app, ctx, path) = app();
    app.screen = Screen::Settings;
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.page, super::panels::Page::Display);
    app.input.event(Button::Pad(PadButton::RightShoulder), true);
    frame(&mut app, &ctx, vec![]);
    app.input
        .event(Button::Pad(PadButton::RightShoulder), false);
    assert_eq!(app.page, super::panels::Page::Audio);
    // The tab strip is row 0: left steps back a page instead of adjusting a value.
    frame(&mut app, &ctx, vec![key(egui::Key::ArrowLeft, true)]);
    frame(&mut app, &ctx, vec![key(egui::Key::ArrowLeft, false)]);
    assert_eq!(app.page, super::panels::Page::Display);
    app.input.event(Button::Pad(PadButton::LeftShoulder), true);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.page, super::panels::Page::Debug);
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 0);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn home_pipes_show_the_saved_stage_tally_across_a_restart() {
    let (mut app, ctx, path) = app();
    app.screen = Screen::Game;
    frame(&mut app, &ctx, vec![]);
    let slots = |app: &DesktopApp| {
        let slots = &app.game.as_ref().unwrap().session.slots;
        [0, 1, 2].map(|slot| super::panels::pipe_stages(slots, slot))
    };
    let game = app.game.as_mut().unwrap();
    game.session.active_slot = Some(0);
    game.session.debug_warp_level(0).unwrap();
    app.apply_debug(DebugAction::ClearAllLevels);
    assert_eq!(
        slots(&app)[0],
        None,
        "an unsaved edit does not reach the pipe"
    );
    app.apply_debug(DebugAction::Save);
    assert_eq!(slots(&app)[0], Some(31));
    // A new launch reads the same pipe image from disk.
    let options = Options {
        assets: app.options.assets.clone(),
        paths: AppPaths {
            settings: app.options.paths.settings.clone(),
            save: app.options.paths.save.clone(),
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
    let restarted = DesktopApp::new(&Context::default(), options, Settings::default(), None);
    assert_eq!(slots(&restarted), [Some(31), None, None]);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn the_scenery_editor_paints_a_margin_cell_and_saves_it() {
    let (mut app, ctx, path) = app();
    frame(&mut app, &ctx, vec![]);
    app.open_editor(&ctx);
    assert!(app.screen == Screen::Editor);
    app.editor.cam = (80, 448);
    app.editor.metatile = 1;
    app.paint_cell(-1, 24).unwrap();
    frame(&mut app, &ctx, vec![]);
    let SessionState::InLevel(level) = &app.game.as_ref().unwrap().session.state else {
        panic!("editor shows a level")
    };
    assert_eq!(level.scenery.get(-1, 24), Some(1));
    let saved = std::fs::read_to_string(path.join("scenery-00.json")).unwrap();
    assert!(saved.contains("\"metatile\": 1"));
    let _ = std::fs::remove_dir_all(path);
}

fn overlay_rects_scale_view_pixels_from_the_image_origin() {
    let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(480.0, 432.0));
    let r = overlay_rect(rect, (160, 144), (5, 7, 10, 21));
    assert_eq!(r.min, egui::pos2(25.0, 41.0));
    assert_eq!(r.size(), egui::vec2(30.0, 63.0));
}
