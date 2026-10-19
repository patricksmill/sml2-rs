# Overworld Map — Decor Sprites & Ambient Animation

- **ROM addresses** (bank 15 = CPU `$4000-$7FFF`, file `0x3C000+`):
  - Cursor OAM chain: `$3C901` (dispatch), `$3C9B5` (OAM staging), tables `$4AD7` (16 pose
    pointers), `$4B77` (scroll arrows), `$4B87` (hidden), `$4B8F`/`$4BAF` (edge arrows),
    `$4BCF`/`$4BD7`/`$4BDF` (stump), `$498C` (stump order `00 08 00 10`)
  - Decor list copy (hub): `$3CBE7` reads 72 bytes at `$4BFD` into `sMapSprite0Page`
  - Decor expanders: `$3CFC8` (walk the 18 slots), `$3D044` (parse + camera cull),
    `$3D001` (draw into OAM from `sMapOAMIndex` = 13)
  - Decor metasprite table: `$4C4D`, 18 words; cells are `(dy, dx, tile, attr)` runs
    terminated by a `$FF` dy
  - Pumpkin animation: `$3D8E6` (`sMapAnimPhase` `$A699`, tables `$5A21`/`$5A29`, step
    table `$6227`, `sMapAnimIndexA` `$A6A2`)
  - Castle sparkles: `$3CEC4` (`sMapSparkleTimer` `$A7A9`, position tables `$4FA8`/`$4FB8`,
    re-armed when `rDIV & $72 == 0`)
  - Screen-5 mover: `sMapMovingX` `$A6A7`, `sMapMovingDir` `$A6A8` (`$3DDD3`)
  - Hub wave strips: `$3E853`, `sMapWaveStrip0..3` = `$A700`/`$A720`/`$A740`/`$A760`
- **Subsystem:** Overworld map OBJ layer
- **Module:** `crates/sml2-core/src/map.rs` (`render_overworld_view`), `map_sim.rs`

## 1. What is implemented

`render_overworld_view` now draws the cursor with the ROM's own chain (`cursor_sprite`,
committed with the pipe-priority fix). It honours:

| `sMapCursorMove` | sprite |
| --- | --- |
| high nibble `== 8` (bit 7) | `$4B77` camera-scroll arrows, 2 frames on `walk_frame` bit 3 |
| nibble bit 2 (`$40`) | `$4B87`, all `$7F` = hidden |
| nibble `$0A`, or bit 1 clear | `$4AD7` pose table entry |
| walking + move bit 1 / bit 0 | `$4B8F` / `$4BAF` edge arrows, 4 frames on `walk_frame & $0C` |
| otherwise | `$4BCF` stump cycling `$498C` |

Cells are staged as the four OAM entries the ROM writes: `(0,0) (+8,0) (0,+8) (+8,+8)`
from `(sMapCursorX, sMapCursorY)`, shifted `(+4,+8)` while a walk stream is live with
bit 5 set.

## 2. Decor slots (`sMapSprite0Page`, 18 × 4 bytes)

Each slot is `(A, X, Y, ID)`:

- `A` high nibble selects the graphics bank as `6 + (A >> 4)`; low nibble feeds the same
  bank number for the camera compare (`$3D044` does `(A & $0F) | $06`).
- `ID == $FF` means the slot is empty. Otherwise `ID` indexes `$4C4D` for the metasprite.
- Metasprites live in bank 15 right after the table and are `(dy, dx, tile, attr)` cells
  ending in a `$FF` dy. Tiles are ordinary indices into the same `$8000` block the map's
  384-tile VRAM set uses, so `OverworldTiles::screen_vram_tiles` already has them.
- The expander culls camera-relative: `$3D044` computes `screen = slot - cam` in 16-bit
  bank-space after an `-$30` bias, sets `sMapSpriteOffscreen`, and `$3D001` writes from
  OAM index 13 upward — i.e. **decor draws behind the cursor** (entries 0-3).

### Measured hub state (PyBoy, 2026-10-19)

- The hub list `$4BFD` has only four live slots:
  `(X,Y,ID)` = `(4C,4F,1)`, `(A8,F0,9)`, `(C0,28,9)`, `(88,70,5)`; the other 14 are `$FF`.
- Metasprites: `ID 1` = 16 cells (4×4), `ID 5` = 9 cells (3×3), `ID 9` = 8 cells (2×4).
- On the live hub screen the OAM dump shows **only cursor entries 0-3** (pose 15 = tiles
  `$0A,$0A,$0C,$0B` at y 76..84, x 75..83) — every decor entry 13+ is zero. So the hub's
  decorative sprites are culled at its default camera; the visible hub art is BG.

## 3. Ambient animation sources (not ported)

| screen | mechanism | state |
| --- | --- | --- |
| hub | 4 wave strips, `$3E853`: every other phase, 16-bit bit-rotate of `(strip[e], strip[e+$10])` and write to VRAM `$88F0`/`$89F0`/`$8CF0`/`$8DF0` | not modelled |
| Pumpkin | `$3D8E6`: `sMapAnimPhase` drives sprite 0 Y (`$5A21`), sprite 1 X (`$5A29`), and the `$6227` step table moves sprite 1's tile id and Y every other frame | not modelled |
| castle | `$3CEC4`: sparkle sprites 16/17, tile id 10 vs `$FF`, positions from `$4FA8`/`$4FB8`, re-armed on `rDIV & $72 == 0`, 4-step timer | not modelled |
| screen 5 | `sMapMovingX`/`sMapMovingDir` mover | not modelled |

`MapSim` currently carries none of the phase/timer bytes (`sMapRasterPhase`,
`sMapAnimPhase`, `sMapSparkleTimer`, `sMapMovingX`), and the renderer only reads the static
tile block, so all four need new state plus per-screen strip/decor data.

## 4. Exit criteria for the remaining work

1. Extract per-screen decor lists and wave-strip initial contents into `extracted/map/`
   (the hub is the only block copy at `$4BFD`; zone screens build their lists in code, so
   each loader needs reading).
2. Add the phase/timer bytes to `MapSim` and advance them exactly as the ROM does.
3. Render decor from all 18 slots with the `$3D044` cull, behind the cursor.
4. Gate with a PyBoy frame diff: for each screen, capture OAM entries 13+ and the VRAM
   bytes at the strip addresses for ~64 frames and compare against the port.
