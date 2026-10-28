use crate::app::editor::state::Tool;
use crate::app::{tests::app, Screen};
use sml2_core::scenery::{SceneryOverlay, SceneryPaint};
use sml2_runtime::scene_scenery::SceneTarget;

#[test]
fn existing_signed_scenery_files_round_trip_without_changing_the_format() {
    let (mut app, ctx, path) = app();
    std::fs::create_dir_all(&path).unwrap();
    let file = path.join("scenery-00.json");
    std::fs::write(&file, r#"{"paints":[{"x":-2,"y":48,"metatile":1}]}"#).unwrap();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    assert_eq!(work.sample((-2, 48)), Some(1));
    work.metatile = 2;
    work.begin((256, -1), false);
    work.finish();
    app.save_scenery();
    assert!(!app.editor.work.as_ref().unwrap().dirty);
    let saved: SceneryOverlay = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(saved.get(-2, 48), Some(1));
    assert_eq!(saved.get(256, -1), Some(2));
    assert!(std::fs::read_dir(&path).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".tmp")));
    app.editor = Default::default();
    app.open_editor(&ctx);
    assert_eq!(app.editor.work.as_ref().unwrap().scenery, saved);
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn malformed_duplicate_and_invalid_tile_files_are_never_silently_replaced() {
    for text in [
        "broken json",
        r#"{"paints":[{"x":-1,"y":0,"metatile":255}]}"#,
        r#"{"paints":[{"x":-1,"y":0,"metatile":1},{"x":-1,"y":0,"metatile":2}]}"#,
    ] {
        let (mut app, ctx, path) = app();
        std::fs::create_dir_all(&path).unwrap();
        let file = path.join("scenery-00.json");
        std::fs::write(&file, text).unwrap();
        app.open_editor(&ctx);
        let work = app.editor.work.as_mut().unwrap();
        assert!(work.load_error.is_some());
        work.begin((-2, 0), false);
        assert!(!work.finish());
        app.save_scenery();
        app.play_editor(&ctx);
        assert!(app.screen == Screen::Editor);
        assert_eq!(std::fs::read_to_string(file).unwrap(), text);
        std::fs::write(
            path.join("scenery-00.json"),
            r#"{"paints":[{"x":-3,"y":48,"metatile":2}]}"#,
        )
        .unwrap();
        app.reload_scenery();
        let work = app.editor.work.as_ref().unwrap();
        assert!(work.load_error.is_none());
        assert_eq!(work.sample((-3, 48)), Some(2));
        std::fs::remove_dir_all(path).ok();
    }
}

#[test]
fn failed_atomic_save_keeps_edits_across_levels_and_retry_writes_them() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 1;
    work.begin((-1, 0), false);
    work.finish();
    let file = path.join("scenery-00.json");
    std::fs::create_dir_all(&file).unwrap();
    app.save_scenery();
    let work = app.editor.work.as_ref().unwrap();
    assert!(work.dirty);
    assert!(work.save_error.is_some());
    app.select_editor_level(1);
    app.select_editor_level(0);
    assert_eq!(app.editor.work.as_ref().unwrap().sample((-1, 0)), Some(1));
    assert!(app.editor.work.as_ref().unwrap().can_undo());
    std::fs::remove_dir(&file).unwrap();
    app.editor.work.as_mut().unwrap().save_error = None;
    app.save_scenery();
    assert!(!app.editor.work.as_ref().unwrap().dirty);
    let saved: SceneryOverlay = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(saved.get(-1, 0), Some(1));
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn level_switches_preserve_view_tool_selection_and_both_history_stacks() {
    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 3;
    work.viewport.center = eframe::egui::pos2(-300.0, 1000.0);
    work.viewport.zoom = 6;
    work.tool = Tool::Rectangle;
    work.begin((-1, 0), false);
    work.finish();
    work.begin((-2, 0), false);
    work.finish();
    work.undo();
    app.select_editor_level(1);
    assert!(!app.editor.work.as_ref().unwrap().can_undo());
    app.select_editor_level(0);
    let work = app.editor.work.as_mut().unwrap();
    assert_eq!(work.viewport.center, eframe::egui::pos2(-300.0, 1000.0));
    assert_eq!(work.viewport.zoom, 6);
    assert_eq!(work.metatile, 3);
    assert_eq!(work.tool, Tool::Rectangle);
    assert!(work.can_undo() && work.can_redo());
    work.redo();
    assert_eq!(work.scenery.len(), 2);
    app.navigate(&ctx, Screen::Home);
    assert!(!app.editor.work.as_ref().unwrap().dirty);
    app.open_editor(&ctx);
    assert_eq!(app.editor.work.as_ref().unwrap().scenery.len(), 2);
    std::fs::remove_dir_all(path).ok();
}

fn paint(metatile: u8, x: i16, y: i16) -> SceneryOverlay {
    SceneryOverlay::try_from_paints(vec![SceneryPaint { x, y, metatile }], 90).unwrap()
}

#[test]
fn scene_drafts_stay_separate_from_level_drafts_and_empty_files_override_bundles() {
    let (mut app, ctx, path) = app();
    std::fs::create_dir_all(&path).unwrap();
    let level_text = r#"{"paints":[{"x":-2,"y":48,"metatile":1}]}"#;
    let map_text = r#"{"paints":[{"x":-1,"y":0,"metatile":23},{"x":0,"y":0,"metatile":4}]}"#;
    std::fs::write(path.join("scenery-00.json"), level_text).unwrap();
    std::fs::write(path.join("scenery-map-09.json"), map_text).unwrap();
    std::fs::write(path.join("scenery-arena-09-02.json"), r#"{"paints":[]}"#).unwrap();
    let session = &mut app.game.as_mut().unwrap().session;
    let bundled_level = session.assets.level(0).unwrap().scenery.clone();
    session
        .set_scene_scenery(SceneTarget::Map(9), paint(8, -3, 1))
        .unwrap();
    session
        .set_scene_scenery(SceneTarget::Arena { level: 9, gfx: 2 }, paint(5, -4, 2))
        .unwrap();
    session
        .set_scene_scenery(SceneTarget::Title, paint(6, -5, 3))
        .unwrap();

    app.open_editor(&ctx);
    let work = app.editor.work.as_ref().unwrap();
    assert!(app.editor.scene.is_none());
    assert_eq!(work.sample((-2, 48)), Some(1));
    assert_eq!(
        app.game
            .as_ref()
            .unwrap()
            .session
            .assets
            .level(0)
            .unwrap()
            .scenery,
        bundled_level,
        "opening a draft must not publish it into the preview session"
    );

    app.select_editor_scene(SceneTarget::Map(9));
    let work = app.editor.work.as_ref().unwrap();
    assert!(work.scene_border);
    assert_eq!(work.sample((-1, 0)), Some(23));
    assert_eq!(
        work.paint((0, 0)),
        Some(4),
        "existing interior paints stay in the draft"
    );
    assert_eq!(
        work.sample((-3, 1)),
        None,
        "a draft replaces the bundled surround"
    );
    assert_eq!(
        app.game
            .as_ref()
            .unwrap()
            .session
            .scene_scenery(SceneTarget::Map(9))
            .get(-3, 1),
        Some(8),
        "the bundled scene stays in the preview until playtest"
    );

    app.select_editor_scene(SceneTarget::Title);
    assert_eq!(app.editor.work.as_ref().unwrap().sample((-5, 3)), Some(6));
    assert!(!path.join("scenery-title.json").exists());

    app.select_editor_scene(SceneTarget::Arena { level: 9, gfx: 2 });
    assert!(app.editor.work.as_ref().unwrap().scenery.is_empty());
    assert_eq!(app.editor.work.as_ref().unwrap().sample((-4, 2)), None);

    app.select_editor_level(0);
    assert_eq!(app.editor.work.as_ref().unwrap().sample((-2, 48)), Some(1));
    assert_eq!(
        std::fs::read_to_string(path.join("scenery-00.json")).unwrap(),
        level_text
    );
    assert_eq!(
        std::fs::read_to_string(path.join("scenery-map-09.json")).unwrap(),
        map_text
    );

    std::fs::write(path.join("scenery-castle.json"), "broken").unwrap();
    app.select_editor_scene(SceneTarget::Castle);
    assert!(app.editor.work.as_ref().unwrap().load_error.is_some());
    app.editor.work.as_mut().unwrap().begin((-1, 0), false);
    assert!(!app.editor.work.as_mut().unwrap().finish());
    app.save_scenery();
    app.select_editor_level(0);
    app.select_editor_scene(SceneTarget::Castle);
    assert!(app.editor.work.as_ref().unwrap().load_error.is_some());
    assert_eq!(
        std::fs::read_to_string(path.join("scenery-castle.json")).unwrap(),
        "broken"
    );
    assert_eq!(
        std::fs::read_to_string(path.join("scenery-00.json")).unwrap(),
        level_text
    );
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn export_snapshots_one_target_and_a_failed_write_keeps_the_draft_and_undo() {
    let (mut app, ctx, path) = app();
    std::fs::create_dir_all(&path).unwrap();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 2;
    work.begin((-1, 0), false);
    app.begin_export();
    assert_eq!(app.editor_export_path(), Some("scenery-00.json"));
    assert!(!app.editor.work.as_ref().unwrap().dirty);
    assert!(app.editor.work.as_ref().unwrap().can_undo());
    let draft = path.join("scenery-00.json");
    let drafted: SceneryOverlay = serde_json::from_slice(&std::fs::read(&draft).unwrap()).unwrap();
    assert_eq!(drafted.get(-1, 0), Some(2));

    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 3;
    work.begin((-2, 0), false);
    work.finish();
    app.save_scenery();
    let blocked = path.join("blocked.json");
    std::fs::create_dir(&blocked).unwrap();
    assert!(app.export_to(&blocked).is_err());
    assert!(blocked.is_dir());
    assert!(std::fs::read_dir(&path).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
    app.editor.work.as_mut().unwrap().undo();
    assert_eq!(app.editor.work.as_ref().unwrap().sample((-2, 0)), None);
    assert_eq!(app.editor.work.as_ref().unwrap().sample((-1, 0)), Some(2));
    app.editor.work.as_mut().unwrap().redo();

    let release = path.join("release").join("scenery-00.json");
    app.export_to(&release).unwrap();
    let exported: SceneryOverlay =
        serde_json::from_slice(&std::fs::read(&release).unwrap()).unwrap();
    assert_eq!(exported.get(-1, 0), Some(2));
    assert_eq!(
        exported.get(-2, 0),
        None,
        "export keeps the snapshot from when it opened"
    );
    let drafted: SceneryOverlay = serde_json::from_slice(&std::fs::read(&draft).unwrap()).unwrap();
    assert_eq!(drafted.get(-2, 0), Some(3));

    app.editor.export = None;
    app.select_editor_scene(SceneTarget::Map(9));
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 7;
    work.begin((-1, -1), false);
    work.extend(Some((1, 1)));
    work.finish();
    assert_eq!(work.paint((-1, -1)), Some(7));
    assert_eq!(work.paint((0, 0)), None);
    assert_eq!(work.paint((1, 1)), None);
    app.begin_export();
    assert_eq!(app.editor_export_path(), Some("scenery-map-09.json"));
    let scene_release = path.join("release").join("scenery-map-09.json");
    app.export_to(&scene_release).unwrap();
    let scene: SceneryOverlay =
        serde_json::from_slice(&std::fs::read(&scene_release).unwrap()).unwrap();
    assert_eq!(scene.get(-1, -1), Some(7));
    assert_eq!(scene.get(0, 0), None);
    let level_release: SceneryOverlay =
        serde_json::from_slice(&std::fs::read(&release).unwrap()).unwrap();
    assert_eq!(level_release, exported);
    assert!(app.editor.work.as_ref().unwrap().load_error.is_none());
    std::fs::write(path.join("scenery-title.json"), "broken").unwrap();
    app.select_editor_scene(SceneTarget::Title);
    assert!(app.editor.work.as_ref().unwrap().load_error.is_some());
    app.begin_export();
    assert!(!app.editor_export_open());
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn switching_levels_and_scenes_keeps_each_workspace() {
    let (mut app, ctx, path) = app();
    std::fs::create_dir_all(&path).unwrap();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 3;
    work.viewport.center = eframe::egui::pos2(-300.0, 1000.0);
    work.viewport.zoom = 6;
    work.tool = Tool::Rectangle;
    work.begin((-1, 0), false);
    work.finish();
    work.begin((-2, 0), false);
    assert!(
        !path.join("scenery-00.json").exists(),
        "an open gesture waits to be saved"
    );
    app.select_editor_scene(SceneTarget::Map(6));
    assert_eq!(
        serde_json::from_slice::<SceneryOverlay>(
            &std::fs::read(path.join("scenery-00.json")).unwrap()
        )
        .unwrap()
        .get(-2, 0),
        Some(3),
        "leaving a target finishes its gesture into that target's draft"
    );

    let work = app.editor.work.as_mut().unwrap();
    assert!(work.scene_border);
    assert!(!work.can_undo());
    assert_eq!(work.tool, Tool::Brush);
    work.metatile = 8;
    work.viewport.zoom = 4;
    work.tool = Tool::Rectangle;
    work.begin((0, 0), false);
    assert!(!work.finish(), "the original picture cannot be painted");
    assert!(work.scenery.is_empty());
    work.begin((-1, 0), false);
    work.extend(Some((2, 0)));
    work.finish();
    assert_eq!(work.paint((-1, 0)), Some(8));
    assert_eq!(work.paint((0, 0)), None);
    assert_eq!(work.paint((1, 0)), None);
    work.undo();

    app.select_editor_level(1);
    assert!(app.editor.scene.is_none());
    assert!(app.editor.work.as_ref().unwrap().scenery.is_empty());
    assert!(!path.join("scenery-01.json").exists());

    app.select_editor_level(0);
    let work = app.editor.work.as_mut().unwrap();
    assert_eq!(work.viewport.center, eframe::egui::pos2(-300.0, 1000.0));
    assert_eq!(work.viewport.zoom, 6);
    assert_eq!(work.metatile, 3);
    assert_eq!(work.tool, Tool::Rectangle);
    assert_eq!(work.scenery.len(), 2);
    assert!(work.can_undo() && !work.can_redo());

    app.select_editor_scene(SceneTarget::Map(6));
    let work = app.editor.work.as_mut().unwrap();
    assert_eq!(work.viewport.zoom, 4);
    assert_eq!(work.metatile, 8);
    assert_eq!(work.tool, Tool::Rectangle);
    assert!(work.can_redo());
    assert!(work.scenery.is_empty());
    work.redo();
    assert_eq!(work.paint((-1, 0)), Some(8));
    app.navigate(&ctx, Screen::Home);
    app.editor = Default::default();
    app.open_editor(&ctx);
    app.select_editor_scene(SceneTarget::Map(6));
    assert_eq!(app.editor.work.as_ref().unwrap().paint((-1, 0)), Some(8));
    assert_eq!(app.editor.work.as_ref().unwrap().paint((0, 0)), None);
    let level: SceneryOverlay =
        serde_json::from_slice(&std::fs::read(path.join("scenery-00.json")).unwrap()).unwrap();
    assert_eq!(level.get(-1, 0), Some(3));
    assert_eq!(level.get(-2, 0), Some(3));
    std::fs::remove_dir_all(path).ok();
}

/// With the game's Expanded levels setting on, Tree Zone 2 opens as its
/// rebuilt 17×4 map. Its gaps are cells of that map, so their scenery is a
/// separate draft and export, and playtest enters the rebuilt level with it.
#[test]
fn a_rebuilt_level_edits_its_own_scenery_on_the_rebuilt_map() {
    let (mut app, ctx, path) = app();
    std::fs::create_dir_all(&path).unwrap();
    assert!(app.settings.expanded_levels);
    app.open_editor(&ctx);
    app.select_editor_level(2);
    let work = app.editor.work.as_mut().unwrap();
    assert!(work.expanded);
    assert_eq!(work.dimensions(), (17 * 16, 4 * 16));
    // Row 0, screen 5: the rebuild emptied it, and the camera can see it.
    assert_eq!(work.layout.get(80, 5), Some(96));
    let cells = work.layout.cells.clone();
    app.editor_atlas(&ctx);
    assert_eq!(
        app.editor.atlas.as_ref().unwrap().layout,
        cells,
        "the canvas draws the rebuilt map"
    );
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 2;
    work.begin((80, 5), false);
    work.finish();
    app.save_scenery();
    let draft = path.join("scenery-02-expanded.json");
    let saved: SceneryOverlay = serde_json::from_slice(&std::fs::read(&draft).unwrap()).unwrap();
    assert_eq!(saved.get(80, 5), Some(2));
    assert!(
        !path.join("scenery-02.json").exists(),
        "the original's draft is untouched"
    );

    app.begin_export();
    assert_eq!(app.editor_export_path(), Some("scenery-02-expanded.json"));
    app.editor.export = None;

    app.play_editor(&ctx);
    let session = &app.game.as_ref().unwrap().session;
    let sml2_runtime::session::SessionState::InLevel(level) = &session.state else {
        panic!("playtest enters the level")
    };
    assert_eq!(
        level.layout_live.world,
        sml2_core::world::World::expanded(17, 4).unwrap()
    );
    assert_eq!(level.scenery.get(80, 5), Some(2));
    std::fs::remove_dir_all(path).ok();
}

/// With the setting off, the same level is the original map and its draft.
#[test]
fn with_expanded_levels_off_a_rebuilt_level_edits_the_original_map() {
    let (mut app, ctx, path) = app();
    std::fs::create_dir_all(&path).unwrap();
    app.settings.expanded_levels = false;
    app.open_editor(&ctx);
    app.select_editor_level(2);
    let work = app.editor.work.as_mut().unwrap();
    assert!(!work.expanded);
    assert_eq!(work.dimensions(), (256, 48));
    work.metatile = 2;
    work.begin((-1, 0), false);
    work.finish();
    app.save_scenery();
    assert!(path.join("scenery-02.json").exists());
    assert!(!path.join("scenery-02-expanded.json").exists());
    app.play_editor(&ctx);
    let session = &app.game.as_ref().unwrap().session;
    let sml2_runtime::session::SessionState::InLevel(level) = &session.state else {
        panic!("playtest enters the level")
    };
    assert_eq!(level.layout_live.world, sml2_core::world::World::ROM);
    std::fs::remove_dir_all(path).ok();
}
