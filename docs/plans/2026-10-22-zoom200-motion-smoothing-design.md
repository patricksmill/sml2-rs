# 200% camera zoom and motion smoothing — design

Extends the widescreen/zoom mod (`2026-10-22-widescreen-zoom-design.md`).

## 200% camera zoom

- `CameraZoom::Zoom200`: view height 288. Views: 320×288 (GB aspect) and
  512×288 (16:9).
- Both fit the hub stitching limit (`view <= 512`), so the overworld hub
  widens; fixed screens pillarbox as at the other zooms.
- The zoom cycle becomes 100% → 125% → 150% → 200% → 100%.

## Motion smoothing

Goal: smoother scrolling and sprite motion on displays faster than 60 Hz by
drawing in-level frames at positions between the last two simulation ticks,
at sub-game-pixel precision. The simulation is untouched; the capture sweep
stays byte-identical.

### Scope

- In-level gameplay and demos (`SessionState::InLevel`, `SessionState::Demo`),
  native and DX renderers.
- Not covered (always shown at the current tick): overworld, all fixed
  screens, the boss OAM path.

### Setting

- Mods → VIEW: "Motion smoothing: Off / On", persisted; default Off.

### Snapshot

- Before each tick of a level context, the session stores the positions it is
  about to replace: camera `(x, y)`, Mario, moon shield, bumpers, fireballs,
  and each object keyed by its slot index.
- `SpriteInst` gains `slot: u8` (`#[serde(default)]`) so objects match across
  ticks.

### Blend factor

- `TickBudget::fraction()` returns the accumulated phase as `alpha ∈ [0, 1)`.
- A redraw shows `prev + (cur - prev) * alpha` for every tracked position.
  This adds up to one tick (~17 ms) of display latency while enabled.

### Rendering

- The frame is the current tick's frame; cells, flips, palettes, priorities
  and 8-bit wrapping are unchanged. Each layer gets a sub-pixel screen offset
  in units of `1/s` game pixels:
  - Background: `cam_cur - cam_lerp`.
  - Each sprite: `(pos_lerp - pos_cur) - (cam_lerp - cam_cur)`.
  - Status bar (window layer): none.
- Each renderer produces, at game resolution, the background with a 1-pixel
  border (plus its colour-index buffer for priority) and a list of positioned
  sprite cells tagged with the offset group they belong to.
- A shared compositor, `crates/sml2-runtime/src/render/motion.rs`, upsamples
  the background by `s` with its offset, blits cells as `s×s` blocks at their
  offsets, and resolves BG-over-OBJ priority per output pixel.
- `s` is the largest divisor of the window's effective integer scale such that
  `(vw·s)·(vh·s) <= 1024·1024`. The GPU still upsamples by an integer factor.
- There is one render path: with smoothing off it runs at `s = 1` with zero
  offsets and no border, and must stay byte-identical to the pre-change
  output (existing renderer tests and the capture sweep check this).
- The background border is 8 px, the largest shift the snap rule allows.

### Snap rules (no offset for that position)

- Camera or a given sprite moved more than 8 px on either axis in one tick.
  A camera snap drops every offset; a sprite snap draws that sprite at its
  current world position against the in-between camera.
- An object slot absent from the previous snapshot is treated like a sprite
  snap.
- Paused (game pause or debug pause), single-stepping, scene-reload blank,
  boss-mode switch, or the state was not a level context on both ticks.

### Desktop

- With smoothing on, the app requests a repaint every frame (vsync-paced)
  instead of every 8 ms.
- The texture size becomes `(vw·s, vh·s)`; the destination rectangle is
  unchanged.

### Testing

- `alpha` from `TickBudget`, `s` selection, snap rules.
- Compositor at `alpha = 1` is byte-identical to the existing frame (native
  and DX).
- A half-pixel background offset at `s = 2` shifts the output by one pixel.
- `cargo test` for all crates; capture sweep unchanged.
