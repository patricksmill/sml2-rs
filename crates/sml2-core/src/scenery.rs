//! Render-only scenery painted outside the original 160×144 level picture.

use serde::{Deserialize, Serialize};

/// One metatile painted into the margin of an enlarged level view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneryPaint {
    pub x: u8,
    pub y: u8,
    pub metatile: u8,
}

/// Sparse paints for one level. Empty for every level loaded from the ROM.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneryOverlay {
    pub paints: Vec<SceneryPaint>,
}

impl SceneryOverlay {
    pub fn empty() -> Self {
        Self { paints: Vec::new() }
    }

    /// Rejects a row outside the 48-row layout, a metatile the level does not
    /// have, and a second paint on the same cell. The message names the cell.
    pub fn try_from_paints(
        paints: Vec<SceneryPaint>,
        metatile_count: usize,
    ) -> Result<Self, String> {
        let mut seen = vec![false; 256 * 48];
        for paint in &paints {
            if paint.y >= 48 {
                return Err(format!(
                    "scenery paint ({}, {}) is outside the layout",
                    paint.x, paint.y
                ));
            }
            if usize::from(paint.metatile) >= metatile_count {
                return Err(format!(
                    "scenery paint ({}, {}) metatile {} is not in the level",
                    paint.x, paint.y, paint.metatile
                ));
            }
            let cell = usize::from(paint.y) * 256 + usize::from(paint.x);
            if seen[cell] {
                return Err(format!(
                    "scenery paint ({}, {}) is duplicated",
                    paint.x, paint.y
                ));
            }
            seen[cell] = true;
        }
        Ok(Self { paints })
    }

    /// Replace the paint on this cell. A bad cell or metatile leaves the
    /// overlay unchanged.
    pub fn set(&mut self, paint: SceneryPaint, metatile_count: usize) -> Result<(), String> {
        let mut paints: Vec<_> = self
            .paints
            .iter()
            .copied()
            .filter(|p| (p.x, p.y) != (paint.x, paint.y))
            .collect();
        paints.push(paint);
        *self = Self::try_from_paints(paints, metatile_count)?;
        Ok(())
    }

    pub fn erase(&mut self, x: u8, y: u8) {
        self.paints.retain(|p| (p.x, p.y) != (x, y));
    }

    /// The metatile painted at layout cell `(x, y)`.
    pub fn get(&self, x: usize, y: usize) -> Option<u8> {
        self.paints
            .iter()
            .find(|p| usize::from(p.x) == x && usize::from(p.y) == y)
            .map(|p| p.metatile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_row_past_the_layout_an_unknown_metatile_and_a_duplicate() {
        let row = SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: 1,
                y: 48,
                metatile: 0,
            }],
            128,
        );
        assert_eq!(
            row.unwrap_err(),
            "scenery paint (1, 48) is outside the layout"
        );
        let tile = SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: 1,
                y: 2,
                metatile: 200,
            }],
            128,
        );
        assert_eq!(
            tile.unwrap_err(),
            "scenery paint (1, 2) metatile 200 is not in the level"
        );
        let dup = SceneryOverlay::try_from_paints(
            vec![
                SceneryPaint {
                    x: 1,
                    y: 2,
                    metatile: 3,
                },
                SceneryPaint {
                    x: 1,
                    y: 2,
                    metatile: 4,
                },
            ],
            128,
        );
        assert_eq!(dup.unwrap_err(), "scenery paint (1, 2) is duplicated");
    }

    #[test]
    fn set_replaces_a_cell_and_erase_removes_it() {
        let mut overlay = SceneryOverlay::empty();
        overlay
            .set(
                SceneryPaint {
                    x: 7,
                    y: 24,
                    metatile: 1,
                },
                128,
            )
            .unwrap();
        overlay
            .set(
                SceneryPaint {
                    x: 7,
                    y: 24,
                    metatile: 4,
                },
                128,
            )
            .unwrap();
        assert_eq!(overlay.get(7, 24), Some(4));
        assert!(overlay
            .set(
                SceneryPaint {
                    x: 1,
                    y: 48,
                    metatile: 1,
                },
                128,
            )
            .is_err());
        assert_eq!(overlay.get(7, 24), Some(4));
        overlay.erase(7, 24);
        assert_eq!(overlay.get(7, 24), None);
    }

    #[test]
    fn a_round_trip_keeps_the_paint() {
        let overlay = SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: 1,
                y: 20,
                metatile: 12,
            }],
            128,
        )
        .unwrap();
        let json = serde_json::to_string(&overlay).unwrap();
        let back: SceneryOverlay = serde_json::from_str(&json).unwrap();
        assert_eq!(back.get(1, 20), Some(12));
        assert_eq!(back.get(0, 0), None);
    }
}
