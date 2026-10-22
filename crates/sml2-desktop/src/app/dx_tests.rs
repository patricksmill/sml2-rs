use super::*;
use crate::dx_import::Worker;
use sml2_dx::cache::CacheKey;

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
