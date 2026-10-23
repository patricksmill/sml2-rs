//! Render-only scenery, independent of the playable level layout and collision.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// One visual metatile overlay. Cell coordinates are in layout cells
/// and may be negative or past the 256×48 map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneryPaint {
    pub x: i16,
    pub y: i16,
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

    /// Rejects a metatile the level does not have, and a second paint on the
    /// same cell. The message names the cell. Cells past the 256×48 layout
    /// are the ones an enlarged view can show beyond the level.
    pub fn try_from_paints(
        paints: Vec<SceneryPaint>,
        metatile_count: usize,
    ) -> Result<Self, String> {
        let mut seen = HashSet::new();
        for paint in &paints {
            if usize::from(paint.metatile) >= metatile_count {
                return Err(format!(
                    "scenery paint ({}, {}) metatile {} is not in the level",
                    paint.x, paint.y, paint.metatile
                ));
            }
            if !seen.insert((paint.x, paint.y)) {
                return Err(format!(
                    "scenery paint ({}, {}) is duplicated",
                    paint.x, paint.y
                ));
            }
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

    pub fn erase(&mut self, x: i16, y: i16) {
        self.paints.retain(|p| (p.x, p.y) != (x, y));
    }

    /// The metatile painted at layout cell `(x, y)`, which may lie past the map.
    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        self.paints
            .iter()
            .find(|p| i32::from(p.x) == x && i32::from(p.y) == y)
            .map(|p| p.metatile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_cell_past_the_layout_and_rejects_a_bad_metatile_or_duplicate() {
        let past = SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: -1,
                y: 48,
                metatile: 0,
            }],
            128,
        )
        .unwrap();
        assert_eq!(past.get(-1, 48), Some(0));
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
                    x: -1,
                    y: 48,
                    metatile: 200,
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
