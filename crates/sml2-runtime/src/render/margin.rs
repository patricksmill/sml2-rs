//! What an enlarged level view draws outside the original 160×144 picture.

use sml2_core::level::{LAYOUT_HEIGHT, LAYOUT_WIDTH};
use sml2_core::scenery::SceneryOverlay;

use super::view::{self, NATIVE};

const PLAYFIELD_H: i32 = 136;

/// How to draw one pixel of an enlarged level view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarginSample {
    /// The original window path, including the centred 160×144 picture.
    Native,
    /// Copy the native playfield pixel at `(nx, ny)` in view coordinates.
    Edge { nx: i32, ny: i32 },
    /// A painted metatile. `(tx, ty)` is the pixel inside that 16×16 cell.
    Paint { metatile: u8, tx: usize, ty: usize },
}

pub(crate) fn margin_sample(
    view: (usize, usize),
    px: i32,
    py: i32,
    cam: (u16, u16),
    scroll_pan: i32,
    overlay: &SceneryOverlay,
) -> MarginSample {
    let (mx, my) = view::level_margin(view);
    let in_native = px >= mx && px < mx + NATIVE.0 as i32 && py >= my && py < my + NATIVE.1 as i32;
    if !view::enlarged(view) || in_native {
        return MarginSample::Native;
    }
    let world_x = cam.0 as i32 - view.0 as i32 / 2 + px;
    let world_y = cam.1 as i32 - view.1 as i32 / 2 - view::level_bias(view) - scroll_pan + py;
    let (cx, cy) = (world_x.div_euclid(16), world_y.div_euclid(16));
    if (0..LAYOUT_WIDTH as i32).contains(&cx) && (0..LAYOUT_HEIGHT as i32).contains(&cy) {
        if let Some(metatile) = overlay.get(cx as usize, cy as usize) {
            return MarginSample::Paint {
                metatile,
                tx: world_x.rem_euclid(16) as usize,
                ty: world_y.rem_euclid(16) as usize,
            };
        }
    }
    MarginSample::Edge {
        nx: px.clamp(mx, mx + NATIVE.0 as i32 - 1),
        ny: py.clamp(my, my + PLAYFIELD_H - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sml2_core::scenery::{SceneryOverlay, SceneryPaint};

    #[test]
    fn the_native_picture_and_a_native_view_stay_on_the_window_path() {
        let overlay = SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: 0,
                y: 0,
                metatile: 1,
            }],
            8,
        )
        .unwrap();
        assert_eq!(
            margin_sample((160, 144), 0, 0, (80, 448), 0, &overlay),
            MarginSample::Native
        );
        assert_eq!(
            margin_sample((384, 216), 112, 54, (80, 448), 0, &overlay),
            MarginSample::Native
        );
    }

    #[test]
    fn past_the_picture_copies_the_playfield_edge_unless_that_cell_is_painted() {
        let cam = (200u16, 448u16);
        let view = (384usize, 216usize);
        assert_eq!(
            margin_sample(view, 0, 76, cam, 0, &SceneryOverlay::empty()),
            MarginSample::Edge { nx: 112, ny: 76 }
        );
        assert_eq!(
            margin_sample(view, 200, 200, cam, 0, &SceneryOverlay::empty()),
            MarginSample::Edge { nx: 200, ny: 189 }
        );
        // Native left is world x 120. View x 111 is world x 119, cell (7, 24).
        let painted = SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: 7,
                y: 24,
                metatile: 4,
            }],
            8,
        )
        .unwrap();
        assert_eq!(
            margin_sample(view, 111, 76, cam, 0, &painted),
            MarginSample::Paint {
                metatile: 4,
                tx: 7,
                ty: 14,
            }
        );
    }
}
