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

struct HeaderOverlay {
    header: Result<Vec<u8>, io::ErrorKind>,
    base: ExtractedContent,
}

impl ContentSource for HeaderOverlay {
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        if path == "cartridge_header.bin" {
            self.header.clone().map_err(Into::into)
        } else {
            self.base.read(path)
        }
    }
}

fn header_source(header: Result<Vec<u8>, io::ErrorKind>) -> HeaderOverlay {
    HeaderOverlay {
        header,
        base: ExtractedContent::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"),
        ),
    }
}

#[test]
fn palette_header_metadata_is_loaded_before_session_ticks() {
    // Content bundles may override cartridge identity without a ROM at runtime.
    let header = [0x19; 24];
    let assets = SessionAssets::load_from(&header_source(Ok(header.to_vec()))).unwrap();
    assert_eq!(assets.cartridge_header, header);
}

#[test]
fn absent_palette_metadata_uses_verified_legacy_sml2_header() {
    let assets = SessionAssets::load_from(&header_source(Err(io::ErrorKind::NotFound))).unwrap();
    let expected = [
        0x4D, 0x41, 0x52, 0x49, 0x4F, 0x4C, 0x41, 0x4E, 0x44, 0x32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
        4, 2, 1, 1,
    ];
    assert_eq!(assets.cartridge_header, expected);
    assert_eq!(
        sml2_core::hardware_palettes::gbc_auto(&assets.cartridge_header).bg,
        [0x67FF, 0x77AC, 0x1A13, 0x2D6B]
    );
}

#[test]
fn malformed_or_unreadable_palette_metadata_is_an_error() {
    for size in [0, 23, 25] {
        let error = SessionAssets::load_from(&header_source(Ok(vec![0; size])))
            .err()
            .unwrap();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
    }
    for kind in [
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::UnexpectedEof,
    ] {
        let error = SessionAssets::load_from(&header_source(Err(kind)))
            .err()
            .unwrap();
        assert_eq!(error.downcast_ref::<io::Error>().unwrap().kind(), kind);
    }
}
