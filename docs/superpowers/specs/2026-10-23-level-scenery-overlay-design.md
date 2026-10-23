# Level scenery overlay — design

Status: awaiting review. Next step: implementation plan after approval.

## Goal

An enlarged level view (widescreen or camera zoom) shows a repeated strip of
the level at the screen edges, and a blank band above or below the floor.
The original game hides both outside the 160×144 picture. This replaces that
margin with authored scenery, and continues the original picture's edge where
nothing is authored.

The centred 160×144 picture stays the original frame. The overlay is drawing
only. It does not change collision, enemies, the camera, or a 160×144 game.

## Decisions

- An enlarged view samples the real layout cell wherever that cell exists,
  with no window wrap and no copied edge. Anywhere Mario can walk stays the
  level.
- Past the 256×48 layout, a paint supplies the cell. An unpainted cell past
  the level is blank. A 160×144 view keeps the original window path.
- Paint coordinates are layout cells and may be negative or past the map.
  A paint inside the layout is stored but not drawn.
- The same rule is used by the native renderer and DX. DX colors a painted
  metatile through the level's existing tile-attribute table.
- No editor screen in this work. The data sits on `LevelContent`, which
  `SessionAssets::insert_level` already replaces for playtest. The level
  editor planned in `docs/plans/2026-10-19-desktop-features-and-extensibility.md`
  later paints this field and shows the two rects.

## Data

```rust
pub struct SceneryOverlay {
    pub paints: Vec<SceneryPaint>,
}

pub struct SceneryPaint {
    pub x: u8,
    pub y: u8,
    pub metatile: u8,
}
```

`x` is a layout column (`0..256`), `y` a layout row (`0..48`), `metatile` an
index into that level's metatile table. `LevelContent` gains
`scenery: SceneryOverlay`. Loading from extracted files sets it empty.
`SceneryOverlay::empty()` and `SceneryOverlay::try_from_paints` are the
constructors. `try_from_paints` rejects, naming the cell:

- `x >= 256` or `y >= 48`
- `metatile` absent from the level's metatile table
- a second paint on the same `(x, y)`

JSON, for the future project file:

```json
{ "paints": [ { "x": 1, "y": 20, "metatile": 12 } ] }
```

## Sampling

One function, used by both renderers:

`margin_sample(view, px, py, cam, scroll_pan, overlay) -> MarginSample`

World pixel of view pixel `(px, py)`:

```
sx = cam_x - view_w / 2
sy = cam_y - view_h / 2 - scroll_pan
world = (sx + px, sy + py)
```

The native rect is centred: its top-left in the view is
`((view_w - 160) / 2, (view_h - 144) / 2)`. The playfield is that rect with
the bottom 8 rows removed (the status bar).

- The view is 160×144, or `(px, py)` lies in the native rect: `Native`. The
  current `window_coord` path draws it, including pipe backgrounds and
  layout rewrites.
- `(px, py)` is outside that rect and its world cell is inside the 256×48
  layout and that cell has a paint: `Paint { metatile }`. A world cell left
  of column 0, above row 0, or past the layout has no paint. The pixel uses
  that metatile's quadrant, the same way a layout cell does. Pipe sampling
  and layout rewrites do not apply.
- Otherwise: `Edge { nx, ny }`, the native-playfield pixel nearest to
  `(px, py)`. Past the left or right, copy that column. Past the top or
  bottom, copy playfield row 0 or row 135. A corner copies the corner.

`Edge` is resolved by reading the pixel the `Native` path produced at
`(nx, ny)`, so the copy matches the live frame, including animation.
Boss arenas, the overworld, and the fixed screens are unchanged.

## Errors

A bad overlay never reaches the renderer. `try_from_paints` is the only
constructor besides `empty`. A level loaded from content has an empty
overlay. Drawing an empty overlay cannot fail.

## Tests

- A 160×144 level frame is unchanged when the level carries paints.
- At a level's starting camera, a 384×216 frame's centre 160×144 equals the
  native frame, and its left column equals the native playfield's left
  column.
- The background rows below the native rect equal native playfield row 135.
  The status bar stays on the bottom of the enlarged view and is not part of
  that comparison.
- A paint on the cell just left of the native world rect appears in the
  margin. A paint inside the native rect does not.
- `try_from_paints` rejects an out-of-range cell, an unknown metatile, and
  a duplicate cell.
- DX follows the same centre and margin result for one painted cell.

## Out of scope

The editor screen, painting tools, undo, and project-file IO. Overworld
maps. Changing the simulated layout, camera locks, or spawn windows.
ROM export.
