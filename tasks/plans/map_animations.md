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

## Phase 3 — Decor slot expander (done)

Goal: the static decor (flags, castles, clouds, bushes) that the BG does not
draw, plus its culling.

**State:** `DECOR_METASPRITES` (17 metasprites, `(dy, dx, tile, attr)` cells
terminated by a `$FF` *tile*), `HUB_DECOR_SLOTS` (the 72 bytes at `$4BFD`) and
`decor_oam()` (the `$3D044` window plus `$3D001`'s cell math) are in
`crates/sml2-core/src/map.rs`. The position math is console-verified: at
`cam = ($BB, $0142)` slot 14 stages OAM `y=62 x=213 tile=$3A`, which matches
PyBoy exactly.

**Wired in `698c94f`** once phase 4 supplied the live slot values. The earlier
"no decor on the hub" reading was a measurement artefact - the staged cells sit
at OAM y=246/254, above the top edge, and my dump filtered `y >= 160`.

1. Extract: the 18-slot list per screen (hub = the 72 bytes at `$4BFD`; zone
   screens build theirs in code, so each loader needs reading), the `$4C4D`
   metasprite table, and the metasprite cells themselves. Emit
   `extracted/map/decor.bin` plus a JSON index.
2. Port `$3CFC8`/`$3D044`/`$3D001`: slot `(A, X, Y, ID)`, `ID == $FF` empty,
   graphics bank `6 + (A >> 4)`, camera-relative cull, draw from OAM index 13
   (behind the cursor).
3. Gate: PyBoy OAM entries 13-27 per screen at three camera positions.

## Phase 4 — Per-screen animation scripts (hub done, `698c94f`)

Hub: coins (slots 4-9, ids 2/3/4 every 8 frames), clouds (slots 1-3, one pixel
every 4 frames, direction on `sMapCloudDriftCounter` bit 6) and sparkles (slots
16/17, id `$0A`, re-armed from `rDIV`) are in `MapSim::hub_decor_tick`; the
coin ids the loader seeds from the six golden-coin flags (`$3CE43`) are still
the `$FF` the list starts with, so coins stay invisible until that lands.

Remaining, in dependency order:

1. **Pumpkin** `$3D8E6`: `sMapAnimPhase` → sprite 0 Y (`$5A21`), sprite 1 X
   (`$5A29`), and the `$6227` step table for sprite 1's id/Y every other frame.
2. **Castle sparkles** `$3CEC4`: `sMapSparkleTimer` (`$A7A9`), id 10 vs `$FF`,
   positions from `$4FA8`/`$4FB8`, re-arm on `rDIV & $72 == 0`.
3. **Screen-5 mover** `$3DDD3`: `sMapMovingX`/`sMapMovingDir`.
4. **Turtle** `$3E7F2` (the other `sMapRasterPhase` user).

Each one is a `MapSim` state machine plus the sprite ids it writes; the decor
expander from phase 3 must be in place first.

## Phase 4b — Zone-screen decor (Pumpkin done)

Zone screens do **not** use the hub's expander. The Pumpkin handler ends with
`call $3E277`, which:

- clears `sStatusBarOAM` (144 bytes), not the hub's 108-byte area;
- starts at **OAM entry 4** (`sMapDecorOAMIndex = 4`), so zone decor loses to the
  cursor (0-3) and the HUD (4-12 is rebuilt by this same clear);
- walks **16 slots of 3 bytes** from `sMapSprite0Page` - `(x, y, id)` in OAM
  space, `$FF` = empty - and calls `$3F75` per slot.

`$3F75` is a bank-switch thunk: `ld a,$18 / ld [$A24E],a / ld [$2100],a /
ld hl,$62E6 / call $509B / ld a,$0F / ... / ret`, i.e. bank 24's `$509B`. That
routine takes `id * 2` into the word table at **bank 24 `$62E6`** and expands
`(dy, dx, tile, attr)` cells, OAM = slot + (dy, dx), same terminator rule as the
hub (a `$FF` tile).

**Verified so far.** With the Pumpkin screen loaded, PyBoy's OAM 4+ matches this
expansion exactly, one frame behind the slot sample (OAM is uploaded from the
shadow): slot 0 `(0, 76, 16)` -> `(76,0,$30,0) (76,8,$30,$20)`; slot 1
`(1, 15, 21)` -> `(15,9,$36,0) (23,4,$32,0) (23,12,$33,0)`.

**Resolved:** the `$62E6` table holds **188 entries** (`$62E6-$645D`; entry 0
points at `$645E`, where the metasprite data starts), so id 152 is valid and the
measured slot 12 id 200 is past it - `$509B` would follow metasprite bytes as if
they were a pointer. The port skips ids >= 188 instead. The table and all 188
metasprites are now in `crates/sml2-core/src/map_zone_decor.rs`, and
`zone_decor_oam` reproduces the console's OAM 4+ for the measured Pumpkin slots
(slot 0 and slot 1 exactly, slot 2 against the previous frame's coords, which is
what the shadow upload shows).

**Where the zone slots actually come from.** `$3CBE7` copies 72 bytes from
`$4BFD` into `sMapSprite0Page` and nothing clears them on a screen change, so a
zone screen reads the **first 48 of those bytes as sixteen 3-byte slots** - the
hub's list with its bytes regrouped, then patched by the loader and by the
handler's slot 0-2 writes. Measured: the block's zone view is `(0,76,79)
(1,16,120) (54,255,16) (168,240,9)`, and a live zone screen reports `(0,76,*)
(1,16,*) (54,255,*)` with the ids rewritten. So the port derives `zone_slots`
from `HUB_DECOR_SLOTS` rather than carrying a measured snapshot - an earlier cut
seeded a measured table that was really hub RAM, which put the Pumpkin screen's
decor at the hub's positions.

**Done for Pumpkin** (`caa22ed` + `3881259` + this commit): `ZONE_DECOR_METASPRITES` and
`zone_decor_oam` live in `map_zone_decor.rs`, `MapSim` carries the sixteen
3-byte slots, and `pumpkin_decor_tick` ports `$3D8E6`. The field names are one
slot off from the 3-byte layout, which is what made the first cut drift:
`sMapSprite0Y` is slot 0's **id**, `sMapSprite1Page` slot 1's **y**,
`sMapSprite1X` slot 1's **id**, `sMapSprite1Y` slot 2's **x** (`(v >> 4) & 7`,
sign bit 7), `sMapSprite1Id` slot 2's **y** (`v & 7`, sign bit 3), and
`sMapSprite2Page` slot 2's **id**. With that mapping the model reproduces all
sixteen slots over twenty frames on three separate trials.

**Also needed:** the other zone screens' slot seeds and handlers (`$3DA2D` for
Tree, and the rest), plus the Pumpkin growth/patch tail. Zone loaders build them in code (the
hub is the only 72-byte block copy), so each loader needs reading; the Pumpkin
handler `$3D8E6` then drives slots 0/1/2 from `sMapAnimPhase` (`$5A21` Y,
`$5A29` X, `$6227` id/Y drift, page `22 + $5A16`) and arms the patch/growth
sequence.

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
