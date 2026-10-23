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

/// Run with SML2_DX_ROM=/path/to/audited.gbc cargo test -p sml2-desktop
/// real_rom_import_restart -- --ignored
#[test]
#[ignore = "requires the user-supplied audited DX ROM and extracted base assets"]
fn real_rom_import_restart_and_dx_mode_preserve_saves_and_cancelled_installation() {
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

    let (key, bundle) =
        dx_import::import_path(&cache, &assets, &rom, &AtomicBool::new(false), |m| {
            eprintln!("DX import: {m}")
        })
        .unwrap();
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

    use sml2_core::character::Character;
    let mut game = Game::new(
        SessionAssets::load_from(&ExtractedContent::new(&assets)).unwrap(),
        save.clone(),
        false,
    );
    assert_eq!(game.session.active_slot, Some(1));
    let loaded_slots = game.session.slots.image();
    game.session.debug_warp_level(0).unwrap();
    game.session.advance(Default::default());
    assert!(game.session.dx_supported());
    let (_, dx) = dx_import::presentation(Ok((restored_key, bundle))).unwrap();
    let mario = game.session.render_dx_frame(&dx, Character::Mario).unwrap();
    let luigi = game.session.render_dx_frame(&dx, Character::Luigi).unwrap();
    assert_eq!(mario.len(), 160 * 144);
    assert_ne!(mario, luigi);
    // DX presentation never touches progress or the save file.
    assert_eq!(game.session.slots.image(), loaded_slots);
    assert_eq!(std::fs::read(save).unwrap(), saved);
}
