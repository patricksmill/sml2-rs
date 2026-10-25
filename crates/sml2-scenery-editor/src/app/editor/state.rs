//! Authoring state and reversible gestures, independent of the live game.

use eframe::egui::{Pos2, Rect, Vec2};
use sml2_core::level::{LevelLayout, LAYOUT_HEIGHT, LAYOUT_WIDTH};
use sml2_core::scenery::{SceneryOverlay, SceneryPaint};
use std::collections::BTreeMap;

pub(super) type Cell = (i16, i16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Tool {
    #[default]
    Brush,
    Eraser,
    Eyedropper,
    Rectangle,
}

impl Tool {
    pub const ALL: [(Self, &'static str, &'static str); 4] = [
        (Self::Brush, "Brush", "B · Drag to paint"),
        (Self::Eraser, "Eraser", "E · Drag to erase"),
        (
            Self::Eyedropper,
            "Pick",
            "I · Pick a tile from terrain or scenery",
        ),
        (Self::Rectangle, "Rectangle", "R · Drag a filled rectangle"),
    ];
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Viewport {
    pub center: Pos2,
    pub zoom: u8,
}

impl Viewport {
    pub fn world(self, rect: Rect, pos: Pos2) -> Pos2 {
        self.center + (pos - rect.center()) / f32::from(self.zoom)
    }

    pub fn screen(self, rect: Rect, world: Pos2) -> Pos2 {
        rect.center() + (world - self.center) * f32::from(self.zoom)
    }

    pub fn cell(self, rect: Rect, pos: Pos2) -> Option<Cell> {
        let world = self.world(rect, pos);
        let (x, y) = ((world.x / 16.0).floor(), (world.y / 16.0).floor());
        (x >= i16::MIN as f32
            && x <= i16::MAX as f32
            && y >= i16::MIN as f32
            && y <= i16::MAX as f32)
            .then_some((x as i16, y as i16))
    }

    pub fn zoom_at(&mut self, rect: Rect, anchor: Pos2, zoom: u8) {
        let before = self.world(rect, anchor);
        self.zoom = zoom.clamp(1, 8);
        self.center += before - self.world(rect, anchor);
        self.clamp();
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.center -= delta / f32::from(self.zoom);
        self.clamp();
    }

    fn clamp(&mut self) {
        let min = i16::MIN as f32 * 16.0;
        let max = i16::MAX as f32 * 16.0;
        self.center.x = self.center.x.clamp(min, max);
        self.center.y = self.center.y.clamp(min, max);
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Change {
    cell: Cell,
    before: Option<u8>,
    after: Option<u8>,
}

type Edit = Vec<Change>;

pub(super) enum Gesture {
    Stroke {
        last: Option<Cell>,
        value: Option<u8>,
        changes: BTreeMap<Cell, Change>,
    },
    Rectangle {
        start: Cell,
        end: Cell,
        value: u8,
    },
}

pub(super) struct Workspace {
    pub metatile: u8,
    pub viewport: Viewport,
    pub start: Pos2,
    pub tool: Tool,
    pub grid: bool,
    pub scenery: SceneryOverlay,
    pub chunk_versions: BTreeMap<Cell, u64>,
    pub layout: LevelLayout,
    pub used: Vec<u8>,
    pub count: usize,
    /// Scene targets protect their native 10×9 cells from authoring.
    pub scene_border: bool,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    pub gesture: Option<Gesture>,
    pub dirty: bool,
    pub load_error: Option<String>,
    pub save_error: Option<String>,
}

impl Workspace {
    pub fn new(layout: LevelLayout, count: usize, start: Pos2) -> Self {
        let used = layout
            .distinct_metatile_ids()
            .into_iter()
            .filter(|&id| usize::from(id) < count)
            .collect::<Vec<_>>();
        Self {
            metatile: used.first().copied().unwrap_or(0),
            viewport: Viewport {
                center: start,
                zoom: 2,
            },
            start,
            tool: Tool::Brush,
            grid: true,
            scenery: SceneryOverlay::empty(),
            chunk_versions: BTreeMap::new(),
            layout,
            used,
            count,
            scene_border: false,
            undo: Vec::new(),
            redo: Vec::new(),
            gesture: None,
            dirty: false,
            load_error: None,
            save_error: None,
        }
    }

    pub fn reset_view(&mut self) {
        self.viewport = Viewport {
            center: self.start,
            zoom: 2,
        };
    }

    pub fn paint(&self, cell: Cell) -> Option<u8> {
        self.scenery.get(i32::from(cell.0), i32::from(cell.1))
    }

    pub fn dimensions(&self) -> (usize, usize) {
        if self.scene_border {
            (10, 9)
        } else {
            (LAYOUT_WIDTH, LAYOUT_HEIGHT)
        }
    }

    pub fn outside(&self, (x, y): Cell) -> bool {
        let (width, height) = self.dimensions();
        !(0..width as i16).contains(&x) || !(0..height as i16).contains(&y)
    }

    pub fn sample(&self, cell: Cell) -> Option<u8> {
        self.paint(cell)
            .or_else(|| self.layout.get(cell.0 as usize, cell.1 as usize))
    }

    fn put(&mut self, cell: Cell, value: Option<u8>) {
        if let Some(metatile) = value {
            self.scenery
                .set(
                    SceneryPaint {
                        x: cell.0,
                        y: cell.1,
                        metatile,
                    },
                    self.count,
                )
                .expect("validated authoring tile");
        } else {
            self.scenery.erase(cell.0, cell.1);
        }
        *self
            .chunk_versions
            .entry((cell.0.div_euclid(16), cell.1.div_euclid(16)))
            .or_default() += 1;
    }

    pub fn begin(&mut self, cell: Cell, erase: bool) {
        if self.gesture.is_some() || self.load_error.is_some() {
            return;
        }
        if self.tool == Tool::Eyedropper && !erase {
            if let Some(id) = self.sample(cell).filter(|&id| usize::from(id) < self.count) {
                self.metatile = id;
            }
            return;
        }
        if usize::from(self.metatile) >= self.count {
            return;
        }
        self.gesture = Some(if self.tool == Tool::Rectangle && !erase {
            Gesture::Rectangle {
                start: cell,
                end: cell,
                value: self.metatile,
            }
        } else {
            Gesture::Stroke {
                last: None,
                value: (!(erase || self.tool == Tool::Eraser)).then_some(self.metatile),
                changes: BTreeMap::new(),
            }
        });
        self.extend(Some(cell));
    }

    pub fn extend(&mut self, cell: Option<Cell>) {
        let Some(mut gesture) = self.gesture.take() else {
            return;
        };
        match &mut gesture {
            Gesture::Rectangle { end, .. } => {
                if let Some(cell) = cell {
                    *end = cell;
                }
            }
            Gesture::Stroke {
                last,
                value,
                changes,
            } => {
                if let Some(cell) = cell {
                    for at in line(last.unwrap_or(cell), cell) {
                        if self.scene_border && !self.outside(at) {
                            continue;
                        }
                        let before = self.paint(at);
                        if before == *value {
                            continue;
                        }
                        changes
                            .entry(at)
                            .or_insert(Change {
                                cell: at,
                                before,
                                after: *value,
                            })
                            .after = *value;
                        self.put(at, *value);
                    }
                }
                *last = cell;
            }
        }
        self.gesture = Some(gesture);
    }

    pub fn finish(&mut self) -> bool {
        let Some(gesture) = self.gesture.take() else {
            return false;
        };
        let edit: Edit = match gesture {
            Gesture::Stroke { changes, .. } => changes
                .into_values()
                .filter(|c| c.before != c.after)
                .collect(),
            Gesture::Rectangle { start, end, value } => {
                let mut edit = Vec::new();
                for y in start.1.min(end.1)..=start.1.max(end.1) {
                    for x in start.0.min(end.0)..=start.0.max(end.0) {
                        let cell = (x, y);
                        if self.scene_border && !self.outside(cell) {
                            continue;
                        }
                        let before = self.paint(cell);
                        if before == Some(value) {
                            continue;
                        }
                        self.put(cell, Some(value));
                        edit.push(Change {
                            cell,
                            before,
                            after: Some(value),
                        });
                    }
                }
                edit
            }
        };
        if edit.is_empty() {
            return false;
        }
        self.undo.push(edit);
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
        true
    }

    pub fn cancel(&mut self) -> bool {
        let Some(gesture) = self.gesture.take() else {
            return false;
        };
        if let Gesture::Stroke { changes, .. } = gesture {
            for change in changes.into_values() {
                self.put(change.cell, change.before);
            }
        }
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        self.cancel();
        if let Some(edit) = self.undo.pop() {
            for change in &edit {
                self.put(change.cell, change.before);
            }
            self.redo.push(edit);
            self.dirty = true;
        }
    }

    pub fn redo(&mut self) {
        self.cancel();
        if let Some(edit) = self.redo.pop() {
            for change in &edit {
                self.put(change.cell, change.after);
            }
            self.undo.push(edit);
            self.dirty = true;
        }
    }
}

/// Integer Bresenham interpolation visits every cell along a fast drag.
fn line(start: Cell, end: Cell) -> Vec<Cell> {
    let (mut x, mut y) = (i32::from(start.0), i32::from(start.1));
    let (ex, ey) = (i32::from(end.0), i32::from(end.1));
    let (dx, dy) = ((ex - x).abs(), -(ey - y).abs());
    let (sx, sy) = (if x < ex { 1 } else { -1 }, if y < ey { 1 } else { -1 });
    let mut error = dx + dy;
    let mut cells = Vec::new();
    loop {
        cells.push((x as i16, y as i16));
        if x == ex && y == ey {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
    cells
}
