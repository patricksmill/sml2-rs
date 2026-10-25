use super::*;
use crate::settings::AppPaths;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

pub(super) fn app() -> (EditorApp, Context, PathBuf) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "sml2_scenery_editor_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let options = Options {
        assets: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"),
        explicit_assets: true,
        paths: AppPaths {
            settings: path.join("settings.json"),
            save: path.join("save.sav"),
        },
        audio: false,
        smoke_frames: None,
    };
    let ctx = Context::default();
    let mut app = EditorApp::new(&ctx, options, Settings::default(), None);
    assert!(app.game.is_some(), "test requires extracted game data");
    app.editor = Default::default();
    app.screen = Screen::Home;
    (app, ctx, path)
}

pub(super) fn frame(
    app: &mut EditorApp,
    ctx: &Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    super::editor::tests::workspace_ui::frame(app, ctx, (800.0, 720.0), events)
}

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
                15
            ],
            bell_tiles: Default::default(),
            room_tiles: Default::default(),
            sprites: sml2_dx::MapSprites {
                hub_decor: vec![vec![]; 17],
                zone_decor: vec![vec![]; 188],
                status_bar: [(0, 0, 0, 0); 9],
                room_metasprites: Default::default(),
            },
        },
        arenas: Vec::new(),
        arena_metatiles: Vec::new(),
        boss_oam_lists: Vec::new(),
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

#[test]
fn authoring_and_playtests_leave_game_settings_and_progress_untouched() {
    let (mut app, ctx, path) = app();
    std::fs::create_dir_all(&path).unwrap();
    let progress = b"player save must survive";
    std::fs::write(&app.options.paths.save, progress).unwrap();
    let settings = b"existing settings must survive";
    std::fs::write(&app.options.paths.settings, settings).unwrap();
    app.open_editor(&ctx);
    app.play_editor(&ctx);
    app.game.as_mut().unwrap().session.active_slot = Some(2);
    app.game.as_mut().unwrap().save_now().unwrap();
    app.game.as_mut().unwrap().tick(Default::default(), 0.0);
    assert_eq!(std::fs::read(&app.options.paths.save).unwrap(), progress);
    assert_eq!(
        std::fs::read(&app.options.paths.settings).unwrap(),
        settings
    );
    std::fs::remove_dir_all(path).ok();
}
