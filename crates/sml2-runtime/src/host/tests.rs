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
