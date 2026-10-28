//! Expanded-level recipes: how to rebuild one level in a larger world.
//!
//! ROM and DX data are never redistributed, so a recipe holds only authored
//! operations and is applied at load to the player's own extraction (vanilla
//! or DX). [`LevelRecipe::apply`] works in this order:
//!
//! 1. The output world is [`World::expanded`] `world` screens, at least as
//!    large as the source on both axes. The source cells, lock and scene
//!    tables, spawns and start blocks are copied in place; new cells are
//!    `fill` and new screens have no locks and scene 0.
//! 2. Every `moves` source region (in cells) is cleared to `fill`, then each
//!    move writes the *loaded* layout's region to its destination, in order.
//!    Reads never see earlier moves, so regions can swap. A move whose
//!    corners and size are whole screens carries those screens' locks and
//!    scenes too (its vacated screens get none).
//! 3. Spawns and start blocks whose point lies in one move's source follow
//!    it; a point in two moves' sources needs an explicit placement.
//! 4. `paint` writes authored metatiles, `screens` sets locks and scenes,
//!    `spawns`, `start` and `secret_start` place records explicitly.
//! 5. Spawns are stably re-sorted by X, the order the stream scan walks.
//!    Spawn placements name records by their loaded index.

use serde::{Deserialize, Serialize};

use crate::level::{LevelLayout, LevelStart, SpawnRecord, METATILE_COUNT};
use crate::world::{World, SCREEN_CELLS};

/// Schema id every recipe carries.
pub const RECIPE_SCHEMA: &str = "sml2-expanded-level/1";

/// The authored rebuild of one level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelRecipe {
    pub schema: String,
    pub level_id: u8,
    /// [`layout_fingerprint`]s of the layouts the recipe was written against
    /// (say vanilla and DX); empty accepts any.
    #[serde(default)]
    pub sources: Vec<String>,
    /// `[wide, high]` in screens.
    pub world: [u16; 2],
    /// Metatile id for new and vacated cells.
    pub fill: u8,
    #[serde(default)]
    pub moves: Vec<RegionMove>,
    #[serde(default)]
    pub paint: Vec<Paint>,
    #[serde(default)]
    pub screens: Vec<ScreenEdit>,
    #[serde(default)]
    pub spawns: Vec<SpawnPlacement>,
    #[serde(default)]
    pub start: Option<LevelStart>,
    #[serde(default)]
    pub secret_start: Option<LevelStart>,
}

/// Move `size` cells at `from` in the loaded layout to `to`. `[col, row]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionMove {
    pub from: [u16; 2],
    pub size: [u16; 2],
    pub to: [u16; 2],
}

/// Authored metatiles. `[col, row]` cells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Paint {
    /// One metatile over a rectangle.
    Fill {
        at: [u16; 2],
        size: [u16; 2],
        id: u8,
    },
    /// A block of metatiles, row-major, `width` per row.
    Cells {
        at: [u16; 2],
        width: u16,
        ids: Vec<u8>,
    },
}

/// Lock bits (`bit0` right, `bit1` left, `bit2` up, `bit3` down) and/or
/// scene id of the screen `[x, y]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenEdit {
    pub at: [u16; 2],
    #[serde(default)]
    pub locks: Option<u8>,
    #[serde(default)]
    pub scene: Option<u8>,
}

/// Place spawn record `index` (its loaded order) at world pixel `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnPlacement {
    pub index: usize,
    pub x: u16,
    pub y: u16,
}

/// The parts of a level a recipe rebuilds. Missing source tables may be
/// empty; they read as 0.
#[derive(Debug, Clone)]
pub struct LevelParts {
    pub layout: LevelLayout,
    pub edge_map: Vec<u8>,
    pub scene_map: Vec<u8>,
    pub spawns: Vec<SpawnRecord>,
    pub start: LevelStart,
    pub secret_start: Option<LevelStart>,
}

/// FNV-1a 64 of a layout's cells, as 16 hex digits: what
/// [`LevelRecipe::sources`] lists.
pub fn layout_fingerprint(layout: &LevelLayout) -> String {
    let hash = layout.cells.iter().fold(0xCBF2_9CE4_8422_2325u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3)
    });
    format!("{hash:016x}")
}

/// A cell rectangle, `at` + `size`, in `[col, row]`.
#[derive(Clone, Copy)]
struct Rect {
    at: [u16; 2],
    size: [u16; 2],
}

impl Rect {
    fn cells(self) -> impl Iterator<Item = (usize, usize)> {
        let [c, r] = self.at.map(usize::from);
        let [w, h] = self.size.map(usize::from);
        (r..r + h).flat_map(move |row| (c..c + w).map(move |col| (col, row)))
    }

    fn check(self, world: World, what: &str) -> Result<(), String> {
        let [c, r] = self.at.map(usize::from);
        let [w, h] = self.size.map(usize::from);
        if w == 0 || h == 0 || c + w > world.cells_wide() || r + h > world.cells_high() {
            return Err(format!(
                "{what} {:?}+{:?} is empty or leaves the {}x{}-cell layout",
                self.at,
                self.size,
                world.cells_wide(),
                world.cells_high()
            ));
        }
        Ok(())
    }

    /// Whether world pixel `(x, y)` lies inside.
    fn holds(self, x: u16, y: u16) -> bool {
        let px = |cells: u16| u32::from(cells) * SCREEN_CELLS as u32;
        let (x, y) = (u32::from(x), u32::from(y));
        (px(self.at[0])..px(self.at[0]) + px(self.size[0])).contains(&x)
            && (px(self.at[1])..px(self.at[1]) + px(self.size[1])).contains(&y)
    }

    /// The same rect in screens, when it is whole screens.
    fn screens(self) -> Option<Rect> {
        let s = SCREEN_CELLS as u16;
        (self.at.iter().chain(&self.size).all(|v| v % s == 0)).then(|| Rect {
            at: self.at.map(|v| v / s),
            size: self.size.map(|v| v / s),
        })
    }
}

impl RegionMove {
    fn source(self) -> Rect {
        Rect {
            at: self.from,
            size: self.size,
        }
    }

    fn dest(self) -> Rect {
        Rect {
            at: self.to,
            size: self.size,
        }
    }

    /// Pixel offset from source to destination.
    fn delta(self) -> (i32, i32) {
        let d = |i: usize| (i32::from(self.to[i]) - i32::from(self.from[i])) * SCREEN_CELLS as i32;
        (d(0), d(1))
    }
}

fn metatile(id: u8, what: &str) -> Result<u8, String> {
    if usize::from(id) < METATILE_COUNT {
        Ok(id)
    } else {
        Err(format!(
            "{what} metatile {id} is past the {METATILE_COUNT}-entry table"
        ))
    }
}

impl LevelRecipe {
    /// Rebuild `source` into the recipe's world (see the module docs).
    pub fn apply(&self, source: &LevelParts) -> Result<LevelParts, String> {
        self.build(source)
            .map_err(|e| format!("level {:02X} recipe: {e}", self.level_id))
    }

    fn build(&self, source: &LevelParts) -> Result<LevelParts, String> {
        if self.schema != RECIPE_SCHEMA {
            return Err(format!(
                "schema {:?}, expected {RECIPE_SCHEMA:?}",
                self.schema
            ));
        }
        let fingerprint = layout_fingerprint(&source.layout);
        if !self.sources.is_empty() && !self.sources.contains(&fingerprint) {
            return Err(format!(
                "written for layouts {:?}, loaded {fingerprint}",
                self.sources
            ));
        }
        let from = source.layout.world;
        let world = World::expanded(self.world[0], self.world[1])?;
        if world.screens_wide < from.screens_wide || world.screens_high < from.screens_high {
            return Err(format!(
                "world {:?} is smaller than the loaded {}x{} screens",
                self.world, from.screens_wide, from.screens_high
            ));
        }
        let fill = metatile(self.fill, "fill")?;
        for m in &self.moves {
            m.source().check(from, "move source")?;
            m.dest().check(world, "move destination")?;
        }

        // 1. The loaded level in place.
        let read = |col: usize, row: usize| source.layout.get(col, row).unwrap_or(fill);
        let mut cells = vec![fill; world.cells_wide() * world.cells_high()];
        for row in 0..from.cells_high() {
            for col in 0..from.cells_wide() {
                cells[row * world.cells_wide() + col] = read(col, row);
            }
        }
        let screen_of = |table: &[u8], x: usize, y: usize| {
            table
                .get(y * usize::from(from.screens_wide) + x)
                .copied()
                .unwrap_or(0)
        };
        let mut edges = vec![0; world.screens()];
        let mut scenes = vec![0; world.screens()];
        let screen = |x: usize, y: usize| y * usize::from(world.screens_wide) + x;
        for y in 0..usize::from(from.screens_high) {
            for x in 0..usize::from(from.screens_wide) {
                edges[screen(x, y)] = screen_of(&source.edge_map, x, y);
                scenes[screen(x, y)] = screen_of(&source.scene_map, x, y);
            }
        }

        // 2. Moves: vacate every source, then write every destination.
        for m in &self.moves {
            for (col, row) in m.source().cells() {
                cells[row * world.cells_wide() + col] = fill;
            }
            for (x, y) in m.source().screens().into_iter().flat_map(Rect::cells) {
                (edges[screen(x, y)], scenes[screen(x, y)]) = (0, 0);
            }
        }
        for m in &self.moves {
            for ((sc, sr), (dc, dr)) in m.source().cells().zip(m.dest().cells()) {
                cells[dr * world.cells_wide() + dc] = read(sc, sr);
            }
            if let (Some(src), Some(dst)) = (m.source().screens(), m.dest().screens()) {
                for ((sx, sy), (dx, dy)) in src.cells().zip(dst.cells()) {
                    edges[screen(dx, dy)] = screen_of(&source.edge_map, sx, sy);
                    scenes[screen(dx, dy)] = screen_of(&source.scene_map, sx, sy);
                }
            }
        }

        // 4. Authored cells and screens.
        for p in &self.paint {
            match p {
                Paint::Fill { at, size, id } => {
                    let id = metatile(*id, "paint")?;
                    let rect = Rect {
                        at: *at,
                        size: *size,
                    };
                    rect.check(world, "paint")?;
                    for (col, row) in rect.cells() {
                        cells[row * world.cells_wide() + col] = id;
                    }
                }
                Paint::Cells { at, width, ids } => {
                    if *width == 0 || ids.len() % usize::from(*width) != 0 {
                        return Err(format!(
                            "paint at {at:?}: {} ids do not fill rows of {width}",
                            ids.len()
                        ));
                    }
                    let rect = Rect {
                        at: *at,
                        size: [*width, (ids.len() / usize::from(*width)) as u16],
                    };
                    rect.check(world, "paint")?;
                    for ((col, row), &id) in rect.cells().zip(ids) {
                        cells[row * world.cells_wide() + col] = metatile(id, "paint")?;
                    }
                }
            }
        }
        for e in &self.screens {
            let [x, y] = e.at.map(usize::from);
            if x >= usize::from(world.screens_wide) || y >= usize::from(world.screens_high) {
                return Err(format!("screen edit {:?} is outside the world", e.at));
            }
            if let Some(locks) = e.locks {
                if locks > 0x0F {
                    return Err(format!(
                        "screen {:?}: lock bits {locks:#04X} past bit 3",
                        e.at
                    ));
                }
                edges[screen(x, y)] = locks;
            }
            if let Some(scene) = e.scene {
                scenes[screen(x, y)] = scene;
            }
        }

        // 3 and 4. Spawns and starts.
        let mut spawns = source.spawns.clone();
        let mut placed = vec![false; spawns.len()];
        for p in &self.spawns {
            let Some(done) = placed.get_mut(p.index) else {
                return Err(format!("spawn {} does not exist", p.index));
            };
            if std::mem::replace(done, true) {
                return Err(format!("spawn {} is placed twice", p.index));
            }
            check_point(world, p.x, p.y, &format!("spawn {}", p.index))?;
            (spawns[p.index].x, spawns[p.index].y) = (p.x, p.y);
        }
        for (i, s) in spawns.iter_mut().enumerate().filter(|(i, _)| !placed[*i]) {
            if let Some((x, y)) = self.follow(world, s.x, s.y, &format!("spawn {i}"))? {
                (s.x, s.y) = (x, y);
            }
        }
        spawns.sort_by_key(|s| s.x);
        let start = self.place_start(world, self.start, source.start, "start")?;
        let secret_start = match source.secret_start {
            Some(s) => Some(self.place_start(world, self.secret_start, s, "secret start")?),
            None => self
                .secret_start
                .map(|s| check_point(world, s.mario_x, s.mario_y, "secret start").map(|()| s))
                .transpose()?,
        };

        Ok(LevelParts {
            layout: LevelLayout::new(world, cells)?,
            edge_map: edges,
            scene_map: scenes,
            spawns,
            start,
            secret_start,
        })
    }

    /// Where the content of cell `(col, row)` ends up: every destination of
    /// the moves whose source holds it, or the cell itself when none does.
    /// For render-only data (scenery paints) that should follow the layout.
    pub fn cell_destinations(&self, col: i32, row: i32) -> Vec<(i32, i32)> {
        let inside = |m: &&RegionMove| {
            let [c, r, w, h] = [m.from[0], m.from[1], m.size[0], m.size[1]].map(i32::from);
            (c..c + w).contains(&col) && (r..r + h).contains(&row)
        };
        let moved: Vec<_> = self
            .moves
            .iter()
            .filter(inside)
            .map(|m| {
                let d = |i: usize| i32::from(m.to[i]) - i32::from(m.from[i]);
                (col + d(0), row + d(1))
            })
            .collect();
        if moved.is_empty() {
            vec![(col, row)]
        } else {
            moved
        }
    }

    /// Where a point moves: `None` outside every move's source.
    fn follow(
        &self,
        world: World,
        x: u16,
        y: u16,
        what: &str,
    ) -> Result<Option<(u16, u16)>, String> {
        let mut holders = self.moves.iter().filter(|m| m.source().holds(x, y));
        let Some(m) = holders.next() else {
            return Ok(None);
        };
        if holders.next().is_some() {
            return Err(format!(
                "{what} at ({x:#06X}, {y:#06X}) is in two moves; place it"
            ));
        }
        let (dx, dy) = m.delta();
        let at = ((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16);
        check_point(world, at.0, at.1, what)?;
        Ok(Some(at))
    }

    /// An explicit start, or the loaded one moved with Mario's point.
    fn place_start(
        &self,
        world: World,
        explicit: Option<LevelStart>,
        loaded: LevelStart,
        what: &str,
    ) -> Result<LevelStart, String> {
        if let Some(s) = explicit {
            check_point(world, s.mario_x, s.mario_y, what)?;
            return Ok(s);
        }
        let Some((x, y)) = self.follow(world, loaded.mario_x, loaded.mario_y, what)? else {
            return Ok(loaded);
        };
        let (dx, dy) = (
            x.wrapping_sub(loaded.mario_x),
            y.wrapping_sub(loaded.mario_y),
        );
        Ok(LevelStart {
            mario_x: x,
            mario_y: y,
            camera_x: loaded.camera_x.wrapping_add(dx),
            camera_y: loaded.camera_y.wrapping_add(dy),
        })
    }
}

fn check_point(world: World, x: u16, y: u16, what: &str) -> Result<(), String> {
    let room = world.room();
    if i32::from(x) >= room.right || i32::from(y) >= room.bottom {
        return Err(format!(
            "{what} at ({x:#06X}, {y:#06X}) is outside the world"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROM_CELLS: usize = 256 * 48;

    /// A ROM-sized level whose cell ids encode their screen.
    fn source() -> LevelParts {
        let cells = (0..ROM_CELLS)
            .map(|i| ((i / 256 / 16) * 16 + (i % 256) / 16) as u8 % 100)
            .collect();
        let spawn = |x, y| SpawnRecord {
            x,
            y,
            anim_index: x as u8,
            mode_flag: 0,
            sprite_id: 0,
            hitbox: [0; 4],
            raw: [0; 6],
        };
        LevelParts {
            layout: LevelLayout::new(World::ROM, cells).unwrap(),
            edge_map: (0..48).map(|i| i as u8 & 0x0F).collect(),
            scene_map: (0..48).map(|i| 0x40 + i as u8).collect(),
            spawns: vec![
                spawn(0x0120, 0x0040),
                spawn(0x0540, 0x0180),
                spawn(0x0910, 0x0190),
            ],
            start: LevelStart {
                mario_y: 0x01A0,
                mario_x: 0x0530,
                camera_y: 0x0180,
                camera_x: 0x0550,
            },
            secret_start: None,
        }
    }

    fn recipe(world: [u16; 2]) -> LevelRecipe {
        LevelRecipe {
            schema: RECIPE_SCHEMA.into(),
            level_id: 2,
            sources: Vec::new(),
            world,
            fill: 127,
            moves: Vec::new(),
            paint: Vec::new(),
            screens: Vec::new(),
            spawns: Vec::new(),
            start: None,
            secret_start: None,
        }
    }

    #[test]
    fn an_empty_recipe_keeps_the_level_in_place() {
        let src = source();
        let out = recipe([16, 3]).apply(&src).unwrap();
        assert_eq!(out.layout.world, World::expanded(16, 3).unwrap());
        assert_eq!(out.layout.cells, src.layout.cells);
        assert_eq!(
            (&out.edge_map, &out.scene_map),
            (&src.edge_map, &src.scene_map)
        );
        assert_eq!((&out.spawns, out.start), (&src.spawns, src.start));
    }

    #[test]
    fn a_larger_world_fills_its_new_cells_and_screens() {
        let out = recipe([20, 4]).apply(&source()).unwrap();
        let l = &out.layout;
        assert_eq!(l.cells.len(), 320 * 64);
        assert_eq!(l.get(255, 47), Some(source().layout.get(255, 47).unwrap()));
        assert_eq!((l.get(256, 0), l.get(0, 48)), (Some(127), Some(127)));
        assert_eq!(out.scene_map[16], 0, "screen (16, 0) is new");
        assert_eq!(
            out.edge_map[20 + 1],
            17 & 0x0F,
            "screen (1, 1) keeps its locks"
        );
        assert_eq!(out.scene_map[20 + 5], 0x40 + 16 + 5);
    }

    /// Tree Zone 2's shape: a section moves into a new fourth screen row and
    /// takes its tables, spawns and the start with it.
    #[test]
    fn a_screen_move_carries_cells_tables_spawns_and_the_start() {
        let src = source();
        let mut r = recipe([16, 4]);
        r.moves.push(RegionMove {
            from: [0x50, 0x10],
            size: [0x30, 0x10],
            to: [0x10, 0x30],
        });
        let out = r.apply(&src).unwrap();
        let l = &out.layout;
        assert_eq!(l.get(0x10, 0x30), src.layout.get(0x50, 0x10));
        assert_eq!(l.get(0x3F, 0x3F), src.layout.get(0x7F, 0x1F));
        assert_eq!(l.get(0x50, 0x10), Some(127), "vacated");
        assert_eq!(l.get(0x80, 0x10), src.layout.get(0x80, 0x10), "untouched");
        // Screens (5..8, 1) -> (1..4, 3).
        assert_eq!(out.edge_map[3 * 16 + 1], src.edge_map[16 + 5]);
        assert_eq!(out.scene_map[3 * 16 + 3], src.scene_map[16 + 7]);
        assert_eq!((out.edge_map[16 + 5], out.scene_map[16 + 5]), (0, 0));
        // The spawn at $0540 moved to $0140 and is re-sorted after $0120.
        let xs: Vec<_> = out
            .spawns
            .iter()
            .map(|s| (s.x, s.y, s.anim_index))
            .collect();
        assert_eq!(
            xs,
            [
                (0x0120, 0x0040, 0x20),
                (0x0140, 0x0380, 0x40),
                (0x0910, 0x0190, 0x10)
            ]
        );
        let s = out.start;
        assert_eq!(
            (s.mario_x, s.mario_y, s.camera_x, s.camera_y),
            (0x0130, 0x03A0, 0x0150, 0x0380)
        );
    }

    #[test]
    fn cells_report_where_their_content_lands() {
        let mut r = recipe([16, 4]);
        r.moves.push(RegionMove {
            from: [0x50, 0x10],
            size: [0x30, 0x10],
            to: [0x10, 0x30],
        });
        assert_eq!(r.cell_destinations(0x51, 0x12), [(0x11, 0x32)]);
        assert_eq!(r.cell_destinations(-3, 0x12), [(-3, 0x12)], "a margin cell stays");
        assert_eq!(r.cell_destinations(0x80, 0x10), [(0x80, 0x10)]);
    }

    #[test]
    fn moves_read_the_loaded_layout_so_regions_can_swap() {
        let src = source();
        let mut r = recipe([16, 3]);
        let a = [0x00, 0x00];
        let b = [0x10, 0x00];
        r.moves.extend([
            RegionMove {
                from: a,
                size: [16, 16],
                to: b,
            },
            RegionMove {
                from: b,
                size: [16, 16],
                to: a,
            },
        ]);
        let out = r.apply(&src).unwrap();
        assert_eq!(out.layout.get(0, 0), src.layout.get(0x10, 0));
        assert_eq!(out.layout.get(0x10, 0), src.layout.get(0, 0));
        assert_eq!(
            (out.edge_map[0], out.edge_map[1]),
            (src.edge_map[1], src.edge_map[0])
        );
    }

    #[test]
    fn a_point_in_two_moves_needs_an_explicit_placement() {
        let mut r = recipe([16, 4]);
        r.moves.extend([
            RegionMove {
                from: [0x50, 0x10],
                size: [0x10, 0x10],
                to: [0, 0x30],
            },
            RegionMove {
                from: [0x50, 0x10],
                size: [0x10, 0x10],
                to: [0x20, 0x30],
            },
        ]);
        r.start = Some(source().start);
        let err = r.apply(&source()).unwrap_err();
        assert!(
            err.contains("spawn 1") && err.contains("two moves"),
            "{err}"
        );
        r.spawns.push(SpawnPlacement {
            index: 1,
            x: 0x0230,
            y: 0x0380,
        });
        let out = r.apply(&source()).unwrap();
        assert!(out.spawns.iter().any(|s| (s.x, s.y) == (0x0230, 0x0380)));
    }

    #[test]
    fn paint_and_screen_edits_apply_after_moves() {
        let mut r = recipe([16, 4]);
        r.paint.extend([
            Paint::Fill {
                at: [0, 0x30],
                size: [4, 2],
                id: 5,
            },
            Paint::Cells {
                at: [1, 0x30],
                width: 2,
                ids: vec![7, 8, 9, 10],
            },
        ]);
        r.screens.push(ScreenEdit {
            at: [3, 3],
            locks: Some(0x0C),
            scene: Some(0),
        });
        let out = r.apply(&source()).unwrap();
        let row = |r: usize| {
            (0..4)
                .map(|c| out.layout.get(c, r).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            (row(0x30), row(0x31)),
            (vec![5, 7, 8, 5], vec![5, 9, 10, 5])
        );
        assert_eq!(out.edge_map[3 * 16 + 3], 0x0C);
    }

    #[test]
    fn recipes_reject_what_they_cannot_apply() {
        let src = source();
        let fails = |r: LevelRecipe, needle: &str| {
            let err = r.apply(&src).unwrap_err();
            assert!(err.contains(needle), "{err:?} lacks {needle:?}");
        };
        fails(
            LevelRecipe {
                schema: "x".into(),
                ..recipe([16, 3])
            },
            "schema",
        );
        fails(
            LevelRecipe {
                sources: vec!["0".repeat(16)],
                ..recipe([16, 3])
            },
            "written for",
        );
        fails(recipe([15, 3]), "smaller");
        fails(recipe([16, 255 + 1]), "limits");
        fails(
            LevelRecipe {
                fill: 128,
                ..recipe([16, 3])
            },
            "fill metatile",
        );
        let mut r = recipe([16, 3]);
        r.moves.push(RegionMove {
            from: [0, 0],
            size: [16, 16],
            to: [0, 0x28],
        });
        fails(r, "move destination");
        let mut r = recipe([16, 3]);
        r.paint.push(Paint::Cells {
            at: [0, 0],
            width: 3,
            ids: vec![1; 4],
        });
        fails(r, "rows of 3");
        let mut r = recipe([16, 3]);
        r.screens.push(ScreenEdit {
            at: [0, 0],
            locks: Some(0x10),
            scene: None,
        });
        fails(r, "lock bits");
        let mut r = recipe([16, 3]);
        r.spawns.push(SpawnPlacement {
            index: 3,
            x: 0,
            y: 0,
        });
        fails(r, "spawn 3 does not exist");
        // The right fingerprint passes.
        let ok = LevelRecipe {
            sources: vec![layout_fingerprint(&src.layout)],
            ..recipe([16, 3])
        };
        assert!(ok.apply(&src).is_ok());
    }

    #[test]
    fn the_json_form_names_each_operation() {
        let json = r#"{
            "schema": "sml2-expanded-level/1",
            "level_id": 2,
            "sources": ["0123456789abcdef"],
            "world": [16, 4],
            "fill": 127,
            "moves": [{ "from": [80, 16], "size": [48, 16], "to": [16, 48] }],
            "paint": [
                { "op": "fill", "at": [0, 48], "size": [4, 2], "id": 5 },
                { "op": "cells", "at": [1, 48], "width": 2, "ids": [7, 8, 9, 10] }
            ],
            "screens": [{ "at": [3, 3], "locks": 12 }],
            "spawns": [{ "index": 1, "x": 560, "y": 896 }],
            "start": { "mario_y": 416, "mario_x": 1328, "camera_y": 384, "camera_x": 1360 }
        }"#;
        let r: LevelRecipe = serde_json::from_str(json).unwrap();
        assert_eq!(r.moves[0].to, [16, 48]);
        assert_eq!(
            r.paint[1],
            Paint::Cells {
                at: [1, 48],
                width: 2,
                ids: vec![7, 8, 9, 10]
            }
        );
        assert_eq!(r.screens[0].scene, None);
        assert!(
            serde_json::from_str::<LevelRecipe>(&json.replace("\"fill\": 127", "\"fil\": 1"))
                .is_err()
        );
        let again: LevelRecipe = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(again, r);
    }
}
