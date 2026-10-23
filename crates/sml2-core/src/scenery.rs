//! Indexed render-only scenery; JSON remains a sparse list of paints.
use crate::level::{LAYOUT_HEIGHT, LAYOUT_WIDTH};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const EMPTY: u16 = 256;
const SIDE: usize = 16;

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
#[derive(Serialize, Deserialize)]
struct SceneryData {
    paints: Vec<SceneryPaint>,
}
impl Serialize for SceneryOverlay {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SceneryData {
            paints: self.paints(),
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for SceneryOverlay {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SceneryData::deserialize(deserializer)?;
        Self::try_from_paints(data.paints, 256).map_err(serde::de::Error::custom)
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
