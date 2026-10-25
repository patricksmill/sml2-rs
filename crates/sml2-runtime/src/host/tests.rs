use super::*;
use crate::session::SessionState;
use std::sync::atomic::{AtomicUsize, Ordering};

fn save_path(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "sml2_host_{name}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn runner(path: PathBuf) -> SessionRunner {
    SessionRunner::new(SessionAssets::load().unwrap(), path, 1, (160, 144), true)
}

#[test]
fn headless_autosave_failure_returns_the_destination_and_preserves_existing_data() {
    let blocked = save_path("blocked");
    fs::write(&blocked, b"existing data").unwrap();
    let destination = blocked.join("save.sav");
    let mut host = runner(destination.clone());
    let SessionState::Overworld(map) = &mut host.state else {
        panic!("map")
    };
    map.pending_save = true;
    let error = host.run_headless(1, None).unwrap_err().to_string();
    assert!(error.contains(&destination.display().to_string()));
    assert_eq!(fs::read(&blocked).unwrap(), b"existing data");
    fs::remove_file(blocked).unwrap();
}

#[test]
fn legacy_save_migrates_and_all_three_pipes_survive_explicit_and_automatic_saves() {
    use sml2_core::map_sim::MapSim;
    let path = save_path("slots");
    let record = MapSim::new().save_record(7, 0x12, 3);
    fs::write(&path, record).unwrap();
    let mut host = runner(path.clone());
    assert_eq!(host.active_slot, Some(0));
    assert_eq!(fs::read(&path).unwrap().len(), SAVE_IMAGE_LEN);
    host.session
        .slots
        .write_slot(1, &MapSim::new().save_record(8, 0x23, 4));
    host.session
        .slots
        .write_slot(2, &MapSim::new().save_record(9, 0x34, 5));
    for slot in 0..3 {
        host.session.active_slot = Some(slot);
        host.session.level_tally = 0x40 + slot;
        host.save_to_disk().unwrap();
        let image = fs::read(&path).unwrap();
        let persisted = SaveSlots::from_image(&image);
        assert_eq!(image.len(), SAVE_IMAGE_LEN);
        assert_eq!(persisted.slot(slot as usize).unwrap()[0x42], 0x40 + slot);
        for other in 0..3 {
            if other != slot as usize {
                assert_eq!(persisted.slot(other), host.slots.slot(other));
            }
        }
        host.session.slots = persisted;
        let SessionState::Overworld(map) = &mut host.state else {
            panic!("map")
        };
        map.pending_save = true;
        host.step_frame(PlayerKeys::default());
        assert_eq!(fs::read(&path).unwrap(), host.slots.image());
    }
    fs::remove_file(path).unwrap();
}

#[test]
fn loads_a_later_valid_pipe_and_leaves_malformed_files_untouched() {
    use sml2_core::map_sim::MapSim;
    let path = save_path("later_pipe");
    let record = MapSim::new().save_record(8, 0x23, 4);
    let mut slots = SaveSlots::new();
    slots.write_slot(2, &record);
    fs::write(&path, slots.image()).unwrap();
    let host = runner(path.clone());
    assert_eq!(host.active_slot, Some(2));
    assert_eq!(host.level_tally, 0x23);
    assert_eq!(host.slots.slot(2), Some(&record));
    for bytes in [b"bad".as_slice(), &[0; SLOT_LEN]] {
        fs::write(&path, bytes).unwrap();
        let mut host = runner(path.clone());
        assert!(!host.load_from_disk());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    fs::remove_file(path).unwrap();
}
