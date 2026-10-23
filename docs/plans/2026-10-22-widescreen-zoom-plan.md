# Widescreen and Camera Zoom Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two desktop Mods settings — Widescreen (Off / 16:9) and Camera zoom (100 / 125 / 150 %) — that enlarge the game view, widen object spawning to match, and keep 160×144 byte-identical.

**Architecture:** The view size `(w, h)` is chosen by the desktop, passed to `Session::set_view` (which feeds a spawn margin to the live level's `ObjectSim`) and to every render entry point. Level rendering already takes a view size; it gains a wider tile window, sprite placement that does not depend on 8-bit screen bytes, and a bottom-anchored status bar. Fixed screens are letterboxed; the hub map is stitched from native-size renders.

**Tech Stack:** Rust workspace (`sml2-core`, `sml2-runtime`, `sml2-desktop`), egui/eframe.

Spec: [`2026-10-22-widescreen-zoom-design.md`](2026-10-22-widescreen-zoom-design.md).

## Global Constraints

- At view `(160, 144)` every frame, test and `captures/sweep.sh` result is unchanged. Any new branch must reduce to the existing code path at 160×144.
- View sizes: Off → 160×144 / 200×180 / 240×216; 16:9 → 256×144 / 320×180 / 384×216.
- Object windows grow by `mx = (w − 160) / 2`, `my = (h − 144) / 2`: act X/Y `$60`, scan X `$70`, alive X/Y `$A0`.
- Attract demos keep a zero margin (their recorded inputs assume native spawns).
- Status bar: 160 px wide, horizontally centred, bottom of the view.
- Bars (letterbox fill): shade 3 (`OutputMode::blank(3)`) natively, `[0, 0, 0]` in DX.
- Commits: Conventional Commits, one per task, staging only that task's files.
- Do not load `captures/*.json`, `PORT_QUEUE.done.json` or whole `DESIGN.md`.

## Spec deviation

Boss OAM cells are clipped by the core boss render pass (`$66000`'s `$A0`/`$A8` bounds). That is simulation output with parity tests; this plan keeps it and only shifts boss cells by the view margin. Arenas fit the native screen, so nothing visible is lost. Task 6 records this in the spec.

## File map

| File | Change |
|---|---|
| `crates/sml2-core/src/objects/mod.rs` | `view_margin` field, widened windows, `fill_view_margin` |
| `crates/sml2-core/src/objects/stream.rs` | extract `record_skipped` / `spawn_record` from `scan` |
| `crates/sml2-core/src/objects/tests/engine.rs` | margin tests |
| `crates/sml2-runtime/src/render/view.rs` (new) | `NATIVE`, `margin`, `enlarged`, `window_span`, `copy_rect`, `letterbox`, `fixed_screen` |
| `crates/sml2-runtime/src/render.rs` | span-aware window, anchored sprites, bottom HUD |
| `crates/sml2-runtime/src/render/session.rs` | fixed-screen letterbox, `overworld_view` stitching |
| `crates/sml2-runtime/src/session.rs` | `view` field, `set_view`, margin propagation |
| `crates/sml2-runtime/src/render/dx.rs` | view-sized DX level/map/screens |
| `crates/sml2-runtime/src/tests/widescreen.rs` (new) | session-level tests |
| `crates/sml2-desktop/src/settings.rs`, `settings/view.rs` (new) | `Widescreen`, `CameraZoom`, `Settings::view` |
| `crates/sml2-desktop/src/app/{mod,panels,screenshot}.rs`, `timing.rs` | view-sized presentation and Mods rows |

---

### Task 1: Object engine spawn margin

**Files:**
- Modify: `crates/sml2-core/src/objects/mod.rs` (struct ~line 639, `new` ~754, `engine` ~1215)
- Modify: `crates/sml2-core/src/objects/stream.rs`
- Test: `crates/sml2-core/src/objects/tests/engine.rs`

**Interfaces:**
- Produces: `pub view_margin: (u16, u16)` on `ObjectSim` (default `(0, 0)`).

- [ ] **Step 1: Write the failing tests** — append to `objects/tests/engine.rs`:

```rust
fn margin_record(x: u16) -> SpawnRecord {
    SpawnRecord {
        x,
        y: 0x0090,
        anim_index: 16,
        mode_flag: 0,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [(x >> 8) as u8, x as u8, 0, 0, 0x90, 16],
    }
}

#[test]
fn a_view_margin_spawns_records_in_the_widened_band_only() {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    // Native scan edge is cam + $70; a 384-px view widens it by 112.
    let records = vec![
        margin_record(cam - 0x70 - 0x40), // left band
        margin_record(cam + 0x40),        // inside the native window: never filled
        margin_record(cam + 0x70 + 0x40), // right band
        margin_record(cam + 0x70 + 0x78), // beyond the widened edge
    ];
    let mut obj = ObjectSim::new(records, cam);
    obj.configure(0);
    obj.view_margin = (112, 36);
    obj.step(cam, 0x0100, &layout, &t, &MarioContact::default(), false);
    assert_eq!(obj.spawned, vec![true, false, true, false]);
    assert_eq!(obj.standby, 2);
}

#[test]
fn a_zero_margin_leaves_a_still_camera_without_spawns() {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    let mut obj = ObjectSim::new(vec![margin_record(cam + 0x70 + 0x40)], cam);
    obj.configure(0);
    obj.step(cam, 0x0100, &layout, &t, &MarioContact::default(), false);
    assert_eq!(obj.spawned, vec![false]);
}

#[test]
fn a_view_margin_keeps_active_slots_alive_past_the_native_edge() {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    let mut obj = ObjectSim::new(Vec::new(), cam);
    obj.configure(0);
    obj.view_margin = (112, 36);
    obj.active = 1;
    let s = &mut obj.slots[0];
    s.b[B_STATE] = 2;
    s.b[B_RECHI] = 0xAD; // not stream-backed
    set_x(&mut s.b, cam + 0xA0 + 0x40); // past native alive, inside widened
    set_y(&mut s.b, 0x0100);
    for _ in 0..4 {
        obj.step(cam, 0x0100, &layout, &t, &MarioContact::default(), false);
    }
    assert_ne!(obj.slots[0].b[B_STATE], 0, "the widened alive window keeps it");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sml2-core view_margin`
Expected: compile error `no field view_margin on ObjectSim`.

- [ ] **Step 3: Extract the spawn body in `objects/stream.rs`**

Replace the block from `// Record at the edge: the mode byte gates` through `self.step_stream(forward);` (end of loop body) with:

```rust
            if self.record_skipped(ri) {
                if !self.step_stream(forward) {
                    return;
                }
                continue;
            }
            if !self.spawn_record(ri, self.stream_ptr(), tables) {
                return; // no free slot: scan stops ($8C93 → $AF)
            }
            // The last record leaves the pointer parked on itself (see
            // `advance_stream`): the engine's `$FF` terminator check refuses
            // the store, and the re-scan then walks into the same record with
            // the post-spawn marker set and stops there every frame. A
            // backward scan (`$8D99`) steps down instead, so a second record
            // in the same column spawns in the same pass.
            self.step_stream(forward);
```

and add after `scan`:

```rust
    /// The mode byte gates (and the engine's post-spawn marker 2 blocks
    /// re-spawns on a later pass).
    pub(super) fn record_skipped(&self, ri: usize) -> bool {
        let m = self.records[ri].mode_flag;
        (if self.easy_mode {
            m & 0x0F != 0
        } else {
            m & 0x0E != 0
        }) || self.spawned[ri]
    }

    /// `$8CA8`: 6 record bytes, record pointer (+2), state = 1, clear
    /// +9..+31, marker 2, `$8240` sprite derive, `$81CF` hitbox. `ptr` is the
    /// record's stream address. False when every slot is taken.
    pub(super) fn spawn_record(&mut self, ri: usize, ptr: u16, tables: &ObjectTables) -> bool {
        let Some(slot) = self.slots.iter().position(|s| s.b[B_STATE] == 0) else {
            return false;
        };
        let r = self.records[ri];
        let mut s = Staging {
            b: [0; 32],
            ..Default::default()
        };
        s.b[B_XHI] = (r.x >> 8) as u8;
        s.b[B_XLO] = r.x as u8;
        s.b[B_MARKER] = r.mode_flag;
        s.b[B_YHI] = (r.y >> 8) as u8;
        s.b[B_YLO] = r.y as u8;
        s.b[B_DEF] = r.anim_index;
        // The engine stores (streamPtrHi, streamPtrLo + 2) with no carry
        // into the hi byte.
        s.b[B_RECHI] = (ptr >> 8) as u8;
        s.b[B_RECLO] = (ptr as u8).wrapping_add(2);
        s.b[B_STATE] = 1;
        self.derive_sprite(&mut s, tables);
        self.refresh_hitbox(&mut s, tables, false);
        self.slots[slot].b = s.b;
        self.records[ri].mode_flag = 2;
        self.spawned[ri] = true;
        self.standby = self.standby.wrapping_add(1);
        true
    }
```

- [ ] **Step 4: Run the existing stream tests to confirm the extraction is behaviour-neutral**

Run: `cargo test -p sml2-core stream`
Expected: PASS (the new tests still fail to compile only if run together; run with the filter `stream`).

- [ ] **Step 5: Add the margin to `objects/mod.rs`**

In `pub struct ObjectSim`, after `pub unported: Vec<u32>,`:

```rust
    /// Extra half-width/half-height of an enlarged view. Not ROM state: zero
    /// keeps every window at the engine's own size.
    pub view_margin: (u16, u16),
```

In `ObjectSim::new`, after `unported: Vec::new(),` add `view_margin: (0, 0),`.

In `fn engine`, replace the five window lines with:

```rust
        let (mx, my) = (i32::from(self.view_margin.0), i32::from(self.view_margin.1));
        let act_x = Self::window(cam_x, 0x60 + mx);
        let act_y = Self::window(cam_y, 0x60 + my);
        let scan_x = Self::window(cam_x, 0x70 + mx);
        let alive_x = Self::window(cam_x, 0xA0 + mx);
        let alive_y = Self::window(cam_y, 0xA0 + my);
```

Replace the scan tail:

```rust
        // $817D: spawn scan + previous-camera update.
        if self.scan_dir != 0 && (self.spawn_x >> 8) != 0xFF {
            self.scan(tables);
        }
        self.fill_view_margin(cam_x, tables);
        self.prev_cam_x = cam_x;
```

Add next to `remove_slot`:

```rust
    /// An enlarged view uncovers records beyond the native scan edges that
    /// the edge scan never crosses (level start, warps, respawns).
    fn fill_view_margin(&mut self, cam_x: u16, tables: &ObjectTables) {
        let mx = i32::from(self.view_margin.0);
        if mx == 0 || self.stream_cursor_cleared {
            return;
        }
        let native = Self::window(cam_x, 0x70);
        let wide = Self::window(cam_x, 0x70 + mx);
        for ri in 0..self.records.len() {
            let x = self.records[ri].x;
            let in_band = (x >= wide.0 && x < native.0) || (x > native.1 && x <= wide.1);
            if !in_band || self.record_skipped(ri) {
                continue;
            }
            let ptr = STREAM_BASE.wrapping_add((ri * RECORD_LEN) as u16);
            if !self.spawn_record(ri, ptr, tables) {
                return;
            }
        }
    }
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p sml2-core`
Expected: all PASS, including the three new tests.

- [ ] **Step 7: Commit**

```bash
git add crates/sml2-core/src/objects/mod.rs crates/sml2-core/src/objects/stream.rs crates/sml2-core/src/objects/tests/engine.rs
git commit -m "feat(objects): widen spawn windows for enlarged views"
```

---

### Task 2: View helpers and the level renderer

**Files:**
- Create: `crates/sml2-runtime/src/render/view.rs`
- Modify: `crates/sml2-runtime/src/render.rs` (`window_origin`/`window_coord` ~71-80, `render_view_output` ~126-440)
- Modify: `crates/sml2-runtime/src/render/dx.rs` (only the two call-site signatures in this task)
- Test: `crates/sml2-runtime/src/render/view.rs` (unit), `crates/sml2-runtime/src/tests/widescreen.rs` (new, registered in `tests/mod.rs`)

**Interfaces:**
- Produces (in `crate::render::view`):
  - `pub const NATIVE: (usize, usize) = (160, 144);`
  - `pub fn margin(view: (usize, usize)) -> (i32, i32)`
  - `pub fn enlarged(view: (usize, usize)) -> bool`
  - `pub fn window_span(len: usize) -> i32`
  - `pub fn copy_rect<P: Copy>(dst: &mut [P], dst_size: (usize, usize), src: &[P], src_w: usize, rect: (usize, usize, usize, usize), at: (i32, i32))`
  - `pub fn letterbox<P: Copy>(src: &[P], src_size: (usize, usize), view: (usize, usize), fill: P) -> Vec<P>`
  - `pub fn fixed_screen<P: Copy>(view: (usize, usize), fill: P, render: impl FnOnce((usize, usize)) -> Vec<P>) -> Vec<P>`
- Changes: `window_origin(view: i32, world: i32, span: i32)`, `window_coord(v: i32, origin: i32, span: i32)` (both `pub(crate)` in `render.rs`).

- [ ] **Step 1: Write `render/view.rs` with its unit tests first**

```rust
//! View-size arithmetic shared by the native and DX renderers.

pub const NATIVE: (usize, usize) = (160, 144);

/// How far an enlarged view's native rect sits from its top-left corner.
pub fn margin(view: (usize, usize)) -> (i32, i32) {
    ((view.0 as i32 - 160) / 2, (view.1 as i32 - 144) / 2)
}

/// A view at least as large as native in both axes, and larger in one.
pub fn enlarged(view: (usize, usize)) -> bool {
    view.0 >= NATIVE.0 && view.1 >= NATIVE.1 && view != NATIVE
}

/// The tilemap window a view samples through: the engine's 256 px, or the
/// view itself (rounded to whole metatiles) when it is wider.
pub fn window_span(len: usize) -> i32 {
    ((len as i32 + 15) & !15).max(256)
}

/// Copy `rect = (x, y, w, h)` of a `src_w`-wide image to `at` in `dst`,
/// clipped to `dst_size`.
pub fn copy_rect<P: Copy>(
    dst: &mut [P],
    dst_size: (usize, usize),
    src: &[P],
    src_w: usize,
    rect: (usize, usize, usize, usize),
    at: (i32, i32),
) {
    let (x0, y0, w, h) = rect;
    for dy in 0..h {
        let ty = at.1 + dy as i32;
        if ty < 0 || ty >= dst_size.1 as i32 {
            continue;
        }
        for dx in 0..w {
            let tx = at.0 + dx as i32;
            if tx < 0 || tx >= dst_size.0 as i32 {
                continue;
            }
            dst[ty as usize * dst_size.0 + tx as usize] = src[(y0 + dy) * src_w + x0 + dx];
        }
    }
}

/// `src` centred in a `view`-sized frame filled with `fill`.
pub fn letterbox<P: Copy>(src: &[P], src_size: (usize, usize), view: (usize, usize), fill: P) -> Vec<P> {
    let mut out = vec![fill; view.0 * view.1];
    let at = (
        (view.0 as i32 - src_size.0 as i32) / 2,
        (view.1 as i32 - src_size.1 as i32) / 2,
    );
    copy_rect(&mut out, view, src, src_size.0, (0, 0, src_size.0, src_size.1), at);
    out
}

/// A screen with native-only art: rendered at 160×144 and letterboxed into
/// an enlarged view; any other view renders as before.
pub fn fixed_screen<P: Copy>(
    view: (usize, usize),
    fill: P,
    render: impl FnOnce((usize, usize)) -> Vec<P>,
) -> Vec<P> {
    if enlarged(view) {
        letterbox(&render(NATIVE), NATIVE, view, fill)
    } else {
        render(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_and_margins_match_the_view_table() {
        assert_eq!(margin((384, 216)), (112, 36));
        assert_eq!(margin(NATIVE), (0, 0));
        assert_eq!(window_span(160), 256);
        assert_eq!(window_span(256), 256);
        assert_eq!(window_span(320), 320);
        assert_eq!(window_span(384), 384);
        assert!(enlarged((256, 144)) && !enlarged(NATIVE) && !enlarged((16, 16)));
    }

    #[test]
    fn letterbox_centres_with_bars() {
        let out = letterbox(&[1u8; 4], (2, 2), (4, 4), 0);
        assert_eq!(out, vec![0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0]);
    }
}
```

Register it in `render.rs` next to `mod dx;`: `pub mod view;`.

- [ ] **Step 2: Run the unit tests**

Run: `cargo test -p sml2-runtime render::view`
Expected: PASS.

- [ ] **Step 3: Write failing level-render tests** — create `crates/sml2-runtime/src/tests/widescreen.rs` and add `mod widescreen;` to `crates/sml2-runtime/src/tests/mod.rs`:

```rust
//! Enlarged views: native parity, sprite centring, tile window, status bar.

use crate::render::view::NATIVE;
use crate::session::{FrameInput, Session, SessionAssets, SessionState};

fn level(id: u8) -> Session {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    session.begin_level(id, 0).expect("level loads");
    for _ in 0..90 {
        session.advance(FrameInput::default());
    }
    session
}

fn crop(frame: &[u8], view: (usize, usize), rect: (usize, usize, usize, usize)) -> Vec<u8> {
    let (x0, y0, w, h) = rect;
    (y0..y0 + h)
        .flat_map(|y| frame[y * view.0 + x0..y * view.0 + x0 + w].to_vec())
        .collect()
}

#[test]
fn the_centre_of_an_enlarged_level_view_matches_native_above_the_status_bar() {
    let session = level(1);
    let native = session.render_frame(NATIVE);
    for view in [(256, 144), (320, 180), (384, 216), (200, 180), (240, 216)] {
        let wide = session.render_frame(view);
        let (mx, my) = crate::render::view::margin(view);
        assert_eq!(
            crop(&wide, view, (mx as usize, my as usize, 160, 136)),
            crop(&native, NATIVE, (0, 0, 160, 136)),
            "{view:?}"
        );
    }
}

#[test]
fn the_status_bar_sits_at_the_bottom_centre_of_an_enlarged_view() {
    let session = level(1);
    let native = session.render_frame(NATIVE);
    let view = (384, 216);
    let wide = session.render_frame(view);
    assert_eq!(
        crop(&wide, view, (112, 208, 160, 8)),
        crop(&native, NATIVE, (0, 136, 160, 8))
    );
}

#[test]
fn a_384_px_level_view_does_not_repeat_columns() {
    let session = level(1);
    let view = (384, 216);
    let frame = session.render_frame(view);
    let column = |x: usize| (0..200).map(|y| frame[y * view.0 + x]).collect::<Vec<_>>();
    let repeats = (0..128).filter(|&x| column(x) == column(x + 256)).count();
    assert!(repeats < 128, "every column 256 px apart is identical");
}
```

- [ ] **Step 4: Run them to verify they fail**

Run: `cargo test -p sml2-runtime widescreen`
Expected: FAIL — the centre test fails on Mario's cells (drawn at native coordinates), the status-bar test fails (bar drawn mid-view), the repeat test fails (256-px wrap).

- [ ] **Step 5: Span-aware tile window in `render.rs`**

Replace `window_origin` and `window_coord` with:

```rust
pub(crate) fn window_origin(view: i32, world: i32, span: i32) -> i32 {
    view.clamp(0, world - span)
}

/// A screen row/column seen through that window: itself inside the level, the window's own
/// rows again outside it. A span is a whole number of 16-px metatiles and of 8-px
/// tiles, so the wrap keeps both grids and the fine scroll intact.
pub(crate) fn window_coord(v: i32, origin: i32, span: i32) -> i32 {
    origin + (v - origin).rem_euclid(span)
}
```

(Keep the existing doc comment above `window_origin` unchanged.) In `render_view_output`:

```rust
    let (span_x, span_y) = (view::window_span(vw), view::window_span(vh));
    let (ox, oy) = (window_origin(sx, world_w, span_x), window_origin(sy, world_h, span_y));
```

and update the four other calls in the BG loop to pass `span_x` / `span_y` (`window_coord(sx + px as i32, ox, span_x)`, `window_coord(sy + py as i32, oy, span_y)`, and in the pipe branch `window_coord(x, window_origin(vx, world_w, span_x), span_x)`, `window_coord(y, window_origin(vy, world_h, span_y), span_y)`).

In `render/dx.rs`, update the same calls with `256` for both spans for now (Task 4 sizes them): `window_origin(sx, world_w, 256)`, `window_coord(..., ox, 256)`, etc.

- [ ] **Step 6: Bottom-anchored status bar**

In `render_view_output`, replace `let hy0 = (vh as i32 - SCREEN_H as i32) / 2 + wy as i32;` with:

```rust
        let hy0 = vh as i32 - SCREEN_H as i32 + wy as i32;
```

- [ ] **Step 7: Anchored player-family sprites**

At the top of the `if let Some(s) = spr {` block add:

```rust
        // `draw_cells` wraps screen positions in 8 bits like OAM. A native view
        // keeps that wrap; a resized one draws each sprite with the camera on
        // its own anchor and adds the true offset.
        let resized = (vw, vh) != view::NATIVE;
        let (mx, my) = view::margin((vw, vh));
        let place = |x: u16, y: u16| -> ((u16, u16), (i32, i32)) {
            if resized {
                (
                    (x, y),
                    (x as i32 - cam_x as i32 + mx, y as i32 - cam_y as i32 + my),
                )
            } else {
                ((cam_x, cam_y), (0, 0))
            }
        };
```

Then in each player-family draw, draw with the returned camera and add the offset. Player:

```rust
        if let Some((px, py, frame)) = s.player {
            let ((ax, ay), (ox, oy)) = place(px, py);
            let cells = sml2_core::player::draw_cells_styled(
                &s.tables,
                frame,
                px,
                py,
                ax,
                ay,
                sml2_core::player::CellStyle {
                    force_palette_one: s.player_star_flash,
                    behind_background: s.player_behind_bg,
                },
            )
            .ok_or_else(|| format!("player frame {frame} has no mapping"))?;
            let cells = in_obj_priority_order(cells);
            for c in cells {
                blit_obj_cell(
                    &mut img, &bg_idx, &mut taken, &s.objblock, &pal0, &pal1,
                    c.x as i32 - 8 + ox,
                    c.y.wrapping_sub(s.player_pan) as i32 - 16 + oy,
                    c.tile, c.attr, vw, vh,
                );
            }
        }
```

(The player binding was `(mx, my, frame)`; it is renamed to `(px, py, frame)` because `mx`/`my` now name the margin.) Apply the same pattern to:
- shield: `let ((ax, ay), (ox, oy)) = place(sx, sy);` → `draw_cells(&s.tables, sframe, sx, sy, ax, ay, false)`, add `ox`/`oy` (rename its binding `(shx, shy, sframe)` to avoid shadowing `sx`/`sy`).
- streak counter: anchor on the player's position, `draw_streak_counter(&s.tables, timer, chain, px, py, ax, ay)`.
- fireballs and bumpers: `place(fx, fy)` / `place(bx, by)`; their y has no pan.

Boss OAM (no world position) gets only the margin:

```rust
        let boss_shift = if resized { (mx, my) } else { (0, 0) };
        for c in &s.boss_oam {
            blit_obj_cell(
                &mut img, &bg_idx, &mut taken, &s.objblock, &pal0, &pal1,
                c.x as i32 - 8 + boss_shift.0,
                c.y as i32 - 16 + boss_shift.1,
                c.tile, c.attr, vw, vh,
            );
        }
```

Level objects (`s.insts`) already use `inst.x - sx`; leave them.

- [ ] **Step 8: Run all runtime tests**

Run: `cargo test -p sml2-runtime`
Expected: PASS, including the three `widescreen` tests. If an existing test rendered a non-native size and asserted the old mid-view status bar, update its expectation to the bottom row and say so in the commit body.

- [ ] **Step 9: Commit**

```bash
git add crates/sml2-runtime/src/render.rs crates/sml2-runtime/src/render/view.rs crates/sml2-runtime/src/render/dx.rs crates/sml2-runtime/src/tests/widescreen.rs crates/sml2-runtime/src/tests/mod.rs
git commit -m "feat(render): draw enlarged level views without wrap or sprite drift"
```

---

### Task 3: Session view, fixed screens and the overworld

**Files:**
- Modify: `crates/sml2-runtime/src/session.rs` (struct ~808, `new` ~1071, `advance` ~1297)
- Modify: `crates/sml2-runtime/src/render/session.rs`
- Test: `crates/sml2-runtime/src/tests/widescreen.rs`

**Interfaces:**
- Consumes: `view::{NATIVE, enlarged, letterbox, fixed_screen, copy_rect}`, `ObjectSim::view_margin`.
- Produces:
  - `pub fn Session::set_view(&mut self, view: (usize, usize))`
  - `pub(crate) fn Session::overworld_view<P: Copy>(&self, map_sim: &MapSim, view: (usize, usize), fill: P, render: impl FnMut(&MapSim, (usize, usize)) -> Result<Vec<P>, String>) -> Result<Vec<P>, String>`

- [ ] **Step 1: Write failing tests** — append to `tests/widescreen.rs`:

```rust
#[test]
fn set_view_feeds_the_level_spawn_margin_but_not_demos() {
    let mut session = level(1);
    session.set_view((384, 216));
    session.advance(FrameInput::default());
    let SessionState::InLevel(ctx) = &session.state else { panic!("in level") };
    assert_eq!(ctx.obj.view_margin, (112, 36));
    assert!(session.start_demo(0));
    session.advance(FrameInput::default());
    let SessionState::Demo(demo) = &session.state else { panic!("in demo") };
    assert_eq!(demo.ctx.obj.view_margin, (0, 0));
}

#[test]
fn fixed_screens_are_centred_with_bars() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.enter_title();
    let native = session.render_frame(NATIVE);
    let view = (384, 216);
    let wide = session.render_frame(view);
    assert_eq!(crop(&wide, view, (112, 36, 160, 144)), native);
    assert!(crop(&wide, view, (0, 0, 112, 216)).iter().all(|&p| p == 0));
}

#[test]
fn the_hub_view_stays_inside_the_map_and_keeps_the_native_centre() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    let SessionState::Overworld(map) = &mut session.state else { panic!("boots to map") };
    map.load_screen(0, 0, &session.assets.node_records);
    map.cam_x = 0xC0;
    map.cam_y = 0xC0;
    let native = session.render_frame(NATIVE);
    let view = (384, 216);
    let wide = session.render_frame(view);
    // The camera is far from the hub edges, so the view is centred on it.
    assert_eq!(
        crop(&wide, view, (112, 36, 160, 136)),
        crop(&native, NATIVE, (0, 0, 160, 136))
    );
    // At the corner the view clamps instead of wrapping.
    let SessionState::Overworld(map) = &mut session.state else { unreachable!() };
    map.cam_x = 0;
    map.cam_y = 0;
    let native = session.render_frame(NATIVE);
    let wide = session.render_frame(view);
    assert_eq!(crop(&wide, view, (0, 0, 160, 136)), crop(&native, NATIVE, (0, 0, 160, 136)));
}

#[test]
fn a_zone_screen_is_drawn_whole_and_centred() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    let SessionState::Overworld(map) = &mut session.state else { panic!("boots to map") };
    map.load_screen(1, 0, &session.assets.node_records);
    let full = session.render_frame((256, 144));
    let view = (384, 216);
    let wide = session.render_frame(view);
    assert_eq!(crop(&wide, view, (64, 36, 256, 144)), full);
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p sml2-runtime widescreen`
Expected: compile error `no method set_view`.

- [ ] **Step 3: `Session::view` and `set_view` in `session.rs`**

Add to `pub struct Session` after `pub frame_n: u64,`:

```rust
    /// Presentation size; its margin widens the live level's spawn windows.
    view: (usize, usize),
```

Initialise `view: crate::render::view::NATIVE,` in `Session::new`. Add:

```rust
    pub fn set_view(&mut self, view: (usize, usize)) {
        self.view = view;
    }
```

In `advance`, immediately after `self.remember_level_state();`:

```rust
        // Demos replay recorded inputs against native spawns.
        if let SessionState::InLevel(ctx) = &mut self.state {
            let (mx, my) = crate::render::view::margin(self.view);
            ctx.obj.view_margin = (mx.max(0) as u16, my.max(0) as u16);
        }
```

- [ ] **Step 4: Letterbox fixed screens in `render/session.rs`**

Add `use super::view;` at the top. In `LevelContext::render_output`, wrap the ending:

```rust
            return Ok(view::fixed_screen((vw, vh), output.blank(3), |size| {
                sml2_core::ending_render::render_ending_output(
                    &self.ending_scene,
                    &self.ending_sequence,
                    size,
                    output,
                )
            }));
```

In `Session::render_output`, bind `let fill = output.blank(3);` and wrap the easy card, `Title`, `FileSelectLoading` (both arms), `FileSelect`, `Castle` and `GameOver` arms the same way: replace each `(vw, vh)` argument with the closure's `size` and the arm body with `view::fixed_screen(view, fill, |size| …)`. Example for the title:

```rust
            SessionState::Title(_) => view::fixed_screen(view, fill, |size| {
                sml2_core::front_end::render::render_title_output(&self.assets.front_end, size, output)
            }),
```

The LCD-off arm becomes `vec![output.blank(0); vw * vh]` unchanged (it is uniform).

- [ ] **Step 5: Overworld stitching in `render/session.rs`**

Replace the `SessionState::Overworld(map_sim)` arm with:

```rust
            SessionState::Overworld(map_sim) => {
                let map_sim = self.shown_map(map_sim);
                self.overworld_view(map_sim, view, fill, |sim, (w, h)| {
                    let assets = MapAssets {
                        tilemaps: &self.assets.overworld_tilemaps,
                        manifest: &self.assets.overworld_manifest,
                        tiles: &self.assets.overworld_tiles,
                        zone_animation_tiles: &self.assets.zone_animation_tiles,
                        wave: self.map_wave.clone(),
                        sprites: None,
                    };
                    let frame = self.overworld_frame(sim, assets, (w, h));
                    Ok(sml2_core::map::rasterize_overworld(
                        &frame.composition,
                        &frame.vram,
                        frame.bg_palette,
                        frame.obj_palette,
                        frame.obj_palette1,
                        w,
                        h,
                        output,
                    ))
                })
                .unwrap_or_else(|_| vec![fill; vw * vh])
            }
```

Add to the `impl Session` that holds `overworld_frame`:

```rust
    /// The overworld in any view. The hub is stitched from native frames
    /// (its sprites are 8-bit OAM positions relative to each frame's camera);
    /// a zone screen is drawn whole; rooms and the pan mode stay native.
    pub(crate) fn overworld_view<P: Copy>(
        &self,
        map_sim: &sml2_core::map_sim::MapSim,
        view: (usize, usize),
        fill: P,
        mut render: impl FnMut(&sml2_core::map_sim::MapSim, (usize, usize)) -> Result<Vec<P>, String>,
    ) -> Result<Vec<P>, String> {
        use super::view::{enlarged, letterbox, NATIVE};
        if !enlarged(view) {
            return render(map_sim, view);
        }
        match map_sim.screen_id {
            0 if map_sim.scroll_only_latch == 0 => self.stitched_hub(map_sim, view, fill, render),
            1..=8 => {
                let size = (view.0.min(256), 144);
                Ok(letterbox(&render(map_sim, size)?, size, view, fill))
            }
            _ => Ok(letterbox(&render(map_sim, NATIVE)?, NATIVE, view, fill)),
        }
    }
```

Then:

```rust
    fn stitched_hub<P: Copy>(
        &self,
        map_sim: &sml2_core::map_sim::MapSim,
        view: (usize, usize),
        fill: P,
        mut render: impl FnMut(&sml2_core::map_sim::MapSim, (usize, usize)) -> Result<Vec<P>, String>,
    ) -> Result<Vec<P>, String> {
        use super::view::{copy_rect, NATIVE};
        const HUB: i32 = 512;
        // Rows above `$3F203`'s status-bar strip; each native frame's own strip is dropped.
        const ROWS: usize = 136;
        // OAM Y $F0 (and the +8 rows and walk shift, which wrap to 0) all land
        // above or below the frame, so the cursor draws nothing.
        const HIDDEN_CURSOR_Y: u8 = 0xF0;
        let (vw, vh) = view;
        let ox = (i32::from(map_sim.cam_x) + 80 - vw as i32 / 2).clamp(0, HUB - vw as i32);
        let oy = (i32::from(map_sim.cam_y) + 72 - vh as i32 / 2).clamp(0, HUB - vh as i32);
        let mut out = vec![fill; vw * vh];
        for ty in (0..vh).step_by(ROWS) {
            for tx in (0..vw).step_by(NATIVE.0) {
                let mut tile = map_sim.clone();
                tile.cam_x = (ox + tx as i32) as u16;
                tile.cam_y = (oy + ty as i32) as u16;
                let cx = i32::from(map_sim.cursor_x) + i32::from(map_sim.cam_x) - i32::from(tile.cam_x);
                let cy = i32::from(map_sim.cursor_y) + i32::from(map_sim.cam_y) - i32::from(tile.cam_y);
                // From -16 the 8-bit wrap still lands each cell where it truly is.
                if (-16..=176).contains(&cx) && (-16..=168).contains(&cy) {
                    tile.cursor_x = cx as u8;
                    tile.cursor_y = cy as u8;
                } else {
                    tile.cursor_y = HIDDEN_CURSOR_Y;
                }
                let pixels = render(&tile, NATIVE)?;
                copy_rect(&mut out, view, &pixels, NATIVE.0, (0, 0, NATIVE.0, ROWS), (tx as i32, ty as i32));
            }
        }
        let mut bar = map_sim.clone();
        bar.cursor_y = HIDDEN_CURSOR_Y;
        let pixels = render(&bar, NATIVE)?;
        let (x0, y0, x1, y1) = sml2_core::map::status_bar_strip_rect(NATIVE.0, NATIVE.1);
        copy_rect(
            &mut out,
            view,
            &pixels,
            NATIVE.0,
            (x0, y0, x1 - x0, y1 - y0),
            ((vw as i32 - 160) / 2 + x0 as i32, vh as i32 - (y1 - y0) as i32),
        );
        Ok(out)
    }
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p sml2-runtime`
Expected: PASS. The existing `session_palette_producer_reaches_title_file_select_all_maps_rooms_and_castle` test still renders 160×144 and must be unchanged.

- [ ] **Step 7: Commit**

```bash
git add crates/sml2-runtime/src/session.rs crates/sml2-runtime/src/render/session.rs crates/sml2-runtime/src/tests/widescreen.rs
git commit -m "feat(session): enlarged views for the map, menus and spawn margin"
```

---

### Task 4: DX renderer at any view size

**Files:**
- Modify: `crates/sml2-runtime/src/render/dx.rs`
- Test: `crates/sml2-runtime/src/tests/widescreen.rs` (DX tests skip when no DX bundle is installed, like the existing DX tests — check `crates/sml2-desktop/src/app/dx_tests.rs` / `sml2-runtime/tests/dx_preview.rs` for the skip helper and reuse it)

**Interfaces:**
- Changes: `pub fn render_dx_frame(&self, dx: &Presentation, character: Character, view: (usize, usize)) -> Result<Vec<[u8; 3]>, String>`
- Internal: `fn render(ctx, dx, character, view)`, `fn draw_cells(objects, size, banks, cells, pan, offset)`, `fn blit(objects, size, banks, x, y, tile, attr)`.

- [ ] **Step 1: Write the failing test** (uses the same skip helper as `tests/dx_preview.rs`):

```rust
#[test]
fn dx_frames_take_the_view_size_and_keep_native_parity() {
    let Some(dx) = crate::tests::dx_presentation() else { return };
    let session = level(1);
    let native = session.render_dx_frame(&dx, Default::default(), NATIVE).unwrap();
    let view = (384, 216);
    let wide = session.render_dx_frame(&dx, Default::default(), view).unwrap();
    assert_eq!(wide.len(), 384 * 216);
    let centre: Vec<_> = (0..136)
        .flat_map(|y| wide[(36 + y) * 384 + 112..(36 + y) * 384 + 272].to_vec())
        .collect();
    assert_eq!(centre, native[..160 * 136].to_vec());
}
```

If no `dx_presentation()` helper exists, add one to `crates/sml2-runtime/src/tests/mod.rs` by moving the loader used in `tests/dx_preview.rs` (it returns `None` when the DX cache is missing).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p sml2-runtime dx_frames_take`
Expected: compile error (missing `view` argument).

- [ ] **Step 3: Thread `view` through `render_dx_frame`**

```rust
    pub fn render_dx_frame(
        &self,
        dx: &Presentation,
        character: Character,
        view: (usize, usize),
    ) -> Result<Vec<[u8; 3]>, String> {
        use super::view::fixed_screen;
        const BAR: [u8; 3] = [0, 0, 0];
        match &self.state {
            SessionState::Title(_) => {
                return Ok(fixed_screen(view, BAR, |_| screen(&dx.title, &[], None).unwrap_or_default()))
            }
            SessionState::FileSelectLoading(loading) => {
                return if loading.lcd_disabled() {
                    Ok(vec![[255; 3]; view.0 * view.1])
                } else {
                    Ok(fixed_screen(view, BAR, |_| screen(&dx.title, &[], None).unwrap_or_default()))
                };
            }
            SessionState::FileSelect(file_select) => {
                let frame = self.render_dx_file_select(file_select, dx, character)?;
                return Ok(fixed_screen(view, BAR, |_| frame));
            }
            SessionState::Overworld(map_sim) => return self.render_dx_map(map_sim, dx, character, view),
            _ => {}
        }
        let ctx = self
            .dx_level()
            .ok_or("DX presentation requires the title, file select or a level")?;
        render(ctx, dx, character, view)
    }
```

`fixed_screen` calls its closure with `NATIVE` when enlarged; for a non-enlarged non-native view the native screens are returned at 160×144, which the desktop never requests (it only asks for table sizes). Propagate `screen(...)` errors by computing it before the closure instead of `unwrap_or_default()` if clippy objects:

```rust
let title = screen(&dx.title, &[], None)?;
return Ok(fixed_screen(view, BAR, |_| title));
```

Use that form for both title arms.

- [ ] **Step 4: DX map** — split `render_dx_map` into colour setup plus a per-frame pixel pass driven by `overworld_view`:

```rust
    fn render_dx_map(
        &self,
        map_sim: &sml2_core::map_sim::MapSim,
        dx: &Presentation,
        character: Character,
        view: (usize, usize),
    ) -> Result<Vec<[u8; 3]>, String> {
        let map_sim = self.shown_map(map_sim);
        let colors = dx
            .map
            .screens
            .get(usize::from(map_sim.screen_id))
            .ok_or("DX colors only the hub and the zones")?;
        // `$26:$77F1`: the hub's eastern half has its own background set.
        let bg_palettes = match &colors.alternate_bg_palettes {
            Some(east) if map_sim.cam_x >= 0xE7 => east,
            _ => &colors.bg_palettes,
        };
        let player = usize::from(character == Character::Luigi);
        let palettes = CgbPalettes::from_bytes(bg_palettes, &colors.obj_palettes[player])
            .map_err(|e| e.to_string())?;
        let space_path =
            map_sim.level_progress[sml2_core::map::progress_offset::SPACE_PATH_0] & 1 != 0;
        let attributes = match (&colors.alternate_attributes, space_path) {
            (Some(alternate), true) => Some(alternate),
            _ => colors.attributes.as_ref(),
        };
        // The only pixels a hub frame leaves empty are the status bar's
        // `$3F203` strip, which DX draws in color 3 (black in every hub set);
        // a zone's are past its 18 rows.
        let blank = IndexedPixel::new(if map_sim.screen_id == 0 { 3 } else { 0 }, 0).unwrap();
        self.overworld_view(map_sim, view, [0, 0, 0], |sim, (w, h)| {
            let wave = self.map_wave.as_ref().and_then(|live| {
                sml2_core::map::MapWave::follow(dx.map.tiles.screen_raw(0), live)
            });
            let assets = crate::render::session::MapAssets {
                tilemaps: &dx.map.tilemaps,
                manifest: &dx.map.manifest,
                tiles: &dx.map.tiles,
                zone_animation_tiles: &dx.map.zone_animation_tiles,
                wave,
                sprites: Some(&dx.map.sprites),
            };
            let frame = self.overworld_frame(sim, assets, (w, h));
            dx_map_pixels(&frame, (w, h), attributes, &dx.map.hub_attributes, blank, &palettes)
        })
    }
```

and move the old pixel body to a free function, replacing every `160`/`144` with `w`/`h`:

```rust
fn dx_map_pixels(
    frame: &crate::render::session::OverworldFrame,
    (w, h): (usize, usize),
    attributes: Option<&Vec<u8>>,
    hub_attributes: &[u8],
    blank: IndexedPixel,
    palettes: &CgbPalettes,
) -> Result<Vec<[u8; 3]>, String> {
    let mut background = Vec::with_capacity(w * h);
    for sample in &frame.composition.bg {
        let pixel = match sample {
            Some(s) => {
                let attr = match attributes {
                    Some(map) => {
                        let (x, y) = (usize::from(s.cell.0), usize::from(s.cell.1));
                        map.get(y * 32 + x).copied().unwrap_or(0)
                    }
                    None => hub_attributes[usize::from(s.tile_byte)],
                };
                let color = frame.vram.get(s.tile).map_or(0, |t| t[usize::from(s.pixel)]);
                BackgroundPixel {
                    index: IndexedPixel::new(color, attr & 7).map_err(|e| e.to_string())?,
                    priority: attr & 0x80 != 0,
                }
            }
            None => BackgroundPixel { index: blank, priority: false },
        };
        background.push(pixel);
    }
    // `draw_map_objects`: smaller X wins, then the earlier entry.
    let mut objects = vec![None; w * h];
    let mut order: Vec<_> = frame.composition.oam.iter().enumerate().collect();
    order.sort_by_key(|&(i, &(_, x, _, _))| (x, i));
    for (_, &(oy, ox, tile, attr)) in order {
        let Some(data) = frame.vram.get(usize::from(tile)) else { continue };
        for dy in 0..8i32 {
            for dx in 0..8i32 {
                let (x, y) = (i32::from(ox) - 8 + dx, i32::from(oy) - 16 + dy);
                if !(0..w as i32).contains(&x) || !(0..h as i32).contains(&y) {
                    continue;
                }
                let sx = if attr & 0x20 != 0 { 7 - dx } else { dx };
                let sy = if attr & 0x40 != 0 { 7 - dy } else { dy };
                let color = data[(sy * 8 + sx) as usize];
                let target = &mut objects[y as usize * w + x as usize];
                if color == 0 || target.is_some() {
                    continue;
                }
                *target = Some(ObjectPixel {
                    index: IndexedPixel::new(color, attr & 7).map_err(|e| e.to_string())?,
                    behind_background: attr & 0x80 != 0,
                });
            }
        }
    }
    cgb::compose_frame(&background, &objects, true, palettes).map_err(|e| e.to_string())
}
```

Match `attributes`' real element type (`colors.attributes` is whatever `Option<…>` `sml2_dx` declares — use `Option<&T>` with that `T`). `OverworldFrame` and `MapAssets` are already `pub(crate)`.

- [ ] **Step 5: DX level at any size** — in `fn render(ctx, dx, character)` add `view: (usize, usize)` and:
  - `let (vw, vh) = view; let (mx, my) = super::view::margin(view); let resized = view != super::view::NATIVE;`
  - background `vec![…; vw * vh]`; `sx = cam.x - vw as i32 / 2`, `sy = cam.y - vh as i32 / 2 - scroll_pan`; spans `super::view::window_span(vw)` / `(vh)` replacing the `256`s from Task 2; loops `0..vh` / `0..vw`; index `py * vw + px`.
  - HUD: `let py = ctx.hud_y as i32 + vh as i32 - 144 + dy as i32;` skip if outside `0..vh`; `x = mx + col * 8 + dx`, skip if outside `0..vw`.
  - objects `vec![None; vw * vh]`.
  - Player-family sprites: the same `place` closure as Task 2 Step 7 (anchor on the sprite when `resized`), passing the offset to `draw_cells`.
  - Level objects: `blit(&mut objects, view, banks, inst.x as i32 - sx + x as i32, inst.y as i32 - sy + y as i32, …)`.
  - `scene_reload_blank` returns `vec![[255; 3]; vw * vh]`.

New helper signatures:

```rust
fn draw_cells(
    objects: &mut [Option<ObjectPixel>],
    size: (usize, usize),
    banks: [&[u8]; 2],
    cells: Vec<Cell>,
    pan: u8,
    offset: (i32, i32),
) -> Result<(), String> {
    for cell in cells {
        blit(
            objects,
            size,
            banks,
            cell.x as i32 - 8 + offset.0,
            cell.y.wrapping_sub(pan) as i32 - 16 + offset.1,
            cell.tile,
            cell.attr,
        )?;
    }
    Ok(())
}

fn blit(
    objects: &mut [Option<ObjectPixel>],
    (w, h): (usize, usize),
    banks: [&[u8]; 2],
    x: i32,
    y: i32,
    tile: u8,
    attr: u8,
) -> Result<(), String> {
    if x <= -8 || x >= w as i32 || y <= -8 || y >= h as i32 {
        return Ok(());
    }
    let attr = CgbAttributes::from_byte(attr);
    for dy in 0..8 {
        for dx in 0..8 {
            let (px, py) = (x + dx, y + dy);
            if !(0..w as i32).contains(&px) || !(0..h as i32).contains(&py) {
                continue;
            }
            let target = &mut objects[py as usize * w + px as usize];
            if target.is_some() {
                continue;
            }
            let index = cgb::sample_tile(banks, tile as usize * 16, attr, dx as u8, dy as u8)
                .map_err(|e| e.to_string())?;
            if index.color() != 0 {
                *target = Some(ObjectPixel { index, behind_background: attr.priority() });
            }
        }
    }
    Ok(())
}
```

Update `screen()` to call `draw_cells(&mut objects, (160, 144), banks, cells.to_vec(), 0, (0, 0))` and the existing unit test to `blit(&mut objects, (160, 144), [&bank0, &bank1], 0, 0, 0, 0x8c)` (twice).

- [ ] **Step 6: Fix callers** — `rg -n "render_dx_frame\(" crates` and pass `(160, 144)` everywhere except the desktop (Task 5 passes the real view).

- [ ] **Step 7: Run tests**

Run: `cargo test -p sml2-runtime && cargo test -p sml2-desktop`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/sml2-runtime/src/render/dx.rs crates/sml2-runtime/src/tests/widescreen.rs crates/sml2-runtime/src/tests/mod.rs crates/sml2-desktop/src
git commit -m "feat(dx): render DX frames at any view size"
```

---

### Task 5: Desktop settings and presentation

**Files:**
- Create: `crates/sml2-desktop/src/settings/view.rs`
- Modify: `crates/sml2-desktop/src/settings.rs`, `crates/sml2-desktop/src/timing.rs`, `crates/sml2-desktop/src/app/{mod,panels,screenshot}.rs`
- Test: `settings/view.rs` (unit), `timing.rs` (unit), `app/tests.rs`

**Interfaces:**
- Produces: `pub enum Widescreen { Off, Wide16x9 }`, `pub enum CameraZoom { Native, Zoom125, Zoom150 }` (both `label()`, `next()`), `Settings::view(&self) -> (usize, usize)`, `game_rectangle(available, zoom, view)`.

- [ ] **Step 1: `settings/view.rs` with tests**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Widescreen {
    #[default]
    Off,
    Wide16x9,
}

impl Widescreen {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Wide16x9 => "16:9",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Wide16x9,
            Self::Wide16x9 => Self::Off,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CameraZoom {
    #[default]
    Native,
    Zoom125,
    Zoom150,
}

impl CameraZoom {
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "100%",
            Self::Zoom125 => "125%",
            Self::Zoom150 => "150%",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Native => Self::Zoom125,
            Self::Zoom125 => Self::Zoom150,
            Self::Zoom150 => Self::Native,
        }
    }
}

/// The game view in game pixels.
pub fn view_size(widescreen: Widescreen, zoom: CameraZoom) -> (usize, usize) {
    let height = match zoom {
        CameraZoom::Native => 144,
        CameraZoom::Zoom125 => 180,
        CameraZoom::Zoom150 => 216,
    };
    let width = match widescreen {
        Widescreen::Off => height * 10 / 9,
        Widescreen::Wide16x9 => height * 16 / 9,
    };
    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn view_sizes_match_the_design_table() {
        use CameraZoom::*;
        use Widescreen::*;
        assert_eq!(view_size(Off, Native), (160, 144));
        assert_eq!(view_size(Off, Zoom125), (200, 180));
        assert_eq!(view_size(Off, Zoom150), (240, 216));
        assert_eq!(view_size(Wide16x9, Native), (256, 144));
        assert_eq!(view_size(Wide16x9, Zoom125), (320, 180));
        assert_eq!(view_size(Wide16x9, Zoom150), (384, 216));
    }
}
```

In `settings.rs`: `mod view; pub use view::{CameraZoom, Widescreen};`, add fields after `character`:

```rust
    /// Widen the game view (Mods). Changes object spawning while enlarged.
    pub widescreen: Widescreen,
    pub camera_zoom: CameraZoom,
```

defaults `Widescreen::Off`, `CameraZoom::Native`, and:

```rust
impl Settings {
    pub fn view(&self) -> (usize, usize) {
        view::view_size(self.widescreen, self.camera_zoom)
    }
}
```

Extend `version_two_settings_restore_optional_dx_selection`'s removed-field list with `"widescreen", "camera_zoom"` so old files are proven to load.

- [ ] **Step 2: Run** `cargo test -p sml2-desktop settings` — Expected: PASS.

- [ ] **Step 3: `game_rectangle` takes the view** — failing test in `timing.rs` tests:

```rust
    #[test]
    fn game_rectangle_scales_the_view_by_whole_steps() {
        let view = (384, 216);
        assert_eq!(
            game_rectangle([1920, 1080], crate::settings::Zoom::Auto, view),
            ([0, 0], [1920, 1080])
        );
        assert_eq!(
            game_rectangle([800, 720], crate::settings::Zoom::Auto, (160, 144)),
            ([80, 0], [640, 576])
        );
    }
```

Implementation:

```rust
pub fn game_rectangle(
    available: [u32; 2],
    zoom: crate::settings::Zoom,
    view: (usize, usize),
) -> ([u32; 2], [u32; 2]) {
    let (w, h) = (view.0 as u32, view.1 as u32);
    let fit = (available[0] / w).min(available[1] / h);
    if fit == 0 {
        return ([0, 0], [0, 0]);
    }
    let scale = match zoom {
        crate::settings::Zoom::Auto => fit,
        crate::settings::Zoom::Integer(n) => u32::from(n).min(fit).max(1),
    };
    let size = [w * scale, h * scale];
    (
        [(available[0] - size[0]) / 2, (available[1] - size[1]) / 2],
        size,
    )
}
```

Update existing callers/tests of `game_rectangle` to pass `(160, 144)` or `self.settings.view()`.

- [ ] **Step 4: Presentation in `app/mod.rs`**
  - Before the tick loop, inside `if let Some(game) = &mut self.game {`: `game.session.set_view(self.settings.view());`
  - `frame_image`: `let view = self.settings.view();` → `render_dx_frame(dx, self.settings.character, view)`, `ColorImage::new([view.0, view.1], …)`, `render_palette_frame(view)`, `colorized(&pixels, self.settings.filter, &header, view)`.
  - `colorized(frame, filter, header, view: (usize, usize))` ends with `ColorImage::new([view.0, view.1], pixels)`.
  - `game_view`: `game_rectangle([...], self.settings.zoom, self.settings.view())`; `self.effective_scale = size[0] / self.settings.view().0 as u32;`
  - `zoom()`: use `let (w, h) = self.settings.view();` and `w as f32 * f32::from(scale)` / `h as f32 * …`.

- [ ] **Step 5: Screenshot verification uses the frame's own size** — in `screenshot.rs`:

```rust
    let [fw, fh] = frame.size;
    let scale = w / fw.max(1);
    if frame.pixels.len() != fw * fh
        || scale == 0
        || w != scale * fw
        || h != scale * fh
        || x + w > image.size[0]
        || y + h > image.size[1]
    {
        return Err("Game rectangle is not an integer-scaled game view".into());
    }
```

and `frame.pixels[(py / scale) * fw + px / scale]`.

- [ ] **Step 6: Mods rows** — in `app/panels.rs`, change `Page::Mods => self.dx_controls(ui),` to:

```rust
                Page::Mods => {
                    self.view_controls(ui);
                    self.dx_controls(ui);
                }
```

and add to the same `impl DesktopApp` block:

```rust
    /// Settings → Mods: the enlarged view.
    fn view_controls(&mut self, ui: &mut egui::Ui) {
        theme::section(ui, "VIEW");
        let adjust = self.menu.row_adjustment();
        let widescreen = self.settings.widescreen;
        if self.menu.row(ui, "Widescreen", Value::Choice(widescreen.label().into())) || adjust != 0 {
            self.settings.widescreen = widescreen.next();
        }
        let adjust = self.menu.row_adjustment();
        let zoom = self.settings.camera_zoom;
        if self.menu.row(ui, "Camera zoom", Value::Choice(zoom.label().into())) || adjust != 0 {
            self.settings.camera_zoom = zoom.next();
        }
        theme::note(
            ui,
            "A larger view also spawns enemies and items earlier, so play differs from the \
             original while it is on. Menus and cutscenes keep their original size.",
        );
    }
```

(Import `Value` / `theme` the way `dx.rs` does.) Replace the caption `"… The game view stays 160 × 144."` at `panels.rs:229` with `"+ / − zoom · F11 fullscreen · F4 cycles filters."`.

- [ ] **Step 7: App test** — add to `app/tests.rs` (`app()` at the top of that file builds an app with a loaded game):

```rust
#[test]
fn the_frame_follows_the_view_settings() {
    let (mut app, _, _) = app();
    app.settings.widescreen = crate::settings::Widescreen::Wide16x9;
    app.settings.camera_zoom = crate::settings::CameraZoom::Zoom150;
    let image = app.frame_image().expect("a game frame");
    assert_eq!(image.size, [384, 216]);
}
```

Also add a red-bundle variant in `app/dx_tests.rs` next to `dx_mode_colors_every_level_of_the_normal_game`:

```rust
#[test]
fn dx_mode_frames_follow_the_view_settings() {
    let (mut app, _, _) = super::tests::app();
    app.dx_bundle = Some(red_bundle());
    app.game.as_mut().unwrap().session.enter_title();
    app.settings.dx_mode = true;
    app.settings.widescreen = crate::settings::Widescreen::Wide16x9;
    let image = app.frame_image().unwrap();
    assert_eq!(image.size, [256, 144]);
    // The title is native art: red in the centre, bars at the sides.
    assert_eq!(image.pixels[72 * 256 + 128], Color32::from_rgb(255, 0, 0));
    assert_eq!(image.pixels[72 * 256], Color32::from_rgb(0, 0, 0));
}
```

- [ ] **Step 8: Run** `cargo test -p sml2-desktop` and `cargo clippy -p sml2-desktop -p sml2-runtime -p sml2-core -- -D warnings` — Expected: PASS / no warnings.

- [ ] **Step 9: Commit**

```bash
git add crates/sml2-desktop/src
git commit -m "feat(desktop): widescreen and camera zoom settings"
```

---

### Task 6: Docs and the parity gate

**Files:**
- Modify: `DESIGN.md` (standing goal 3 only, lines ~119-143)
- Modify: `docs/plans/2026-10-22-widescreen-zoom-design.md` (spec deviation, quirks)
- Modify: `docs/desktop.md` (settings list, if it enumerates Mods)

- [ ] **Step 1: Update standing goal 3** — replace the HUD sentences of the first bullet with:

```markdown
  default AND the parity anchor. At 160×144 the HUD strip is in the native
  rect; in any larger view it is bottom-centred (the desktop's Widescreen and
  Camera zoom mods). With the status bar excluded, the centre 160×144 slice of
  a larger level frame is pixel-identical to the native frame — re-assert
  `tests/widescreen.rs` after any renderer change.
```

- [ ] **Step 2: Spec** — in the design doc, under "2. Rendering → Boss OAM", replace the clipping sentence with: "Boss cells keep the core pass's native clipping (simulation output) and are shifted by the view margin." Add every handler found to check fixed screen edges to "Known quirks" (search: `rg -n "screen\.x >=|screen\.x <|OFFSCREEN" crates/sml2-core/src/objects`).

- [ ] **Step 3: Full gate**

```bash
cargo test -p sml2-core && cargo test -p sml2-runtime && cargo test -p sml2-desktop
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

Expected: all tests pass; sweep output matches the pre-change baseline (run the sweep once on `cb8d6f5` first if no baseline is recorded).

- [ ] **Step 4: Commit**

```bash
git add DESIGN.md docs/plans/2026-10-22-widescreen-zoom-design.md docs/desktop.md
git commit -m "docs: record enlarged-view behaviour and known quirks"
```
