//! Authored scenery overlays native and enlarged views without changing physics.
//! Enlarged views sample unpainted terrain directly and stay blank past the map.

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
    /// A painted visual metatile. `(tx, ty)` is the pixel inside that cell.
    Paint { metatile: u8, tx: usize, ty: usize },
}

/// Resolve sparse lookups once per visible metatile, shared by both renderers.
pub(crate) struct SceneryWindow {
    view: (usize, usize),
    origin: (i32, i32),
    first: (i32, i32),
    width: usize,
    paints: Vec<Option<u8>>,
    rewrites: Vec<Option<u8>>,
}
impl SceneryWindow {
    pub fn new(
        view: (usize, usize),
        cam: (u16, u16),
        pan: i32,
        border: i32,
        overlay: &SceneryOverlay,
    ) -> Self {
        let origin = (
            cam.0 as i32 - view.0 as i32 / 2,
            cam.1 as i32 - view.1 as i32 / 2 - view::level_bias(view) - pan,
        );
        let first = (
            (origin.0 - border).div_euclid(16),
            (origin.1 - border).div_euclid(16),
        );
        let last = (
            (origin.0 + view.0 as i32 + border - 1).div_euclid(16),
            (origin.1 + view.1 as i32 + border - 1).div_euclid(16),
        );
        let width = (last.0 - first.0 + 1) as usize;
        let mut paints = Vec::with_capacity(width * (last.1 - first.1 + 1) as usize);
        for y in first.1..=last.1 {
            for x in first.0..=last.0 {
                paints.push(overlay.get(x, y));
            }
        }
        Self {
            view,
            origin,
            first,
            width,
            rewrites: vec![None; paints.len()],
            paints,
        }
    }
    /// Live block writes supersede cosmetic paints, without mutating authoring data.
    pub fn with_layout_writes(mut self, writes: &[(u32, u8)]) -> Self {
        let height = self.paints.len() / self.width;
        for &(key, value) in writes {
            let (x, y) = (
                (key % LAYOUT_WIDTH as u32) as i32,
                (key / LAYOUT_WIDTH as u32) as i32,
            );
            if y >= LAYOUT_HEIGHT as i32 {
                continue;
            }
            let (x, y) = (x - self.first.0, y - self.first.1);
            if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < height {
                let at = y as usize * self.width + x as usize;
                // Match the existing first-write lookup contract.
                if self.rewrites[at].is_none() {
                    self.rewrites[at] = Some(value);
                }
                self.paints[at] = None;
            }
        }
        self
    }
    pub fn rewrite(&self, px: i32, py: i32) -> Option<u8> {
        let x = (self.origin.0 + px).div_euclid(16) - self.first.0;
        let y = (self.origin.1 + py).div_euclid(16) - self.first.1;
        self.rewrites[y as usize * self.width + x as usize]
    }
    pub fn sample(&self, px: i32, py: i32) -> MarginSample {
        let (world_x, world_y) = (self.origin.0 + px, self.origin.1 + py);
        let (cx, cy) = (world_x.div_euclid(16), world_y.div_euclid(16));
        let paint =
            self.paints[(cy - self.first.1) as usize * self.width + (cx - self.first.0) as usize];
        let view = self.view;
        if let Some(metatile) = paint {
            return MarginSample::Paint {
                metatile,
                tx: world_x.rem_euclid(16) as usize,
                ty: world_y.rem_euclid(16) as usize,
            };
        }
        if !view::enlarged(view) {
            return MarginSample::Native;
        }
        if (0..LAYOUT_WIDTH as i32).contains(&cx) && (0..LAYOUT_HEIGHT as i32).contains(&cy) {
            return MarginSample::Level {
                wx: world_x,
                wy: world_y,
            };
        }
        MarginSample::Blank
    }
}
#[cfg(test)]
fn margin_sample(
    view: (usize, usize),
    px: i32,
    py: i32,
    cam: (u16, u16),
    pan: i32,
    overlay: &SceneryOverlay,
) -> MarginSample {
    SceneryWindow::new(view, cam, pan, 0, overlay).sample(px, py)
}

/// Signed layout cell under a view pixel. Scenery can cover cells both inside
/// and outside the map without changing the collision layout.
pub fn paintable_cell(
    view: (usize, usize),
    px: i32,
    py: i32,
    cam: (u16, u16),
    scroll_pan: i32,
) -> Option<(i16, i16)> {
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
        // Scenery can fill cells inside the map as a render-only overlay.
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
            MarginSample::Paint {
                metatile: 4,
                tx: 8,
                ty: 14
            }
        );
        assert_eq!(paintable_cell(view, 0, 76, cam, 0), Some((-7, 24)));
        assert_eq!(paintable_cell(view, 120, 76, cam, 0), Some((0, 24)));
        assert_eq!(paintable_cell((160, 144), 0, 76, cam, 0), Some((0, 28)));
    }
}
