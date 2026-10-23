# 200% Zoom and Motion Smoothing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a 200% camera zoom step and an optional motion-smoothing mode that draws levels at sub-pixel positions between the last two 60 Hz ticks.

**Architecture:** The session keeps the previous tick's positions (`MotionSnapshot`). At render time, `Offsets::between` turns previous/current positions and the tick fraction into per-layer shifts in output pixels. Both level renderers (native, DX) build `Layers` (bordered background, unshifted window, pre-sampled OBJ cells), and one generic `Layers::compose` upsamples by an integer scale and applies the shifts. With smoothing off this runs at scale 1, zero shifts, no border and is byte-identical to today.

**Tech Stack:** Rust workspace (`sml2-runtime`, `sml2-desktop`), egui/eframe.

**Spec:** `docs/plans/2026-10-22-zoom200-motion-smoothing-design.md`

## Global Constraints

- Simulation is untouched; `bash captures/sweep.sh` must stay byte-identical.
- Smoothing off ⇒ every renderer output byte-identical to before (all existing tests pass unchanged except the two blit unit tests rewritten in Tasks 3 and 4).
- Snap threshold `SNAP = 8` game px per axis per tick; background border `BORDER = 8`.
- Scale `s` = largest divisor of the window's effective integer scale with `(vw·s)·(vh·s) <= 2048·2048`.
- Smoothing covers `SessionState::InLevel` and `SessionState::Demo` only; everything else renders at scale 1.
- Setting default: Off. Commits: Conventional Commits.
- Quick gate per task: `cargo test -p sml2-runtime` / `cargo test -p sml2-desktop` (and `cargo test -p sml2-core` once at the end).

---

### Task 1: 200% camera zoom

**Files:**
- Modify: `crates/sml2-desktop/src/settings/view.rs`

**Interfaces:**
- Produces: `CameraZoom::Zoom200`; `view_size(_, Zoom200)` = `(320, 288)` / `(512, 288)`.

- [ ] **Step 1: Extend the test** — in `view_sizes_match_the_design_table` add:

```rust
        assert_eq!(view_size(Off, Zoom200), (320, 288));
        assert_eq!(view_size(Wide16x9, Zoom200), (512, 288));
```

and a new test:

```rust
    #[test]
    fn zoom_cycles_through_200_percent_back_to_native() {
        use CameraZoom::*;
        assert_eq!(Zoom150.next(), Zoom200);
        assert_eq!(Zoom200.next(), Native);
        assert_eq!(Zoom200.label(), "200%");
    }
```

- [ ] **Step 2: Run** `cargo test -p sml2-desktop view_` — expect compile FAIL (`Zoom200` missing).

- [ ] **Step 3: Implement** — add `Zoom200` after `Zoom150` in the enum; `label` → `"200%"`; `next`: `Zoom150 => Zoom200`, `Zoom200 => Native`; `view_size` height `CameraZoom::Zoom200 => 288`.

- [ ] **Step 4: Run** `cargo test -p sml2-desktop` — expect PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-desktop/src/settings/view.rs
git commit -m "feat(desktop): add 200% camera zoom"
```

---

### Task 2: Motion model and per-tick snapshot

**Files:**
- Create: `crates/sml2-runtime/src/render/motion.rs`
- Modify: `crates/sml2-runtime/src/render.rs` (add `pub mod motion;`, `SpriteInst.slot`)
- Modify: `crates/sml2-runtime/src/session.rs` (`SpriteInst` construction ~line 506, `Session` field, `advance`, new methods)
- Modify: `crates/sml2-runtime/src/play.rs` (~line 528, `SpriteInst` construction)
- Modify: `crates/sml2-runtime/src/render/palette_tests.rs` (~line 126, add `slot: 0`)

**Interfaces:**
- Produces (all in `sml2_runtime::render::motion`):
  - `pub const SNAP: i32 = 8; pub const BORDER: usize = 8;`
  - `pub struct Smooth { pub alpha: f32, pub scale: usize }`, `Smooth::OFF`
  - `pub fn pick_scale(view: (usize, usize), effective_scale: u32) -> usize`
  - `pub struct MotionSnapshot { cam, player, shield, fireballs, bumpers, objects }`, `MotionSnapshot::of(&LevelContext) -> Option<Self>`
  - `pub struct Offsets { pub scale, pub bg, pub player, pub shield, pub fireballs, pub bumpers, pub objects }` with `Offsets::none()`, `Offsets::between(prev, cur, smooth)`, `border()`, `fireball(i)`, `bumper(i)`, `object(slot)`
- Produces on `Session`: field `pub(crate) motion_prev: Option<MotionSnapshot>`, `pub fn motion_offsets(&self, smooth: Smooth) -> Offsets`, `pub(crate) fn level_ctx(&self) -> Option<&LevelContext>`
- `SpriteInst` gains `#[serde(default)] pub slot: u8` (the object slot index).

- [ ] **Step 1: Write `motion.rs` with tests first**

```rust
//! Motion smoothing: in-level positions between the last two ticks, as
//! screen shifts in output pixels for the layer compositor.

use crate::session::LevelContext;

/// Largest per-tick move (game px, either axis) that is blended; larger moves
/// are warps and draw at the current tick.
pub const SNAP: i32 = 8;
/// Background pixels rendered past each view edge, so a background shifted by
/// up to `SNAP` never exposes unrendered pixels.
pub const BORDER: usize = SNAP as usize;
const MAX_PIXELS: usize = 2048 * 2048;

/// How far through the current tick a frame is drawn (`alpha` in `[0, 1]`) and
/// at how many output pixels per game pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Smooth {
    pub alpha: f32,
    pub scale: usize,
}

impl Smooth {
    pub const OFF: Self = Self { alpha: 1.0, scale: 1 };
}

/// The largest divisor of `effective_scale` that keeps a `view` frame within
/// 2048² output pixels, so the GPU still upsamples by a whole number.
pub fn pick_scale(view: (usize, usize), effective_scale: u32) -> usize {
    let n = effective_scale.max(1) as usize;
    (1..=n)
        .rev()
        .find(|s| n % s == 0 && view.0 * s * view.1 * s <= MAX_PIXELS)
        .unwrap_or(1)
}

/// The world positions one tick drew with. The camera and Mario include their
/// pans, so a pan change reads as motion.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionSnapshot {
    pub cam: (u16, u16),
    pub player: Option<(u16, u16)>,
    pub shield: Option<(u16, u16)>,
    pub fireballs: Vec<(u16, u16)>,
    pub bumpers: Vec<(u16, u16)>,
    /// `(slot, position)` per drawn object.
    pub objects: Vec<(u8, (u16, u16))>,
}

impl MotionSnapshot {
    /// `None` where frames are never blended: paused, the scene-reload blank
    /// and boss fights.
    pub fn of(ctx: &LevelContext) -> Option<Self> {
        let s = ctx.spr.as_ref()?;
        if ctx.sim.paused || ctx.scene_reload_blank || ctx.cam.boss_mode() || !s.boss_oam.is_empty() {
            return None;
        }
        let pos = |&(x, y, _): &(u16, u16, u8)| (x, y);
        let mario = |(x, y, _): (u16, u16, u8)| (x, y.wrapping_sub(u16::from(s.player_pan)));
        Some(Self {
            cam: (ctx.cam.x, ctx.cam.y.wrapping_sub(u16::from(s.scroll_pan))),
            player: s.player.map(mario),
            shield: s.player_shield.map(mario),
            fireballs: s.fireballs.iter().map(pos).collect(),
            bumpers: s.bumpers.iter().map(pos).collect(),
            objects: s.insts.iter().map(|i| (i.slot, (i.x, i.y))).collect(),
        })
    }
}

/// Screen shifts in output pixels. A sprite without its own entry moves with
/// the background (world-fixed against the in-between camera).
#[derive(Debug, Clone, PartialEq)]
pub struct Offsets {
    pub scale: usize,
    pub bg: (i32, i32),
    pub player: (i32, i32),
    pub shield: (i32, i32),
    pub fireballs: Vec<(i32, i32)>,
    pub bumpers: Vec<(i32, i32)>,
    pub objects: Vec<(u8, (i32, i32))>,
}

impl Offsets {
    pub fn none() -> Self {
        Self {
            scale: 1,
            bg: (0, 0),
            player: (0, 0),
            shield: (0, 0),
            fireballs: Vec::new(),
            bumpers: Vec::new(),
            objects: Vec::new(),
        }
    }

    pub fn border(&self) -> usize {
        if self.scale > 1 { BORDER } else { 0 }
    }

    pub fn between(prev: Option<&MotionSnapshot>, cur: Option<&MotionSnapshot>, smooth: Smooth) -> Self {
        let s = smooth.scale.max(1);
        let mut out = Self { scale: s, ..Self::none() };
        let (Some(prev), Some(cur)) = (prev, cur) else {
            return out;
        };
        let Some(cam) = shift(prev.cam, cur.cam, smooth.alpha, s) else {
            return out;
        };
        out.bg = (-cam.0, -cam.1);
        let rel = |p: Option<(u16, u16)>, c: (u16, u16)| {
            let own = p.and_then(|p| shift(p, c, smooth.alpha, s)).unwrap_or((0, 0));
            (own.0 - cam.0, own.1 - cam.1)
        };
        out.player = cur.player.map_or(out.bg, |c| rel(prev.player, c));
        out.shield = cur.shield.map_or(out.bg, |c| rel(prev.shield, c));
        out.fireballs = cur
            .fireballs
            .iter()
            .enumerate()
            .map(|(i, &c)| rel(prev.fireballs.get(i).copied(), c))
            .collect();
        out.bumpers = cur
            .bumpers
            .iter()
            .enumerate()
            .map(|(i, &c)| rel(prev.bumpers.get(i).copied(), c))
            .collect();
        out.objects = cur
            .objects
            .iter()
            .map(|&(slot, c)| {
                let p = prev.objects.iter().find(|o| o.0 == slot).map(|o| o.1);
                (slot, rel(p, c))
            })
            .collect();
        out
    }

    pub fn fireball(&self, i: usize) -> (i32, i32) {
        self.fireballs.get(i).copied().unwrap_or(self.bg)
    }

    pub fn bumper(&self, i: usize) -> (i32, i32) {
        self.bumpers.get(i).copied().unwrap_or(self.bg)
    }

    pub fn object(&self, slot: u8) -> (i32, i32) {
        self.objects.iter().find(|o| o.0 == slot).map_or(self.bg, |o| o.1)
    }
}

/// `(alpha - 1) * (cur - prev)` in output pixels; `None` past `SNAP`.
fn shift(prev: (u16, u16), cur: (u16, u16), alpha: f32, s: usize) -> Option<(i32, i32)> {
    let d = |p: u16, c: u16| i32::from(c.wrapping_sub(p) as i16);
    let (dx, dy) = (d(prev.0, cur.0), d(prev.1, cur.1));
    if dx.abs() > SNAP || dy.abs() > SNAP {
        return None;
    }
    let f = (alpha.clamp(0.0, 1.0) - 1.0) * s as f32;
    Some(((dx as f32 * f).round() as i32, (dy as f32 * f).round() as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(cam: (u16, u16)) -> MotionSnapshot {
        MotionSnapshot { cam, ..Default::default() }
    }

    #[test]
    fn scale_is_the_largest_divisor_within_the_pixel_budget() {
        assert_eq!(pick_scale((160, 144), 3), 3);
        assert_eq!(pick_scale((160, 144), 7), 7);
        assert_eq!(pick_scale((256, 144), 15), 5);
        assert_eq!(pick_scale((160, 144), 0), 1);
    }

    #[test]
    fn sprites_moving_with_the_camera_stay_put_and_static_ones_follow_the_background() {
        let mut prev = snap((100, 50));
        prev.player = Some((110, 60));
        prev.objects = vec![(3, (130, 60))];
        let mut cur = snap((104, 50));
        cur.player = Some((114, 60));
        cur.objects = vec![(3, (130, 60)), (5, (140, 60))];
        let o = Offsets::between(Some(&prev), Some(&cur), Smooth { alpha: 0.5, scale: 2 });
        assert_eq!(o.bg, (4, 0));
        assert_eq!(o.player, (0, 0));
        assert_eq!(o.object(3), (4, 0));
        assert_eq!(o.object(5), (4, 0), "a new slot is world-fixed");
        assert_eq!(o.fireball(0), (4, 0));
    }

    #[test]
    fn warps_pauses_and_full_ticks_draw_the_current_tick() {
        let s = Smooth { alpha: 0.25, scale: 3 };
        let warp = Offsets::between(Some(&snap((100, 50))), Some(&snap((120, 50))), s);
        assert_eq!(warp, Offsets { scale: 3, ..Offsets::none() });
        let missing = Offsets::between(None, Some(&snap((100, 50))), s);
        assert_eq!(missing.bg, (0, 0));
        let done = Offsets::between(
            Some(&snap((100, 50))),
            Some(&snap((104, 50))),
            Smooth { alpha: 1.0, scale: 3 },
        );
        assert_eq!(done.bg, (0, 0));
    }

    #[test]
    fn a_sprite_that_jumps_is_drawn_world_fixed() {
        let mut prev = snap((100, 50));
        prev.objects = vec![(2, (10, 10))];
        let mut cur = snap((102, 50));
        cur.objects = vec![(2, (200, 10))];
        let o = Offsets::between(Some(&prev), Some(&cur), Smooth { alpha: 0.5, scale: 1 });
        assert_eq!(o.object(2), o.bg);
    }
}
```

Register it in `crates/sml2-runtime/src/render.rs` next to `pub mod view;`: `pub mod motion;`

- [ ] **Step 2: Run** `cargo test -p sml2-runtime motion` — expect compile FAIL (`SpriteInst` has no `slot`, `boss_mode`/fields may need visibility; fix only what the compiler names).

- [ ] **Step 3: Add `SpriteInst.slot`** in `crates/sml2-runtime/src/render.rs`:

```rust
pub struct SpriteInst {
    pub sprite: u8,
    pub x: u16,
    pub y: u16,
    #[serde(default)]
    pub flip: u8,
    /// The object slot, so a smoothed frame matches objects across ticks.
    #[serde(default)]
    pub slot: u8,
}
```

In both `session.rs` (~506) and `play.rs` (~528) change `.map(|(_, o)| SpriteInst {` to `.map(|(i, o)| SpriteInst {` and add `slot: i as u8,`. In `palette_tests.rs` (~126) add `slot: 0,`. Fix any other construction site the compiler reports the same way.

- [ ] **Step 4: Capture the snapshot in `Session`** (`crates/sml2-runtime/src/session.rs`)

Add a field to `struct Session` (after `view`):

```rust
    /// The previous tick's in-level positions, for motion smoothing.
    pub(crate) motion_prev: Option<crate::render::motion::MotionSnapshot>,
```

Initialise it to `None` wherever `Session { .. }` is constructed (`rg -n "view: " crates/sml2-runtime/src/session.rs` finds the constructor). At the very top of `pub fn advance`, before `self.remember_level_state();`:

```rust
        self.motion_prev = self
            .level_ctx()
            .and_then(crate::render::motion::MotionSnapshot::of);
```

Add to `impl Session`:

```rust
    /// The level context on screen: live play or an attract demo.
    pub(crate) fn level_ctx(&self) -> Option<&LevelContext> {
        match &self.state {
            SessionState::InLevel(ctx) => Some(ctx),
            SessionState::Demo(demo) => Some(&demo.ctx),
            _ => None,
        }
    }

    /// Screen shifts for a frame drawn `smooth.alpha` of the way from the
    /// previous tick to the current one.
    pub fn motion_offsets(&self, smooth: crate::render::motion::Smooth) -> crate::render::motion::Offsets {
        use crate::render::motion::{MotionSnapshot, Offsets};
        let cur = self.level_ctx().and_then(MotionSnapshot::of);
        Offsets::between(self.motion_prev.as_ref(), cur.as_ref(), smooth)
    }
```

(If `ctx`/`demo.ctx` are boxed, return `Some(ctx.as_ref())` / `Some(demo.ctx.as_ref())`; `dx.rs`'s `dx_level` shows the pattern. Then simplify `dx_level` to use `self.level_ctx()`.)

Add a session test at the bottom of `crates/sml2-runtime/src/render/motion.rs` tests:

```rust
    #[test]
    fn a_level_session_remembers_the_previous_tick() {
        use crate::session::{FrameInput, Session, SessionAssets};
        let mut session = Session::new(SessionAssets::load().unwrap());
        session.begin_level(0, 0).unwrap();
        for _ in 0..3 {
            session.advance(FrameInput::default());
        }
        assert!(session.motion_prev.is_some());
        let done = session.motion_offsets(Smooth::OFF);
        assert_eq!((done.bg, done.player), ((0, 0), (0, 0)));
        session.enter_title();
        session.advance(FrameInput::default());
        assert!(session.motion_prev.is_none());
    }
```

- [ ] **Step 5: Run** `cargo test -p sml2-runtime` — expect PASS (all existing tests too).

- [ ] **Step 6: Commit**

```bash
git add crates/sml2-runtime/src/render/motion.rs crates/sml2-runtime/src/render.rs crates/sml2-runtime/src/session.rs crates/sml2-runtime/src/play.rs crates/sml2-runtime/src/render/palette_tests.rs crates/sml2-runtime/src/render/dx.rs
git commit -m "feat(runtime): track previous-tick positions for motion smoothing"
```

---

### Task 3: Layer compositor and native renderer

**Files:**
- Create: `crates/sml2-runtime/src/render/layers.rs`
- Modify: `crates/sml2-runtime/src/render.rs` (`render_view_output` split; `blit_obj_cell` → `native_cell`; its unit test)
- Modify: `crates/sml2-runtime/src/render/session.rs` (`LevelContext::render_output`, `Session::render_output`, new `render_palette_frame_smooth`)

**Interfaces:**
- Consumes: `motion::{Offsets, Smooth}`, `Session::motion_offsets`.
- Produces:
  - `pub(crate) struct layers::Cell<O> { pub x: i32, pub y: i32, pub shift: (i32, i32), pub pixels: [Option<O>; 64] }`
  - `pub(crate) struct layers::Layers<B, O>` with `new(view, border, fill)`, `bg_mut(x, y)`, fields `window: Vec<Option<B>>`, `cells: Vec<Cell<O>>`, and `compose(&self, scale, bg_shift, hidden) -> (Vec<B>, Vec<Option<O>>)`
  - `pub fn render_view_output_smooth(<render_view_output args>, offsets: &motion::Offsets) -> Result<Vec<u8>, String>` (size `vw·s × vh·s`)
  - `Session::render_palette_frame_smooth(&self, view, smooth: Smooth) -> (Vec<PalettePixel>, usize)`, the `usize` being the scale actually used (1 outside levels).

- [ ] **Step 1: Write `layers.rs` with tests**

```rust
//! Layered frames: a background with a border, an unshifted window layer and
//! pre-sampled OBJ cells, composed at an integer scale with per-layer shifts
//! in output pixels. At scale 1 with no shifts this is the plain GB picture.

/// An 8×8 OBJ cell at game position `(x, y)`, shifted by `shift` output
/// pixels; `None` pixels are transparent. Pushed in OAM priority order.
#[derive(Clone)]
pub(crate) struct Cell<O> {
    pub x: i32,
    pub y: i32,
    pub shift: (i32, i32),
    pub pixels: [Option<O>; 64],
}

pub(crate) struct Layers<B, O> {
    pub view: (usize, usize),
    pub border: usize,
    /// `(vw + 2·border) × (vh + 2·border)`, origin at `(-border, -border)`.
    pub bg: Vec<B>,
    /// The window (status bar), never shifted; `None` shows the background.
    pub window: Vec<Option<B>>,
    pub cells: Vec<Cell<O>>,
}

impl<B: Copy, O: Copy> Layers<B, O> {
    pub fn new(view: (usize, usize), border: usize, fill: B) -> Self {
        Self {
            view,
            border,
            bg: vec![fill; (view.0 + 2 * border) * (view.1 + 2 * border)],
            window: vec![None; view.0 * view.1],
            cells: Vec::new(),
        }
    }

    /// The background pixel at view coordinates `-border..view + border`.
    pub fn bg_mut(&mut self, x: i32, y: i32) -> &mut B {
        let b = self.border as i32;
        let w = self.view.0 + 2 * self.border;
        &mut self.bg[(y + b) as usize * w + (x + b) as usize]
    }

    /// Upsample by `scale`: the background shifted by `bg_shift`, the window
    /// over it, and the first cell pixel per output pixel that `hidden` does
    /// not reject (a rejected pixel leaves the slot to later cells).
    pub fn compose(
        &self,
        scale: usize,
        bg_shift: (i32, i32),
        hidden: impl Fn(&O, &B) -> bool,
    ) -> (Vec<B>, Vec<Option<O>>) {
        let (vw, vh) = self.view;
        let (w, h) = (vw * scale, vh * scale);
        let (s, b) = (scale as i32, self.border as i32);
        let (bw, bh) = ((vw + 2 * self.border) as i32, (vh + 2 * self.border) as i32);
        let mut bg = Vec::with_capacity(w * h);
        for y in 0..h as i32 {
            let gy = ((y - bg_shift.1).div_euclid(s) + b).clamp(0, bh - 1);
            for x in 0..w as i32 {
                let window = self.window[(y / s) as usize * vw + (x / s) as usize];
                bg.push(window.unwrap_or_else(|| {
                    let gx = ((x - bg_shift.0).div_euclid(s) + b).clamp(0, bw - 1);
                    self.bg[gy as usize * bw as usize + gx as usize]
                }));
            }
        }
        let mut obj = vec![None; w * h];
        for cell in &self.cells {
            for (i, p) in cell.pixels.iter().enumerate() {
                let Some(p) = p else { continue };
                let x0 = (cell.x + (i % 8) as i32) * s + cell.shift.0;
                let y0 = (cell.y + (i / 8) as i32) * s + cell.shift.1;
                for y in y0.max(0)..(y0 + s).min(h as i32) {
                    for x in x0.max(0)..(x0 + s).min(w as i32) {
                        let idx = y as usize * w + x as usize;
                        if obj[idx].is_none() && !hidden(p, &bg[idx]) {
                            obj[idx] = Some(*p);
                        }
                    }
                }
            }
        }
        (bg, obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(x: i32, y: i32, shift: (i32, i32), v: u8) -> Cell<u8> {
        Cell { x, y, shift, pixels: [Some(v); 64] }
    }

    #[test]
    fn a_half_pixel_background_shift_at_scale_two_moves_one_output_pixel() {
        let mut layers: Layers<u8, u8> = Layers::new((2, 1), 1, 0);
        layers.bg = vec![9, 9, 9, 9, 9, 1, 2, 9, 9, 9, 9, 9];
        let (bg, _) = layers.compose(2, (1, 0), |_, _| false);
        assert_eq!(&bg[..4], &[9, 1, 1, 2]);
        let (bg, _) = layers.compose(2, (0, 0), |_, _| false);
        assert_eq!(&bg[..4], &[1, 1, 2, 2]);
    }

    #[test]
    fn the_window_ignores_the_background_shift() {
        let mut layers: Layers<u8, u8> = Layers::new((2, 1), 1, 0);
        layers.window[1] = Some(7);
        let (bg, _) = layers.compose(1, (1, 0), |_, _| false);
        assert_eq!(bg[1], 7);
    }

    #[test]
    fn earlier_cells_win_and_hidden_pixels_leave_the_slot_free() {
        let mut layers: Layers<u8, u8> = Layers::new((8, 8), 0, 1);
        layers.cells.push(solid(0, 0, (0, 0), 5));
        layers.cells.push(solid(0, 0, (0, 0), 6));
        let (_, obj) = layers.compose(1, (0, 0), |_, _| false);
        assert_eq!(obj[0], Some(5));
        let (_, obj) = layers.compose(1, (0, 0), |&o, _| o == 5);
        assert_eq!(obj[0], Some(6));
    }

    #[test]
    fn cells_upsample_and_shift_in_output_pixels() {
        let mut layers: Layers<u8, u8> = Layers::new((8, 8), 0, 0);
        layers.cells.push(solid(-8, 0, (1, 0), 3));
        let (_, obj) = layers.compose(2, (0, 0), |_, _| false);
        assert_eq!(obj[0], Some(3), "last column, shifted in by one output px");
        assert_eq!(obj[1], None);
    }
}
```

Register in `render.rs`: `mod layers;`

- [ ] **Step 2: Run** `cargo test -p sml2-runtime layers` — expect PASS for the new tests (they only exercise `layers.rs`).

- [ ] **Step 3: Split the native renderer** in `crates/sml2-runtime/src/render.rs`

Add the pixel types and cell sampler (replacing `blit_obj_cell`, keep its doc comment's rules):

```rust
/// A native BG/window pixel: its mapped shade and colour index (priority).
#[derive(Clone, Copy)]
struct NativeBg {
    shade: u8,
    index: u8,
}

/// A native OBJ pixel: its mapped shade and attr bit 7.
#[derive(Clone, Copy)]
struct NativeObj {
    shade: u8,
    behind: bool,
}

/// One 8x8 OAM cell at screen `(x, y)`: colour 0 is transparent, attr bit5 =
/// X flip / bit6 = Y flip / bit4 = OBP1 / bit7 = BG priority, and OBJ tiles
/// use the `$8000` mode (`$8000 + t*16`).
#[allow(clippy::too_many_arguments)]
fn native_cell(
    objblock: &[u8],
    pal0: &[u8],
    pal1: &[u8],
    x: i32,
    y: i32,
    tile: u8,
    attr: u8,
    shift: (i32, i32),
) -> layers::Cell<NativeObj> {
    let (xf, yf) = (attr & 0x20 != 0, attr & 0x40 != 0);
    let pal = if attr & 0x10 != 0 { pal1 } else { pal0 };
    let t = tile as usize;
    let off = if t < 128 { t * 16 } else { 0x800 + (t - 128) * 16 };
    let bytes: [u8; 16] = objblock[off..off + 16].try_into().expect("objblock is $1000 bytes");
    let colors = video::tile_colors(&bytes);
    let pixels = std::array::from_fn(|i| {
        let (dx, dy) = (i % 8, i / 8);
        let c = colors[(if yf { 7 - dy } else { dy }) * 8 + if xf { 7 - dx } else { dx }];
        (c != 0).then(|| NativeObj {
            shade: pal[c as usize],
            behind: attr & 0x80 != 0,
        })
    });
    layers::Cell { x, y, shift, pixels }
}
```

Make `render_view_output` a thin wrapper and move its body into a layer builder:

```rust
#[allow(clippy::too_many_arguments)]
pub fn render_view_output(
    /* unchanged parameters */
) -> Result<Vec<u8>, String> {
    render_view_output_smooth(
        manifest, layout, overlay, bg, cam_x, cam_y, vw, vh, hud, spr, pal_override, output,
        &motion::Offsets::none(),
    )
}

/// [`render_view_output`] at `offsets.scale` output pixels per game pixel,
/// each layer shifted by `offsets`.
#[allow(clippy::too_many_arguments)]
pub fn render_view_output_smooth(
    /* same parameters as render_view_output */,
    offsets: &motion::Offsets,
) -> Result<Vec<u8>, String> {
    let layers = render_view_layers(
        manifest, layout, overlay, bg, cam_x, cam_y, vw, vh, hud, spr, pal_override, output, offsets,
    )?;
    let (bg, obj) = layers.compose(offsets.scale, offsets.bg, |o, b| o.behind && b.index != 0);
    Ok(bg.iter().zip(&obj).map(|(b, o)| o.map_or(b.shade, |o| o.shade)).collect())
}
```

`fn render_view_layers(...same params..., offsets: &motion::Offsets) -> Result<layers::Layers<NativeBg, NativeObj>, String>` is today's body with these mechanical edits:

1. Replace `let mut img = vec![shades[0]; vw * vh]; let mut bg_idx = vec![0u8; vw * vh];` with
   ```rust
   let border = offsets.border() as i32;
   let mut layers = layers::Layers::new((vw, vh), offsets.border(), NativeBg { shade: shades[0], index: 0 });
   ```
2. BG loop: iterate `for py in -border..vh as i32 + border` and `for px in -border..vw as i32 + border` (keep `sx`, `sy`, `ox`, `oy`, `span_*` computed from the real view; replace `py as i32`/`px as i32` with `py`/`px`). Replace the two writes with
   ```rust
   *layers.bg_mut(px, py) = NativeBg { shade: shades[col as usize], index: col };
   ```
3. HUD loop: replace the two writes with
   ```rust
   layers.window[idx] = Some(NativeBg { shade: shades[c as usize], index: c });
   ```
4. Drop `let mut taken = ...`. Every `blit_obj_cell(&mut img, &bg_idx, &mut taken, &s.objblock, &pal0, &pal1, X, Y, TILE, ATTR, vw, vh)` becomes
   `layers.cells.push(native_cell(&s.objblock, &pal0, &pal1, X, Y, TILE, ATTR, SHIFT))` with `SHIFT`:
   - player cells, streak counter: `offsets.player`
   - shield: `offsets.shield`
   - fireballs: iterate with `.enumerate()`; `offsets.fireball(i)`
   - bumpers: iterate with `.enumerate()`; `offsets.bumper(i)`
   - level objects: `offsets.object(inst.slot)`
   - boss OAM: `(0, 0)`
5. `Ok(img)` → `Ok(layers)`.

Add `use` for `motion` where needed (`pub mod motion;` is already declared in `render.rs`).

Rewrite the test `the_smaller_oam_x_wins_overlapping_obj_pixels` (update the `use super::{...}` list: drop `blit_obj_cell`, add `native_cell`, `layers`, `NativeObj`) so it builds cells instead of blitting:

```rust
        let mut layers: super::layers::Layers<u8, NativeObj> = super::layers::Layers::new((16, 8), 0, 0);
        for c in in_obj_priority_order(cells) {
            layers.cells.push(native_cell(
                &objblock, &pal, &pal, c.x as i32 - 8, c.y as i32 - 16, c.tile, c.attr, (0, 0),
            ));
        }
        let (bg, obj) = layers.compose(1, (0, 0), |_, _| false);
        let img: Vec<u8> = bg.iter().zip(&obj).map(|(b, o)| o.map_or(*b, |o| o.shade)).collect();
        assert_eq!(&img[..9], &[3, 3, 3, 3, 3, 3, 3, 3, 1]);
```

- [ ] **Step 4: Run** `cargo test -p sml2-runtime` — expect PASS. All existing palette/render tests prove the scale-1 path is byte-identical.

- [ ] **Step 5: Thread offsets through the session render** (`crates/sml2-runtime/src/render/session.rs`)

- `LevelContext::render_output(&self, vw, vh, output)` → add `offsets: &motion::Offsets`, return `Result<(Vec<u8>, usize), Box<dyn Error>>`: the ending branch returns `(frame, 1)`; the level branch calls `render_view_output_smooth(..., output, offsets)` and returns `(frame, offsets.scale)`. `LevelContext::render_frame` passes `&Offsets::none()` and returns `.0`.
- `Session::render_output(&self, view, output)` → `render_output(&self, view, output, smooth: Smooth) -> (Vec<u8>, usize)`. `InLevel`/`Demo` arms: `ctx.render_output(vw, vh, output, &self.motion_offsets(smooth)).unwrap_or_else(|_| (vec![output.blank(3); vw * vh], 1))`; every other arm returns `(frame, 1)`.
- `render_frame` and `render_palette_frame` call with `Smooth::OFF` and take `.0`.
- Add:

```rust
    /// [`Session::render_palette_frame`] drawn `smooth.alpha` of the way from
    /// the previous tick, with the scale actually used (1 outside levels).
    pub fn render_palette_frame_smooth(&self, view: (usize, usize), smooth: Smooth) -> (Vec<PalettePixel>, usize) {
        let (frame, scale) = self.render_output(view, OutputMode::PaletteIndices, smooth);
        (frame.into_iter().map(PalettePixel::from_token).collect(), scale)
    }
```

Add to `palette_tests.rs`:

```rust
#[test]
fn smoothing_at_a_full_tick_is_the_native_frame_upsampled() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    for _ in 0..30 {
        session.advance(FrameInput::default());
    }
    let native = session.render_palette_frame((256, 144));
    let (smooth, scale) =
        session.render_palette_frame_smooth((256, 144), crate::render::motion::Smooth { alpha: 1.0, scale: 2 });
    assert_eq!(scale, 2);
    let upsampled: Vec<_> = (0..288)
        .flat_map(|y| (0..512).map(move |x| (x, y)))
        .map(|(x, y)| native[(y / 2) * 256 + x / 2])
        .collect();
    assert_eq!(smooth, upsampled);
}

#[test]
fn smoothing_shifts_the_level_but_not_the_status_bar() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    for _ in 0..30 {
        session.advance(FrameInput::default());
    }
    let mut prev = session
        .level_ctx()
        .and_then(crate::render::motion::MotionSnapshot::of)
        .unwrap();
    prev.cam.0 = prev.cam.0.wrapping_sub(4);
    session.motion_prev = Some(prev);
    let half = crate::render::motion::Smooth { alpha: 0.5, scale: 1 };
    let (shifted, _) = session.render_palette_frame_smooth((160, 144), half);
    let native = session.render_palette_frame((160, 144));
    assert_ne!(shifted[..160 * 136], native[..160 * 136]);
    assert_eq!(shifted[160 * 136..], native[160 * 136..]);
}
```

- [ ] **Step 6: Run** `cargo test -p sml2-runtime` and `cargo test -p sml2-desktop` (its backdrop uses `render_view_output`) — expect PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/sml2-runtime/src/render/layers.rs crates/sml2-runtime/src/render.rs crates/sml2-runtime/src/render/session.rs crates/sml2-runtime/src/render/palette_tests.rs
git commit -m "feat(runtime): compose native level frames from shiftable layers"
```

---

### Task 4: DX renderer layers

**Files:**
- Modify: `crates/sml2-runtime/src/render/dx.rs` (`render`, `draw_cells`, `blit`, its test, `render_dx_frame`)
- Test: `crates/sml2-runtime/tests/dx_preview.rs`

**Interfaces:**
- Consumes: `layers::{Layers, Cell}`, `motion::{Offsets, Smooth}`, `Session::motion_offsets`.
- Produces: `Session::render_dx_frame_smooth(&self, dx: &Presentation, character: Character, view: (usize, usize), smooth: Smooth) -> Result<(Vec<[u8; 3]>, usize), String>`. `render_dx_frame` keeps its signature and calls it with `Smooth::OFF`, returning `.0`.

- [ ] **Step 1: Write the failing integration test** in `crates/sml2-runtime/tests/dx_preview.rs` (reuse the file's `session()` and `synthetic_bundle()` helpers):

```rust
#[test]
fn dx_smoothing_at_a_full_tick_is_the_dx_frame_upsampled() {
    let bundle = synthetic_bundle();
    let mut game = session();
    game.begin_level(0, 0).unwrap();
    for _ in 0..30 {
        game.advance(FrameInput::default());
    }
    let native = game.render_dx_frame(&bundle, Mario, (256, 144)).unwrap();
    let (smooth, scale) = game
        .render_dx_frame_smooth(
            &bundle,
            Mario,
            (256, 144),
            sml2_runtime::render::motion::Smooth { alpha: 1.0, scale: 3 },
        )
        .unwrap();
    assert_eq!(scale, 3);
    assert_eq!(smooth.len(), 768 * 432);
    assert!((0..432).all(|y| (0..768).all(|x| smooth[y * 768 + x] == native[(y / 3) * 256 + x / 3])));
}
```

- [ ] **Step 2: Run** `cargo test -p sml2-runtime --test dx_preview dx_smoothing` — expect compile FAIL (`render_dx_frame_smooth` missing).

- [ ] **Step 3: Convert `dx.rs`**

Replace `blit`/`draw_cells` with a cell sampler and a cell pusher:

```rust
/// One 8x8 CGB OBJ cell at screen `(x, y)`; colour 0 is transparent.
fn dx_cell(
    banks: [&[u8]; 2],
    x: i32,
    y: i32,
    tile: u8,
    attr: u8,
    shift: (i32, i32),
) -> Result<Cell8, String> {
    let attr = CgbAttributes::from_byte(attr);
    let mut pixels = [None; 64];
    for (i, pixel) in pixels.iter_mut().enumerate() {
        let index = cgb::sample_tile(banks, tile as usize * 16, attr, (i % 8) as u8, (i / 8) as u8)
            .map_err(|e| e.to_string())?;
        if index.color() != 0 {
            *pixel = Some(ObjectPixel { index, behind_background: attr.priority() });
        }
    }
    Ok(Cell8 { x, y, shift, pixels })
}

type Cell8 = super::layers::Cell<ObjectPixel>;

fn push_cells(
    out: &mut Vec<Cell8>,
    banks: [&[u8]; 2],
    cells: Vec<Cell>,
    pan: u8,
    offset: (i32, i32),
    shift: (i32, i32),
) -> Result<(), String> {
    for cell in cells {
        out.push(dx_cell(
            banks,
            cell.x as i32 - 8 + offset.0,
            cell.y.wrapping_sub(pan) as i32 - 16 + offset.1,
            cell.tile,
            cell.attr,
            shift,
        )?);
    }
    Ok(())
}
```

(The existing `dx_cell(ctx, level, cx, cy)` helper name collides; name the new sampler `obj_cell` instead if so.)

In `render(ctx, dx, character, view)` add a parameter `offsets: &super::motion::Offsets` and:

1. Replace `let mut background = vec![BackgroundPixel {..}; vw * vh];` with
   ```rust
   let border = offsets.border() as i32;
   let blank = BackgroundPixel { index: IndexedPixel::new(0, 0).unwrap(), priority: false };
   let mut layers = super::layers::Layers::new(view, offsets.border(), blank);
   ```
2. BG loop: `for py in -border..vh as i32 + border`, `for px in -border..vw as i32 + border`; use `px`/`py` directly where `px as i32`/`py as i32` appear; write `*layers.bg_mut(px, py) = BackgroundPixel { .. }`.
3. HUD loop: write `layers.window[py as usize * vw + x as usize] = Some(BackgroundPixel { .. })`.
4. Drop `let mut objects = vec![None; vw * vh];`. `draw_cells(&mut objects, view, banks, cells, PAN, offset)` → `push_cells(&mut layers.cells, banks, cells, PAN, offset, SHIFT)`, and the object `blit(&mut objects, view, banks, X, Y, TILE, ATTR)` → `layers.cells.push(obj_cell(banks, X, Y, TILE, ATTR, offsets.object(inst.slot))?)`. `SHIFT`s: fireballs `offsets.fireball(i)` (enumerate), player and streak counter `offsets.player`, shield `offsets.shield`, bumpers `offsets.bumper(i)` (enumerate).
5. Scene-reload blank: `return Ok(vec![[255; 3]; vw * vh * offsets.scale * offsets.scale]);`
6. End:
   ```rust
   let (background, objects) = layers.compose(offsets.scale, offsets.bg, |_, _| false);
   cgb::compose_frame(&background, &objects, true, &palettes).map_err(|e| e.to_string())
   ```

Rename `render_dx_frame`'s body into `render_dx_frame_smooth` returning `(frame, scale)`: the title / file select / overworld arms return `(frame, 1)`; the level arm calls `render(ctx, dx, character, view, &self.motion_offsets(smooth))` and returns its scale `self.motion_offsets(smooth).scale` (compute the offsets once into a local). Then:

```rust
    pub fn render_dx_frame(&self, dx: &Presentation, character: Character, view: (usize, usize)) -> Result<Vec<[u8; 3]>, String> {
        self.render_dx_frame_smooth(dx, character, view, super::motion::Smooth::OFF).map(|(frame, _)| frame)
    }
```

Rewrite the unit test `cgb_overlay_uses_mapping_bank_and_earlier_oam_even_when_behind`:

```rust
    #[test]
    fn cgb_overlay_uses_mapping_bank_and_earlier_oam_even_when_behind() {
        let bank0 = [0; 32];
        let mut bank1 = [0; 32];
        bank1[..16].copy_from_slice(&[0x80, 0].repeat(8));
        let blank = BackgroundPixel { index: IndexedPixel::new(0, 0).unwrap(), priority: false };
        let mut layers = crate::render::layers::Layers::new((160, 144), 0, blank);
        layers.cells.push(obj_cell([&bank0, &bank1], 0, 0, 0, 0x8c, (0, 0)).unwrap());
        layers.cells.push(obj_cell([&bank0, &bank1], 0, 0, 0, 0x0e, (0, 0)).unwrap());
        let (_, objects) = layers.compose(1, (0, 0), |_, _| false);
        assert_eq!(
            objects[0],
            Some(ObjectPixel { index: IndexedPixel::new(1, 4).unwrap(), behind_background: true })
        );
        assert!(objects[1].is_none());
    }
```

- [ ] **Step 4: Run** `cargo test -p sml2-runtime` and `cargo test -p sml2-desktop` — expect PASS (existing DX tests prove scale-1 identity).

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-runtime/src/render/dx.rs crates/sml2-runtime/tests/dx_preview.rs
git commit -m "feat(runtime): smooth DX level frames through shared layers"
```

---

### Task 5: Desktop setting, tick fraction and presentation

**Files:**
- Modify: `crates/sml2-desktop/src/settings.rs` (field, default, version-two test list)
- Modify: `crates/sml2-desktop/src/timing.rs` (`TickBudget::fraction`)
- Modify: `crates/sml2-desktop/src/app/panels.rs` (`view_controls` row)
- Modify: `crates/sml2-desktop/src/app/mod.rs` (`update`, `frame_image`, `colorized`)
- Modify: `docs/plans/2026-10-22-widescreen-zoom-design.md` (one-line pointer to the new spec)

**Interfaces:**
- Consumes: `motion::{Smooth, pick_scale}`, `Session::render_palette_frame_smooth`, `Session::render_dx_frame_smooth`.
- Produces: `Settings::motion_smoothing: bool`; `TickBudget::fraction(&self) -> f32`.

- [ ] **Step 1: Tests first**

In `timing.rs` tests:

```rust
    #[test]
    fn fraction_is_the_progress_into_the_next_tick() {
        let mut budget = TickBudget::default();
        assert_eq!(budget.fraction(), 0.0);
        assert_eq!(budget.advance(Duration::from_micros(25_000), true), 1);
        assert!((budget.fraction() - 0.5).abs() < 0.001);
        budget.advance(Duration::ZERO, false);
        assert_eq!(budget.fraction(), 0.0);
    }
```

In `settings.rs`, add `"motion_smoothing"` to the field list in `version_two_settings_restore_optional_dx_selection`.

- [ ] **Step 2: Run** `cargo test -p sml2-desktop` — expect compile FAIL.

- [ ] **Step 3: Implement**

`timing.rs`:

```rust
    /// How far into the next tick the accumulated time is, in `[0, 1)`.
    pub fn fraction(&self) -> f32 {
        self.phase as f32 / 1_000_000_000.0
    }
```

`settings.rs`: after `camera_zoom`:

```rust
    /// Draw levels between the last two ticks on fast displays (Mods).
    pub motion_smoothing: bool,
```

default `motion_smoothing: false`.

`panels.rs` `view_controls`, after the camera-zoom row:

```rust
        let adjust = self.menu.row_adjustment();
        let smoothing = self.settings.motion_smoothing;
        if self.menu.row(
            ui,
            "Motion smoothing",
            Value::Choice(if smoothing { "On" } else { "Off" }.into()),
        ) || adjust != 0
        {
            self.settings.motion_smoothing = !smoothing;
        }
```

and append to the note: `" Motion smoothing draws levels between frames on displays faster than 60 Hz, one frame behind."` (Check how `Value` rows for booleans are written elsewhere in `panels.rs`, e.g. the DX mode row, and match that style if it differs.)

`app/mod.rs`:

- In `update`, capture `let stepped = self.step && focused;` before `self.step = false;`, then compute the smoothing request right before `frame_image`:

```rust
        let smooth = if self.settings.motion_smoothing && running && !stepped {
            sml2_runtime::render::motion::Smooth {
                alpha: self.budget.fraction(),
                scale: sml2_runtime::render::motion::pick_scale(self.settings.view(), self.effective_scale),
            }
        } else {
            sml2_runtime::render::motion::Smooth::OFF
        };
        if let Some(image) = self.frame_image(smooth) {
```

- Replace `ctx.request_repaint_after(Duration::from_millis(8));` with:

```rust
        if self.settings.motion_smoothing && running {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(8));
        }
```

- `frame_image(&mut self, smooth: Smooth)`: DX arm calls `render_dx_frame_smooth(dx, self.settings.character, view, smooth)` and builds `ColorImage::new([view.0 * scale, view.1 * scale], ..)`; native arm calls `render_palette_frame_smooth(view, smooth)` and passes `(view.0 * scale, view.1 * scale)` to `colorized`.

`game_view` is unchanged: the destination rectangle is the view times `effective_scale`, and `scale` divides `effective_scale`, so the GPU upsamples by a whole number and `screenshot::verify` still applies.

Add an app test in `crates/sml2-desktop/src/app/tests.rs` (same setup as `actual_session_frames_use_the_extracted_cartridge_palette_for_every_filter`). The Mods test above it walks rows by index; if adding the row shifts nothing before it, leave it alone, otherwise update its row numbers.

```rust
#[test]
fn motion_smoothing_submits_a_frame_at_the_chosen_scale() {
    let (mut app, _, path) = app();
    app.apply_debug(DebugAction::WarpLevel(4));
    let smooth = sml2_runtime::render::motion::Smooth { alpha: 1.0, scale: 2 };
    let image = app.frame_image(smooth).unwrap();
    assert_eq!(image.size, [320, 288]);
    let off = app.frame_image(sml2_runtime::render::motion::Smooth::OFF).unwrap();
    assert_eq!(off.size, [160, 144]);
    let _ = std::fs::remove_dir_all(path);
}
```

`docs/plans/2026-10-22-widescreen-zoom-design.md`: append under its zoom section: `200% and motion smoothing: see 2026-10-22-zoom200-motion-smoothing-design.md.`

- [ ] **Step 4: Run** `cargo test -p sml2-desktop` then `cargo test -p sml2-runtime` — expect PASS.

- [ ] **Step 5: Full gate**

```bash
cargo test -p sml2-core
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

Expected: all tests pass; sweep reports the same results as before this plan (byte-identical).

- [ ] **Step 6: Commit**

```bash
git add crates/sml2-desktop/src/settings.rs crates/sml2-desktop/src/timing.rs crates/sml2-desktop/src/app/panels.rs crates/sml2-desktop/src/app/mod.rs crates/sml2-desktop/src/app/tests.rs docs/plans/2026-10-22-widescreen-zoom-design.md
git commit -m "feat(desktop): add motion smoothing setting and smoothed presentation"
```
