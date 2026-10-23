//! What an enlarged level view draws. Inside the layout the real cell is
//! sampled, with no window wrap. Past the layout a paint may cover the cell;
//! otherwise it stays blank.

use sml2_core::level::{LAYOUT_HEIGHT, LAYOUT_WIDTH};
use sml2_core::scenery::SceneryOverlay;

use super::view;

/// How to draw one pixel of an enlarged level view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarginSample {
    /// The original window path. Used for a 160×144 view.
    Native,
    /// The real layout pixel at this world position.
    Level { wx: i32, wy: i32 },
    /// Past the level, with nothing painted.
    Blank,
    /// A painted metatile past the level. `(tx, ty)` is the pixel inside that cell.
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
    if !view::enlarged(view) {
        return MarginSample::Native;
    }
    let world_x = cam.0 as i32 - view.0 as i32 / 2 + px;
    let world_y = cam.1 as i32 - view.1 as i32 / 2 - view::level_bias(view) - scroll_pan + py;
    let (cx, cy) = (world_x.div_euclid(16), world_y.div_euclid(16));
    if (0..LAYOUT_WIDTH as i32).contains(&cx) && (0..LAYOUT_HEIGHT as i32).contains(&cy) {
        return MarginSample::Level {
            wx: world_x,
            wy: world_y,
        };
    }
    if let Some(metatile) = overlay.get(cx, cy) {
        return MarginSample::Paint {
            metatile,
            tx: world_x.rem_euclid(16) as usize,
            ty: world_y.rem_euclid(16) as usize,
        };
    }
    MarginSample::Blank
}

/// The layout cell under a view pixel when that cell lies past the 256×48
/// level. Those are the cells the scenery editor can paint. Cells Mario can
/// walk on are inside the level and are not paintable.
pub fn paintable_cell(
    view: (usize, usize),
    px: i32,
    py: i32,
    cam: (u16, u16),
    scroll_pan: i32,
) -> Option<(i16, i16)> {
    if !matches!(
        margin_sample(view, px, py, cam, scroll_pan, &SceneryOverlay::empty()),
        MarginSample::Blank
    ) {
        return None;
    }
    let world_x = cam.0 as i32 - view.0 as i32 / 2 + px;
    let world_y = cam.1 as i32 - view.1 as i32 / 2 - view::level_bias(view) - scroll_pan + py;
    let (cx, cy) = (world_x.div_euclid(16), world_y.div_euclid(16));
    i16::try_from(cx).ok().zip(i16::try_from(cy).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sml2_core::scenery::{SceneryOverlay, SceneryPaint};

    #[test]
    fn native_views_use_the_window_path_and_enlarged_views_sample_real_level_cells() {
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
            // Enlarged views draw the real layout even inside the native
            // picture. The existing paint must not cover this terrain cell.
            MarginSample::Level { wx: 0, wy: 376 }
        );
    }

    #[test]
    fn inside_the_level_is_the_real_cell_and_past_it_is_blank_until_painted() {
        // Camera at the left edge: view x 0 is world x -112, past the level.
        // View x 120 is world x 8, inside the level.
        let cam = (80u16, 448u16);
        let view = (384usize, 216usize);
        assert_eq!(
            margin_sample(view, 0, 76, cam, 0, &SceneryOverlay::empty()),
            MarginSample::Blank
        );
        assert_eq!(
            margin_sample(view, 120, 76, cam, 0, &SceneryOverlay::empty()),
            MarginSample::Level { wx: 8, wy: 398 }
        );
        let painted = SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: -7,
                y: 24,
                metatile: 4,
            }],
            8,
        )
        .unwrap();
        assert_eq!(
            margin_sample(view, 0, 76, cam, 0, &painted),
            MarginSample::Paint {
                metatile: 4,
                tx: 0,
                ty: 14,
            }
        );
        // A paint inside the level does not cover the real cell.
        let inside = SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: 0,
                y: 24,
                metatile: 4,
            }],
            8,
        )
        .unwrap();
        assert_eq!(
            margin_sample(view, 120, 76, cam, 0, &inside),
            MarginSample::Level { wx: 8, wy: 398 }
        );
        assert_eq!(paintable_cell(view, 0, 76, cam, 0), Some((-7, 24)));
        assert_eq!(paintable_cell(view, 120, 76, cam, 0), None);
        assert_eq!(paintable_cell((160, 144), 0, 76, cam, 0), None);
    }
}
