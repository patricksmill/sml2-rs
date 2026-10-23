use super::state::*;
use eframe::egui::{self, Pos2, Rect};
use sml2_core::level::{LevelLayout, LAYOUT_SIZE};

mod persistence;
mod preview;
mod rendering;
mod workspace_ui;

fn workspace() -> Workspace {
    let mut work = Workspace::new(
        LevelLayout {
            cells: vec![0; LAYOUT_SIZE],
        },
        128,
        Pos2::ZERO,
    );
    work.metatile = 1;
    work
}

#[test]
fn signed_cells_and_pointer_anchored_zoom_stay_aligned() {
    let rect = Rect::from_min_size(egui::pos2(80.0, 30.0), egui::vec2(800.0, 500.0));
    let mut view = Viewport {
        center: Pos2::ZERO,
        zoom: 2,
    };
    let pointer = rect.center() - egui::vec2(1.0, 33.0);
    assert_eq!(view.cell(rect, pointer), Some((-1, -2)));
    let world = view.world(rect, pointer);
    view.zoom_at(rect, pointer, 8);
    assert_eq!(view.world(rect, pointer), world);
    assert_eq!(view.screen(rect, world), pointer);
    view.pan(egui::vec2(128.0, 64.0));
    assert_eq!(view.world(rect, pointer), world - egui::vec2(16.0, 8.0));
}

#[test]
fn fast_strokes_interpolate_and_undo_as_one_gesture() {
    let mut work = workspace();
    work.begin((-10, 24), false);
    work.extend(Some((-1, 24)));
    assert!(!work.dirty, "saving waits for release");
    assert!(work.finish());
    for x in -10..0 {
        assert_eq!(work.sample((x, 24)), Some(1));
    }
    work.undo();
    assert!(work.scenery.paints.is_empty());
    work.redo();
    assert_eq!(work.scenery.paints.len(), 10);
}

#[test]
fn rectangle_skips_the_map_and_cancellation_keeps_existing_scenery() {
    let mut work = workspace();
    work.tool = Tool::Rectangle;
    work.begin((-2, 0), false);
    work.extend(Some((2, 1)));
    assert!(
        work.scenery.paints.is_empty(),
        "rectangle is a preview until release"
    );
    work.cancel();
    assert!(!work.can_undo());
    work.begin((-2, 0), false);
    work.extend(Some((2, 1)));
    work.finish();
    assert_eq!(work.scenery.paints.len(), 4);
    assert_eq!(work.sample((0, 0)), Some(0));
    work.begin((-1, 0), true);
    assert_eq!(work.sample((-1, 0)), None);
    work.cancel();
    assert_eq!(work.sample((-1, 0)), Some(1));
    work.undo();
    assert!(work.scenery.paints.is_empty());
}

#[test]
fn eyedropper_and_eraser_handle_blank_and_protected_cells() {
    let mut work = workspace();
    work.begin((-1, 0), false);
    work.finish();
    work.tool = Tool::Eyedropper;
    work.metatile = 9;
    work.begin((-2, 0), false);
    assert_eq!(work.metatile, 9);
    work.begin((-1, 0), false);
    assert_eq!(work.metatile, 1);
    work.begin((0, 0), false);
    assert_eq!(work.metatile, 0);
    work.tool = Tool::Eraser;
    work.begin((-1, 0), false);
    work.extend(Some((5, 0)));
    work.finish();
    assert!(work.scenery.paints.is_empty());
    assert_eq!(work.layout.cells, vec![0; LAYOUT_SIZE]);
    work.undo();
    assert_eq!(work.sample((-1, 0)), Some(1));
}

#[test]
fn no_ops_preserve_redo_and_new_edits_clear_it() {
    let mut work = workspace();
    work.begin((-1, 0), false);
    work.finish();
    work.undo();
    work.begin((0, 0), false);
    assert!(!work.finish());
    assert!(work.can_redo());
    work.begin((-2, 0), false);
    work.finish();
    assert!(!work.can_redo());
    work.dirty = false;
    work.begin((-2, 0), false);
    assert!(!work.finish());
    assert!(!work.dirty);
}

#[test]
fn history_retains_the_latest_hundred_gestures() {
    let mut work = workspace();
    for y in 0..110 {
        work.begin((-1, y), false);
        work.finish();
    }
    for _ in 0..110 {
        work.undo();
    }
    assert_eq!(work.scenery.paints.len(), 10);
    for y in 0..10 {
        assert_eq!(work.sample((-1, y)), Some(1));
    }
}
