use super::*;
use crate::{
    content::{ContentSource, ExtractedContent},
    session::SessionState,
    FrameInput, Session,
};
use sml2_core::player_sim::PlayerKeys;
use std::{cell::Cell, io, path::PathBuf};

const EXPORTED_LEVEL: &str =
    r#"{"paints":[{"x":-2,"y":48,"metatile":1},{"x":0,"y":25,"metatile":2}]}"#;

struct Source {
    base: ExtractedContent,
    reads: Cell<usize>,
}

impl Source {
    fn new() -> Self {
        Self {
            base: ExtractedContent::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"),
            ),
            reads: Cell::new(0),
        }
    }
}

impl ContentSource for Source {
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        self.reads.set(self.reads.get() + 1);
        self.base.read(path)
    }
}

fn advance(session: &mut Session, keys: PlayerKeys, frames: usize) {
    for _ in 0..frames {
        session.advance(FrameInput { keys, div: 0 });
    }
}

#[test]
fn exported_bundled_scenery_loads_through_every_pipe_without_tick_reads() {
    for slot in 0..3 {
        let source = Source::new();
        let assets =
            SessionAssets::load_from_with_scenery(&source, &[(0, EXPORTED_LEVEL)]).unwrap();
        assert_eq!(assets.level(0).unwrap().scenery.get(-2, 48), Some(1));
        let reads = source.reads.get();
        let mut session = Session::new(assets);
        session.enter_file_select();
        advance(&mut session, PlayerKeys::default(), 64);
        for _ in 0..slot {
            advance(
                &mut session,
                PlayerKeys {
                    right: true,
                    ..Default::default()
                },
                1,
            );
            advance(&mut session, PlayerKeys::default(), 40);
        }
        advance(
            &mut session,
            PlayerKeys {
                down: true,
                ..Default::default()
            },
            1,
        );
        advance(&mut session, PlayerKeys::default(), 100);
        assert_eq!(session.active_slot, Some(slot));
        assert!(matches!(session.state, SessionState::Overworld(_)));
        for _ in 0..2 {
            session.begin_level(0, 0).unwrap();
            advance(&mut session, PlayerKeys::default(), 3);
            let SessionState::InLevel(level) = &session.state else {
                panic!("level 0 must load");
            };
            assert_eq!(level.scenery.get(-2, 48), Some(1));
            assert_eq!(level.scenery.get(0, 25), Some(2));
            session.render_palette_frame((384, 216));
        }
        assert_eq!(
            source.reads.get(),
            reads,
            "simulation and rendering must not read files"
        );
    }
}

#[test]
fn scenery_changes_only_presentation_content() {
    let source = Source::new();
    let original = SessionAssets::load_from_with_scenery(&source, &[]).unwrap();
    let decorated = SessionAssets::load_from_with_scenery(&source, &[(0, EXPORTED_LEVEL)]).unwrap();
    let a = original.level(0).unwrap();
    let b = decorated.level(0).unwrap();
    assert_eq!(a.layout.cells, b.layout.cells);
    assert_eq!(a.objblock, b.objblock);
    assert_eq!(a.bg_base, b.bg_base);
    assert_eq!(a.manifest.metatile_classes, b.manifest.metatile_classes);
    let mut a = Session::new(original);
    let mut b = Session::new(decorated);
    for session in [&mut a, &mut b] {
        session.begin_level(0, 0).unwrap();
        advance(
            session,
            PlayerKeys {
                right: true,
                ..Default::default()
            },
            20,
        );
    }
    let (SessionState::InLevel(a_level), SessionState::InLevel(b_level)) = (&a.state, &b.state)
    else {
        panic!("both sessions must remain in the level");
    };
    assert_eq!(a_level.sim.x.to_le_bytes(), b_level.sim.x.to_le_bytes());
    assert_eq!(a_level.sim.y.to_le_bytes(), b_level.sim.y.to_le_bytes());
    assert_eq!(a_level.sim.layout_writes, b_level.sim.layout_writes);
    assert_eq!(a.save_record(), b.save_record());
}

#[test]
fn invalid_approved_content_is_a_startup_error() {
    let source = Source::new();
    for (entries, expected) in [
        (vec![(0, "broken json")], "bundled scenery for level 0"),
        (
            vec![(0, r#"{"paints":[{"x":0,"y":0,"metatile":255}]}"#)],
            "metatile 255",
        ),
        (
            vec![(
                0,
                r#"{"paints":[{"x":0,"y":0,"metatile":1},{"x":0,"y":0,"metatile":2}]}"#,
            )],
            "duplicated",
        ),
        (
            vec![(0, EXPORTED_LEVEL), (0, EXPORTED_LEVEL)],
            "registered twice",
        ),
        (vec![(32, EXPORTED_LEVEL)], "outside 0..32"),
    ] {
        let error = SessionAssets::load_from_with_scenery(&source, &entries)
            .err()
            .unwrap();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn registry_validation_never_partially_installs_overlays() {
    let mut assets = SessionAssets::load_from_with_scenery(&Source::new(), &[]).unwrap();
    assert!(apply(&mut assets, &[(0, EXPORTED_LEVEL), (1, "broken json")]).is_err());
    assert!(assets.level(0).unwrap().scenery.is_empty());
}

#[test]
fn shipping_registry_loads_from_the_public_entry_point() {
    SessionAssets::load_from(&Source::new()).unwrap();
}

/// Expanded scenery is authored on the rebuilt map, so the rebuild uses it
/// as it is instead of moving the original level's scenery.
#[test]
fn expanded_scenery_replaces_the_moved_scenery_of_a_rebuilt_level() {
    let mut assets =
        SessionAssets::load_from_with_scenery(&Source::new(), &[(2, EXPORTED_LEVEL)]).unwrap();
    let gap = r#"{"paints":[{"x":80,"y":5,"metatile":3}]}"#;
    apply_expanded(&mut assets, &[(2, gap)]).unwrap();
    let level = assets.level(2).unwrap();
    assert_eq!(
        level.scenery.get(0, 25),
        Some(2),
        "the original keeps its own"
    );
    let rebuilt = assets.expanded_level(level).unwrap().unwrap();
    assert_eq!(rebuilt.scenery.get(80, 5), Some(3));
    assert_eq!(
        rebuilt.scenery.len(),
        1,
        "nothing moved in from the original"
    );

    let error = apply_expanded(&mut assets, &[(0, gap)]).unwrap_err();
    assert!(error.contains("level 0 has no recipe"), "{error}");
    assert!(assets.level(0).unwrap().expanded_scenery.is_none());
}

/// Scenery for a level that did not load leaves that level's own entry
/// error in place instead of failing the whole boot.
#[test]
fn scenery_for_a_level_that_did_not_load_is_skipped() {
    struct Missing(Source);
    impl ContentSource for Missing {
        fn read(&self, path: &str) -> io::Result<Vec<u8>> {
            match path {
                "levels/level02.manifest.json" => Err(io::ErrorKind::NotFound.into()),
                _ => self.0.read(path),
            }
        }
    }
    let assets =
        SessionAssets::load_from_with_scenery(&Missing(Source::new()), &[(1, EXPORTED_LEVEL)])
            .unwrap();
    assert!(assets.level(1).is_err());
}
