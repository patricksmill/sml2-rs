use super::*;
use crate::timing::TickBudget;
use sml2_core::front_end::SaveSlots;
use sml2_core::map_sim::MapSim;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn temp_save(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("sml2_desktop_{name}_{}_{id}.sav", std::process::id()))
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
