//! Indexed render-only scenery.
//!
//! Files group the painted cells into rectangles, each tiled with a small
//! pattern ([`SceneryFill`], format [`SCENERY_FORMAT`]); the first format,
//! one `{"paints": [...]}` entry per cell, still loads.
use crate::level::{LAYOUT_HEIGHT, LAYOUT_WIDTH};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
const EMPTY: u16 = 256;
const SIDE: usize = 16;

/// The grouped file format every writer uses.
pub const SCENERY_FORMAT: &str = "sml2-scenery/2";

/// Most cells one file may describe, so a damaged size cannot exhaust memory.
const MAX_FILL_CELLS: u64 = 1 << 24;

/// Pattern sizes the writer tries, smallest first so a tie keeps the simpler.
const PATTERNS: [(i16, i16); 8] = [
    (1, 1),
    (2, 1),
    (1, 2),
    (2, 2),
    (3, 1),
    (4, 1),
    (3, 2),
    (4, 2),
];

/// A rectangle of `size` cells from `at`, tiled with `tiles` (rows of
/// metatile ids) from its top-left cell. Later fills cover earlier ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneryFill {
    pub at: [i16; 2],
    pub size: [u16; 2],
    pub tiles: Vec<Vec<u8>>,
}

impl SceneryFill {
    /// Each cell the fill paints, with its metatile.
    fn cells(&self) -> Result<impl Iterator<Item = SceneryPaint> + '_, String> {
        let [x, y] = self.at;
        let [w, h] = self.size;
        let pattern_width = self.tiles.first().map_or(0, Vec::len);
        if w == 0
            || h == 0
            || pattern_width == 0
            || self.tiles.iter().any(|row| row.len() != pattern_width)
        {
            return Err(format!(
                "scenery fill at ({x}, {y}): it needs a size and equal, non-empty tile rows"
            ));
        }
        if i32::from(x) + i32::from(w) - 1 > i32::from(i16::MAX)
            || i32::from(y) + i32::from(h) - 1 > i32::from(i16::MAX)
        {
            return Err(format!("scenery fill at ({x}, {y}) runs past the canvas"));
        }
        Ok((0..h).flat_map(move |dy| {
            (0..w).map(move |dx| SceneryPaint {
                x: x + dx as i16,
                y: y + dy as i16,
                metatile: self.tiles[usize::from(dy) % self.tiles.len()]
                    [usize::from(dx) % pattern_width],
            })
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneryPaint {
    pub x: i16,
    pub y: i16,
    pub metatile: u8,
}

#[derive(Debug, Clone)]
struct Chunk {
    cells: Box<[u16; 256]>,
    used: usize,
}

/// Lazily allocated 24 KiB map grid plus sparse 16×16 chunks outside it.
/// All mutations pass through set/erase so the index cannot become stale.
#[derive(Debug, Clone, Default)]
pub struct SceneryOverlay {
    dense: Option<Box<[u16]>>,
    chunks: BTreeMap<(i16, i16), Chunk>,
    len: usize,
}
/// Either file format: `format` + `fills`, or the first format's `paints`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneryData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fills: Vec<SceneryFill>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    paints: Vec<SceneryPaint>,
}
impl Serialize for SceneryOverlay {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SceneryData {
            format: Some(SCENERY_FORMAT.into()),
            fills: self.fills(),
            paints: Vec::new(),
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for SceneryOverlay {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SceneryData::deserialize(deserializer)?;
        match data.format.as_deref() {
            None if data.fills.is_empty() => Self::try_from_paints(data.paints, 256),
            Some(SCENERY_FORMAT) if data.paints.is_empty() => {
                Self::try_from_fills(&data.fills, 256)
            }
            Some(format) if format != SCENERY_FORMAT => Err(format!(
                "scenery format {format:?} is not {SCENERY_FORMAT:?}"
            )),
            _ => Err("scenery mixes `paints` and `fills`".into()),
        }
        .map_err(serde::de::Error::custom)
    }
}
impl PartialEq for SceneryOverlay {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self
                .paints()
                .iter()
                .all(|p| other.get(p.x.into(), p.y.into()) == Some(p.metatile))
    }
}
impl Eq for SceneryOverlay {}
impl SceneryOverlay {
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn try_from_paints(paints: Vec<SceneryPaint>, count: usize) -> Result<Self, String> {
        let mut result = Self::empty();
        for p in paints {
            if result.get(p.x.into(), p.y.into()).is_some() {
                return Err(format!("scenery paint ({}, {}) is duplicated", p.x, p.y));
            }
            result.set(p, count)?;
        }
        Ok(result)
    }
    /// Paint `fills` in order; a later fill covers an earlier one's cells.
    pub fn try_from_fills(fills: &[SceneryFill], count: usize) -> Result<Self, String> {
        let total: u64 = fills
            .iter()
            .map(|f| u64::from(f.size[0]) * u64::from(f.size[1]))
            .sum();
        if total > MAX_FILL_CELLS {
            return Err(format!(
                "scenery fills describe {total} cells, over {MAX_FILL_CELLS}"
            ));
        }
        let mut result = Self::empty();
        for fill in fills {
            for paint in fill.cells()? {
                result.set(paint, count)?;
            }
        }
        Ok(result)
    }

    /// The painted cells as disjoint pattern rectangles, for files. Greedy and
    /// deterministic: from each uncovered cell in row order it takes the
    /// pattern size whose rectangle covers the most uncovered cells.
    pub fn fills(&self) -> Vec<SceneryFill> {
        let cells: BTreeMap<(i16, i16), u8> = self
            .paints()
            .into_iter()
            .map(|p| ((p.y, p.x), p.metatile))
            .collect();
        let mut covered = BTreeSet::new();
        let mut fills = Vec::new();
        for &(y, x) in cells.keys() {
            if covered.contains(&(y, x)) {
                continue;
            }
            let free = |cx: i16, cy: i16| {
                cells
                    .get(&(cy, cx))
                    .copied()
                    .filter(|_| !covered.contains(&(cy, cx)))
            };
            let mut best: Option<(u32, i16, i16, Vec<Vec<u8>>)> = None;
            for (pw, ph) in PATTERNS {
                let Some(tiles) = (0..ph)
                    .map(|j| {
                        (0..pw)
                            .map(|i| free(x.checked_add(i)?, y.checked_add(j)?))
                            .collect::<Option<Vec<_>>>()
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                let matches = |cx: i16, cy: i16| {
                    free(cx, cy) == Some(tiles[((cy - y) % ph) as usize][((cx - x) % pw) as usize])
                };
                let column =
                    |cx: i16| (0..ph).all(|j| y.checked_add(j).is_some_and(|cy| matches(cx, cy)));
                let mut w = pw;
                while x.checked_add(w).is_some_and(column) {
                    w += 1;
                }
                let row = |cy: i16| (0..w).all(|i| matches(x + i, cy));
                let mut h = ph;
                while y.checked_add(h).is_some_and(row) {
                    h += 1;
                }
                let area = w as u32 * h as u32;
                if best.as_ref().is_none_or(|b| area > b.0) {
                    best = Some((area, w, h, tiles));
                }
            }
            let (_, w, h, tiles) = best.expect("the 1x1 pattern always fits");
            // Inclusive: a fill can end on the canvas's last cell.
            for cy in y..=y + (h - 1) {
                for cx in x..=x + (w - 1) {
                    covered.insert((cy, cx));
                }
            }
            fills.push(SceneryFill {
                at: [x, y],
                size: [w as u16, h as u16],
                tiles,
            });
        }
        fills
    }

    /// The file text: the grouped format, one fill per line.
    pub fn to_json(&self) -> String {
        let fills: Vec<String> = self
            .fills()
            .iter()
            .map(|fill| format!("    {}", serde_json::to_string(fill).expect("plain data")))
            .collect();
        let fills = if fills.is_empty() {
            "[]".to_string()
        } else {
            format!("[\n{}\n  ]", fills.join(",\n"))
        };
        format!("{{\n  \"format\": \"{SCENERY_FORMAT}\",\n  \"fills\": {fills}\n}}\n")
    }

    fn inside(x: i32, y: i32) -> bool {
        (0..LAYOUT_WIDTH as i32).contains(&x) && (0..LAYOUT_HEIGHT as i32).contains(&y)
    }
    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        let value = if Self::inside(x, y) {
            *self
                .dense
                .as_ref()?
                .get(y as usize * LAYOUT_WIDTH + x as usize)?
        } else {
            let (x, y) = (i16::try_from(x).ok()?, i16::try_from(y).ok()?);
            let chunk = self.chunks.get(&(x.div_euclid(16), y.div_euclid(16)))?;
            chunk.cells[y.rem_euclid(16) as usize * SIDE + x.rem_euclid(16) as usize]
        };
        (value != EMPTY).then_some(value as u8)
    }
    pub fn set(&mut self, p: SceneryPaint, count: usize) -> Result<(), String> {
        if p.metatile as usize >= count {
            return Err(format!(
                "scenery paint ({}, {}) metatile {} is not in the level",
                p.x, p.y, p.metatile
            ));
        }
        let value = if Self::inside(p.x.into(), p.y.into()) {
            let cells = self.dense.get_or_insert_with(|| {
                vec![EMPTY; LAYOUT_WIDTH * LAYOUT_HEIGHT].into_boxed_slice()
            });
            &mut cells[p.y as usize * LAYOUT_WIDTH + p.x as usize]
        } else {
            let chunk = self
                .chunks
                .entry((p.x.div_euclid(16), p.y.div_euclid(16)))
                .or_insert_with(|| Chunk {
                    cells: Box::new([EMPTY; 256]),
                    used: 0,
                });
            let cell =
                &mut chunk.cells[p.y.rem_euclid(16) as usize * SIDE + p.x.rem_euclid(16) as usize];
            if *cell == EMPTY {
                chunk.used += 1;
            }
            cell
        };
        if *value == EMPTY {
            self.len += 1;
        }
        *value = p.metatile.into();
        Ok(())
    }
    pub fn erase(&mut self, x: i16, y: i16) {
        let removed = if Self::inside(x.into(), y.into()) {
            if let Some(cells) = &mut self.dense {
                std::mem::replace(&mut cells[y as usize * LAYOUT_WIDTH + x as usize], EMPTY)
                    != EMPTY
            } else {
                false
            }
        } else {
            let key = (x.div_euclid(16), y.div_euclid(16));
            if let Some(chunk) = self.chunks.get_mut(&key) {
                let removed = std::mem::replace(
                    &mut chunk.cells[y.rem_euclid(16) as usize * SIDE + x.rem_euclid(16) as usize],
                    EMPTY,
                ) != EMPTY;
                if removed {
                    chunk.used -= 1;
                }
                if chunk.used == 0 {
                    self.chunks.remove(&key);
                }
                removed
            } else {
                false
            }
        };
        if removed {
            self.len -= 1;
        }
        if self.len == 0 {
            self.dense = None;
        }
    }
    /// Materialize a deterministic sparse snapshot for IO, never for rendering.
    pub fn paints(&self) -> Vec<SceneryPaint> {
        let mut paints = Vec::with_capacity(self.len);
        if let Some(cells) = &self.dense {
            for (i, &value) in cells.iter().enumerate() {
                if value != EMPTY {
                    paints.push(SceneryPaint {
                        x: (i % LAYOUT_WIDTH) as i16,
                        y: (i / LAYOUT_WIDTH) as i16,
                        metatile: value as u8,
                    });
                }
            }
        }
        for (&(cx, cy), chunk) in &self.chunks {
            for (i, &value) in chunk.cells.iter().enumerate() {
                if value != EMPTY {
                    paints.push(SceneryPaint {
                        x: cx * 16 + (i % 16) as i16,
                        y: cy * 16 + (i / 16) as i16,
                        metatile: value as u8,
                    });
                }
            }
        }
        paints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_edits_match_sparse_reference_across_chunk_edges_and_signed_extremes() {
        let mut overlay = SceneryOverlay::empty();
        let mut reference = BTreeMap::new();
        for y in [i16::MIN, -17, -16, -1, 0, 47, 48, i16::MAX] {
            for x in [i16::MIN, -17, -16, -1, 0, 255, 256, i16::MAX] {
                for metatile in [0, 255, 7] {
                    overlay.set(SceneryPaint { x, y, metatile }, 256).unwrap();
                    reference.insert((x, y), metatile);
                    assert_eq!(overlay.get(x.into(), y.into()), Some(metatile));
                    assert_eq!(overlay.len(), reference.len());
                }
            }
        }
        let json = serde_json::to_string(&overlay).unwrap();
        let restored: SceneryOverlay = serde_json::from_str(&json).unwrap();
        assert_eq!(overlay, restored);
        for (&(x, y), &id) in &reference {
            assert_eq!(restored.get(x.into(), y.into()), Some(id));
            overlay.erase(x, y);
            overlay.erase(x, y);
        }
        assert!(overlay.is_empty());
        assert!(overlay.dense.is_none());
        assert!(overlay.chunks.is_empty());
        assert_eq!(overlay.get(i32::MAX, 0), None);
    }

    fn overlay(cells: impl IntoIterator<Item = (i16, i16, u8)>) -> SceneryOverlay {
        let mut overlay = SceneryOverlay::empty();
        for (x, y, metatile) in cells {
            overlay.set(SceneryPaint { x, y, metatile }, 256).unwrap();
        }
        overlay
    }

    /// A wall of alternating columns (Tree Zone 2's 34/35) is one fill, a
    /// solid block another, and a stray cell a third.
    #[test]
    fn files_group_cells_into_pattern_rectangles() {
        let wall = (0..32).flat_map(|y| (-16..48).map(move |x| (x, y, 34 + (x & 1) as u8)));
        let block = (0..5).flat_map(|y| (100..140).map(move |x| (x, y + 40, 45)));
        let overlay = overlay(wall.chain(block).chain([(7, 60, 1)]));
        let fills = overlay.fills();
        assert_eq!(
            fills,
            [
                SceneryFill {
                    at: [-16, 0],
                    size: [64, 32],
                    tiles: vec![vec![34, 35]],
                },
                SceneryFill {
                    at: [100, 40],
                    size: [40, 5],
                    tiles: vec![vec![45]],
                },
                SceneryFill {
                    at: [7, 60],
                    size: [1, 1],
                    tiles: vec![vec![1]],
                },
            ]
        );
        let text = overlay.to_json();
        assert_eq!(text.lines().count(), 8, "one line per fill:\n{text}");
        let restored: SceneryOverlay = serde_json::from_str(&text).unwrap();
        assert_eq!(restored, overlay);
        assert_eq!(
            serde_json::from_str::<SceneryOverlay>(&SceneryOverlay::empty().to_json()).unwrap(),
            SceneryOverlay::empty()
        );
    }

    #[test]
    fn the_first_per_cell_format_still_loads() {
        let old = r#"{"paints":[{"x":-2,"y":48,"metatile":1},{"x":0,"y":25,"metatile":2}]}"#;
        let overlay: SceneryOverlay = serde_json::from_str(old).unwrap();
        assert_eq!(
            (overlay.get(-2, 48), overlay.get(0, 25)),
            (Some(1), Some(2))
        );
        let rewritten: SceneryOverlay = serde_json::from_str(&overlay.to_json()).unwrap();
        assert_eq!(rewritten, overlay);
    }

    #[test]
    fn later_fills_cover_earlier_ones_and_bad_fills_are_rejected() {
        let fill = |at, size, tiles: Vec<Vec<u8>>| SceneryFill { at, size, tiles };
        let layered = SceneryOverlay::try_from_fills(
            &[
                fill([0, 0], [4, 1], vec![vec![1]]),
                fill([1, 0], [2, 1], vec![vec![2, 3]]),
            ],
            128,
        )
        .unwrap();
        let row: Vec<_> = (0..4).map(|x| layered.get(x, 0)).collect();
        assert_eq!(row, [Some(1), Some(2), Some(3), Some(1)]);
        for (bad, message) in [
            (fill([0, 0], [0, 1], vec![vec![1]]), "needs a size"),
            (fill([0, 0], [2, 2], vec![vec![1, 2], vec![3]]), "equal"),
            (
                fill([i16::MAX, 0], [2, 1], vec![vec![1]]),
                "past the canvas",
            ),
            (fill([0, 0], [1, 1], vec![vec![200]]), "not in the level"),
            (
                fill([0, 0], [u16::MAX, u16::MAX], vec![vec![1]]),
                "cells, over",
            ),
        ] {
            let error = SceneryOverlay::try_from_fills(&[bad], 128).unwrap_err();
            assert!(error.contains(message), "{error:?} lacks {message:?}");
        }
        for text in [
            r#"{"format":"sml2-scenery/9","fills":[]}"#,
            r#"{"format":"sml2-scenery/2","paints":[{"x":0,"y":0,"metatile":1}]}"#,
            r#"{"fills":[{"at":[0,0],"size":[1,1],"tiles":[[1]]}]}"#,
        ] {
            assert!(
                serde_json::from_str::<SceneryOverlay>(text).is_err(),
                "{text}"
            );
        }
    }

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
