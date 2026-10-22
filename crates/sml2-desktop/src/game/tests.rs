use super::*;
use crate::timing::TickBudget;
use sml2_core::front_end::SaveSlots;
use sml2_core::map_sim::MapSim;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn temp_save(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "sml2_desktop_{name}_{}_{id}.sav",
        std::process::id()
    ))
}

#[test]
fn catch_up_ticks_each_render_audio_at_the_live_game_rate() {
    let save = std::env::temp_dir().join(format!("sml2-audio-ticks-{}.sav", std::process::id()));
    let mut game = Game::new(SessionAssets::load().unwrap(), save, false);
    let mut budget = TickBudget::default();
    let mut samples = 0;
    let mut ticks = 0;
    for elapsed in [50, 0, 16, 34] {
        for _ in 0..budget.advance(Duration::from_millis(elapsed), true) {
            game.tick(PlayerKeys::default(), 1.0);
            assert_eq!(game.buffer.len(), 735 * 2);
            samples += game.buffer.len() / 2;
            ticks += 1;
        }
    }
    assert_eq!(ticks, 6);
    assert_eq!(game.session.sound_driver.update_count, 6);
    assert_eq!(samples, 4410, "100 ms of gameplay produces 100 ms of PCM");
}

#[test]
fn a_legacy_single_record_save_migrates_into_pipe_a() {
    let path = temp_save("migrate");
    let record = MapSim::new().save_record(5, 0x12, 0);
    std::fs::write(&path, record).unwrap();

    let game = Game::new(SessionAssets::load().unwrap(), path.clone(), false);
    assert!(game.session.slots.valid(0));
    assert_eq!(game.session.slots.load_slot(0), Some(record));
    assert_eq!(game.session.active_slot, Some(0));
    assert_eq!(game.session.level_tally, 0x12);
    assert_eq!(
        std::fs::read(&path).unwrap().len(),
        SAVE_IMAGE_LEN,
        "the legacy file is rewritten as the three-pipe image"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_slot_image_resumes_the_first_valid_pipe() {
    let path = temp_save("image");
    let mut slots = SaveSlots::new();
    let record = MapSim::new().save_record(5, 0x07, 0);
    assert!(slots.write_slot(1, &record));
    std::fs::write(&path, slots.image()).unwrap();

    let game = Game::new(SessionAssets::load().unwrap(), path.clone(), false);
    assert_eq!(game.session.active_slot, Some(1));
    assert_eq!(game.session.level_tally, 7);
    assert!(matches!(
        game.session.state,
        sml2_runtime::session::SessionState::Overworld(_)
    ));
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_wrong_length_save_is_reported_and_left_alone() {
    let path = temp_save("short");
    std::fs::write(&path, b"abc").unwrap();
    let game = Game::new(SessionAssets::load().unwrap(), path.clone(), false);
    assert!(game
        .diagnostics
        .iter()
        .any(|d| d.contains("is not a 240-byte slot image")));
    assert_eq!(std::fs::read(&path).unwrap(), b"abc");
    let _ = std::fs::remove_file(path);
}

#[test]
fn saving_persists_all_three_pipes_as_one_image() {
    let path = temp_save("persist");
    let mut slots = SaveSlots::new();
    assert!(slots.write_slot(0, &MapSim::new().save_record(5, 1, 0)));
    std::fs::write(&path, slots.image()).unwrap();

    let mut game = Game::new(SessionAssets::load().unwrap(), path.clone(), false);
    assert!(game
        .session
        .slots
        .write_slot(2, &MapSim::new().save_record(5, 9, 0)));
    game.persist_slots().unwrap();

    let data = std::fs::read(&path).unwrap();
    assert_eq!(data.len(), SAVE_IMAGE_LEN);
    assert_eq!(data, game.session.slots.image());
    assert!(game.session.slots.valid(2));
    let _ = std::fs::remove_file(path);
}

#[test]
fn dx_preview_never_opens_migrates_or_writes_normal_save() {
    let path = temp_save("dx-isolation");
    let legacy = MapSim::new().save_record(5, 0x12, 0);
    std::fs::write(&path, legacy).unwrap();
    let mut game = Game::preview(SessionAssets::load().unwrap(), false).unwrap();
    // Even accidentally assigning the normal path cannot enable persistence.
    game.save = path.clone();
    assert_eq!(game.session.active_slot, None);
    assert_eq!(game.session.level_tally, 0);
    assert!(!game.preview_finished());
    game.session.active_slot = Some(0);
    assert!(game.save_now().is_err());
    game.persist_slots().unwrap();
    game.tick(PlayerKeys::default(), 0.0);
    assert_eq!(std::fs::read(&path).unwrap(), legacy);
    let sml2_runtime::session::SessionState::InLevel(ctx) = &mut game.session.state else {
        panic!("preview must start directly in opening level");
    };
    assert_eq!(ctx.manifest.level_id, 0);
    ctx.sim.dead = 1;
    assert!(game.preview_finished());
    if let sml2_runtime::session::SessionState::InLevel(ctx) = &mut game.session.state {
        ctx.sim.dead = 0;
        ctx.sim.exit_completed = true;
    }
    assert!(game.preview_finished());
    game.session.enter_title();
    assert!(game.preview_finished());
    std::fs::remove_file(path).unwrap();
}

/// Run with SML2_DX_ROM=/path/to/audited.gbc cargo test -p sml2-desktop
/// real_rom_import_restart_preview -- --ignored
#[test]
#[ignore = "requires the user-supplied audited DX ROM and extracted base assets"]
fn real_rom_import_restart_preview_preserves_saves_and_cancelled_installation() {
    use crate::{dx_import, settings::Settings};
    use sml2_runtime::content::ExtractedContent;
    use std::sync::atomic::AtomicBool;
    let rom = PathBuf::from(std::env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM"));
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("cache");
    let save = temp.path().join("normal.sav");
    let config = temp.path().join("settings.json");
    let mut slots = SaveSlots::new();
    slots.write_slot(1, &MapSim::new().save_record(5, 7, 0));
    let saved = slots.image();
    std::fs::write(&save, saved).unwrap();

    let worker_cache = cache.clone();
    let worker_assets = assets.clone();
    let worker_rom = rom.clone();
    let worker = dx_import::Worker::start(move |cancel, progress| {
        dx_import::import_path(&worker_cache, &worker_assets, &worker_rom, cancel, progress)
            .map_err(|e| e.to_string())
    });
    let (key, bundle) = loop {
        match worker
            .events
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
        {
            dx_import::Event::Progress(message) => eprintln!("DX import: {message}"),
            dx_import::Event::Finished(result) => break result.unwrap(),
            dx_import::Event::FileChosen(_) => panic!("import worker unexpectedly opened a picker"),
        }
    };
    let installed = bundle.encode().unwrap();
    let mut settings = Settings::default();
    settings.dx_installation = Some(key.clone());
    settings.save(&config).unwrap();
    drop(bundle);
    drop(settings);

    let restored_settings = Settings::load(&config).unwrap();
    let (restored_key, bundle) = dx_import::restore(
        &cache,
        &assets,
        restored_settings.dx_installation.unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(restored_key, key);
    assert_eq!(bundle.encode().unwrap(), installed);
    assert!(matches!(
        dx_import::import_path(&cache, &assets, &rom, &AtomicBool::new(true), |_| {},),
        Err(sml2_dx::ImportError::Cancelled)
    ));
    let (_, unchanged) = dx_import::restore(&cache, &assets, key, &AtomicBool::new(false)).unwrap();
    assert_eq!(unchanged.encode().unwrap(), installed);

    let normal = Game::new(
        SessionAssets::load_from(&ExtractedContent::new(&assets)).unwrap(),
        save.clone(),
        false,
    );
    // Normal loading initializes empty slot records in memory. The preview must
    // preserve that loaded state as well as the original bytes on disk.
    let normal_slots = normal.session.slots.image();
    let mut preview = Game::preview(
        SessionAssets::load_from(&ExtractedContent::new(&assets)).unwrap(),
        false,
    )
    .unwrap();
    assert_eq!(preview.session.active_slot, None);
    assert_eq!(
        preview.session.frame_n, 1,
        "initial small-Mario draw state is primed"
    );
    assert!(matches!(&preview.session.state,
        sml2_runtime::session::SessionState::InLevel(ctx) if ctx.sim.powerup == 0));
    assert_eq!(
        preview.session.render_dx_frame(&bundle).unwrap().len(),
        160 * 144
    );
    preview.save = save.clone();
    preview.session.active_slot = Some(0);
    assert!(preview.save_now().is_err());
    preview.tick(PlayerKeys::default(), 0.0);
    assert_eq!(std::fs::read(save).unwrap(), saved);
    assert_eq!(normal.session.slots.image(), normal_slots);
    assert_eq!(normal.session.active_slot, Some(1));
}
