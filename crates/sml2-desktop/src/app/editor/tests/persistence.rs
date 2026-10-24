use crate::app::editor::state::Tool;
use crate::app::{tests::app, Screen};
use sml2_core::scenery::SceneryOverlay;

#[test]
fn saved_scenery_loads_on_restart_through_every_save_pipe() {
    use sml2_core::player_sim::PlayerKeys;
    use sml2_runtime::{session::SessionState, FrameInput};

    for slot in 0..3 {
        let (app, ctx, path) = app();
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("scenery-00.json"),
            r#"{"paints":[{"x":-2,"y":48,"metatile":1}]}"#,
        )
        .unwrap();
        let mut restarted =
            super::super::DesktopApp::new(&ctx, app.options, Default::default(), None);
        assert!(
            restarted.editor.work.is_none(),
            "no editor or playtest needed"
        );
        let session = &mut restarted.game.as_mut().unwrap().session;
        session.enter_file_select();
        let tick = |session: &mut sml2_runtime::Session, keys| {
            session.advance(FrameInput { keys, div: 0 });
        };
        for _ in 0..64 {
            tick(session, PlayerKeys::default());
        }
        for _ in 0..slot {
            tick(
                session,
                PlayerKeys {
                    right: true,
                    ..Default::default()
                },
            );
            for _ in 0..40 {
                tick(session, PlayerKeys::default());
            }
        }
        tick(
            session,
            PlayerKeys {
                down: true,
                ..Default::default()
            },
        );
        for _ in 0..100 {
            tick(session, PlayerKeys::default());
        }
        assert_eq!(session.active_slot, Some(slot));
        assert!(matches!(session.state, SessionState::Overworld(_)));
        session.begin_level(0, 0).unwrap();
        let SessionState::InLevel(level) = &session.state else {
            panic!("level 0 must load");
        };
        assert_eq!(level.scenery.get(-2, 48), Some(1));
        std::fs::remove_dir_all(path).ok();
    }
}

#[test]
fn saved_editor_changes_apply_on_normal_entry_and_erasing_clears_them() {
    use sml2_runtime::session::SessionState;

    let (mut app, ctx, path) = app();
    app.open_editor(&ctx);
    let work = app.editor.work.as_mut().unwrap();
    work.metatile = 1;
    work.begin((-2, 48), false);
    work.finish();
    app.save_scenery();
    for erased in [false, true] {
        if erased {
            let work = app.editor.work.as_mut().unwrap();
            work.begin((-2, 48), true);
            work.finish();
            app.save_scenery();
        }
        let session = &mut app.game.as_mut().unwrap().session;
        session.begin_level(0, 0).unwrap();
        let SessionState::InLevel(level) = &session.state else {
            panic!("level 0 must load");
        };
        assert_eq!(
            level.scenery.get(-2, 48),
            if erased { None } else { Some(1) }
        );
        assert!(!app.editor.playtesting);
    }
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn startup_reports_invalid_scenery_and_still_loads_other_levels() {
    let (app, ctx, path) = app();
    std::fs::create_dir_all(&path).unwrap();
    let file = path.join("scenery-00.json");
    std::fs::write(&file, "broken json").unwrap();
    std::fs::write(
        path.join("scenery-01.json"),
        r#"{"paints":[{"x":-2,"y":48,"metatile":1}]}"#,
    )
    .unwrap();
    let restarted = super::super::DesktopApp::new(&ctx, app.options, Default::default(), None);
    let game = restarted.game.as_ref().unwrap();
    assert!(game.session.assets.level(0).unwrap().scenery.is_empty());
    assert_eq!(
        game.session.assets.level(1).unwrap().scenery.get(-2, 48),
        Some(1)
    );
    assert!(game
        .diagnostics
        .iter()
        .any(|error| error.contains("scenery-00.json")));
    assert_eq!(std::fs::read_to_string(file).unwrap(), "broken json");
    std::fs::remove_dir_all(path).ok();
}

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
