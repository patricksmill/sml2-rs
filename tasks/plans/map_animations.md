# Plan — Overworld Map Animations

Status: phase 0 and 1 done; 2–5 open. Replaces the open questions in
`tasks/subsystems/map_decor_and_animation.md` with an order of work.

Every phase ends with the same gate:

```
cargo test -q -p sml2-core -p sml2-runtime      # 0 warnings
bash captures/sweep.sh                          # byte-identical except traj4_ff0
```

plus, for anything that draws, a PyBoy comparison of the screen in question
(OAM dump and/or frame diff) recorded in the commit message.

## Phase 0 — Cursor metasprites (done, `952a0cc`)

`cursor_sprite` + the `$4AD7`/`$4B77`/`$4B87`/`$4B8F`/`$4BAF`/`$4BCF` tables.
Covers the walk cycle, the hidden case and the `$8x` walk-step arrows.

## Phase 1 — Camera pan arrows (done, `34659f9`)

`$3C9F4` stages `$4A11`/`$4A21` into OAM 0-3 while `sMapScrollOnlyLatch != 0`,
so the four border arrows blink and the cursor disappears. This is the
"blinking arrows when moving the camera" case.

The blink is `ldh a,[$FF97] / swap a / and $01`, i.e. **bit 4** of the frame
counter — a 16-frame half-period, measured on the console (B+Right on the hub:
frames 18-29 blank, 30-45 visible, 46-47 blank). Reading bit 7 instead gives a
2.1 s half-period, which is the wrong rhythm.

## Phase 2 — Hub wave strips (next)

Goal: the hub's shoreline shimmer.

1. Extract the four strip initialisers. `sMapWaveStrip0..3` live at
   `$A700`/`$A720`/`$A740`/`$A760` (32 bytes each); the hub loader seeds them
   from a bank-15 table before `$3E853` starts rotating. Dump the seed and the
   VRAM targets `$88F0`/`$89F0`/`$8CF0`/`$8DF0`, add them to
   `extracted/map/` as `waves.bin`.
2. Add `wave_strip: [[u8; 32]; 4]` and `raster_phase: u8` to `MapSim`, and port
   `$3E853`: `e = (phase & 7) * 2`, apply the 16-bit rotate
   `(strip[e], strip[e+$10])` and write both bytes back. Note the ROM only runs
   the rotate when `phase & 1 == 0`, i.e. every other frame.
3. Apply the strip bytes to the decoded VRAM tiles before rendering (they are
   the last 16 bytes of the four tiles starting at the addresses above, so the
   renderer needs the tile offset, not a new tile).
4. Gate: PyBoy, hub, 64 frames — compare the four VRAM tile pairs each frame.

## Phase 3 — Decor slot expander (all screens)

Goal: the static decor (flags, castles, clouds, bushes) that the BG does not
draw, plus its culling.

1. Extract: the 18-slot list per screen (hub = the 72 bytes at `$4BFD`; zone
   screens build theirs in code, so each loader needs reading), the `$4C4D`
   metasprite table, and the metasprite cells themselves. Emit
   `extracted/map/decor.bin` plus a JSON index.
2. Port `$3CFC8`/`$3D044`/`$3D001`: slot `(A, X, Y, ID)`, `ID == $FF` empty,
   graphics bank `6 + (A >> 4)`, camera-relative cull, draw from OAM index 13
   (behind the cursor).
3. Gate: PyBoy OAM entries 13-27 per screen at three camera positions.

## Phase 4 — Per-screen animation scripts

In dependency order:

1. **Pumpkin** `$3D8E6`: `sMapAnimPhase` → sprite 0 Y (`$5A21`), sprite 1 X
   (`$5A29`), and the `$6227` step table for sprite 1's id/Y every other frame.
2. **Castle sparkles** `$3CEC4`: `sMapSparkleTimer` (`$A7A9`), id 10 vs `$FF`,
   positions from `$4FA8`/`$4FB8`, re-arm on `rDIV & $72 == 0`.
3. **Screen-5 mover** `$3DDD3`: `sMapMovingX`/`sMapMovingDir`.
4. **Turtle** `$3E7F2` (the other `sMapRasterPhase` user).

Each one is a `MapSim` state machine plus the sprite ids it writes; the decor
expander from phase 3 must be in place first.

## Phase 5 — Cleanup

- Fold `render_overworld_view`'s argument list (19 now) into a small
  `MapRenderInput` struct; the pan/cursor params pushed it over the useful
  limit.
- Add the missing OAM-order test: decor (13+) must lose to the cursor (0-3).

## Risks

- **Banked graphics**: decor slot `A` names a graphics bank; if two slots on one
  screen use different banks, the port needs more than the single 384-tile
  block `OverworldTiles` holds. Check this while extracting phase 3, before
  porting the expander.
- **Camera cull bias**: `$3D044` compares against `cam - $30` in 16-bit
  bank-space, which is not the naive `slot - cam`. Port it literally and let the
  PyBoy diff catch the off-by-one.
- **Strip writes**: phase 2 writes raw VRAM bytes mid-tile; if the port only
  models whole tiles, the shimmer will look chunky. Verify against the console
  before widening the tile model.
