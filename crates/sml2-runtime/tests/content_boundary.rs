use std::{cell::Cell, io, path::PathBuf};

use sml2_core::{level::LevelManifest, player_sim::PlayerKeys};
use sml2_runtime::{
    content::{ContentSource, ExtractedContent, SessionAssets},
    session::{SessionRunner, SessionState, StepOutcome, MAP_ENTRY_FRAMES},
};

/// An importer/editor may overlay one asset without writing an extracted tree.
struct Overlay {
    base: ExtractedContent,
    manifest: Vec<u8>,
    reads: Cell<usize>,
}

impl ContentSource for Overlay {
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        self.reads.set(self.reads.get() + 1);
        match path {
            "levels/level01.manifest.json" => Ok(self.manifest.clone()),
            "levels/level02.manifest.json" => Err(io::ErrorKind::NotFound.into()),
            _ => self.base.read(path),
        }
    }
}

#[test]
fn injected_content_reaches_level_entry_without_reopening_the_source() {
    let base =
        ExtractedContent::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"));
    let mut manifest: LevelManifest =
        serde_json::from_slice(&base.read("levels/level01.manifest.json").unwrap()).unwrap();
    manifest.header.mario_spawn_x = 0x0123;
    let source = Overlay {
        base,
        manifest: serde_json::to_vec(&manifest).unwrap(),
        reads: Cell::new(0),
    };
    let assets = SessionAssets::load_from(&source).unwrap();
    // A missing unrelated level is cached as an entry error, not a boot failure.
    assert!(assets.level(1).is_err());
    let reads = source.reads.get();
    let save =
        std::env::temp_dir().join(format!("sml2_content_boundary_{}.sav", std::process::id()));
    let _ = std::fs::remove_file(&save);
    let mut runner = SessionRunner::new(assets, save, 1, (160, 144), true);
    runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    let mut entered = false;
    for _ in 0..MAP_ENTRY_FRAMES + 8 {
        if runner.step_frame(PlayerKeys::default()) == StepOutcome::LevelEntered(0) {
            entered = true;
            break;
        }
    }
    assert!(entered);
    let SessionState::InLevel(level) = &runner.state else {
        panic!("expected level")
    };
    assert_eq!(level.sim.x.to_le_bytes(), [0x23, 0x01]);
    assert_eq!(source.reads.get(), reads, "ticking must not read content");
}
