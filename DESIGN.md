# sml2-rs design notes

Rust port of Super Mario Land 2, built on the byte-identical disassembly in the
sibling `sml2decomp` repo (`SML2_DECOMP_ROOT`, default `../sml2decomp`). This
file records the architecture and the goals the code must keep room for; read
it before adding engine/renderer code.

## Layout

- `crates/sml2-core` — data model: level/header/metatile/layout structs,
  collision classes (`collision.rs`), object tables + behavior profiles
  (`object.rs`), shared display helpers (`video.rs`). Pure data, no I/O.
- `crates/sml2-extract` — build-time pipeline: parses the disassembly sources +
  `baserom.gb`, emits `extracted/`:
  - `levels/levelNN.manifest.json` + `index.json` (headers, metatiles, layouts,
    collision classes, spawn records with `sprite_id`/`hitbox`)
  - `levels/*.decompressed.bin` (raw `$B000` layouts)
  - `levels/levelNN.bgblock.bin` (composed `$8800-$97FF` BG tile block)
  - `objects.json` (anim→sprite maps, extent records, OAM sprite mappings,
    per-anim behavior profiles)
  - `renders/levelNN{,.metatiles,.sprites}.png` (reference art)
  - `gfx/**` (tile sheets)
- `crates/sml2-runtime` — the port. v0: full-frame renderer over `extracted/`
  only (no ROM parsing):
  `sml2-runtime --level N [--camera X Y] [--size WxH] [--out frame.png]
  [--scale 4] [--anim-phase 0..3] [--hud-digits 0500000395]
  [--sprites sprites.json] [--mario X Y FRAME]`
  or `--window` (resizable: integer scale + letterbox; arrows scroll,
  shift = fast, Q/ESC quits; animated tiles advance on the engine's cadence).
  `--play` runs the PORTED simulation in that window: arrows move Mario
  (Z/Space jump, X/Shift run), the ported camera follows, frames come from
  the decoded `$FFC6` selection (`player_sim::frame_index`). `--replay
  file.json` re-plays a captured live trajectory through the sim + camera and
  reports per-field divergences; `--filmstrip DIR` also writes every 30th
  replayed frame as a PNG (sim state rendered through the normal pipeline).
  Renders a `--size` viewport (default 160×144, world-clamped, centred on the
  camera) from the level layout + metatile table + BG block, applies
  the level's `$9200` tile animation (bank $1D rows from `tileanim.bin`),
  draws the HUD status bar (window layer at WY=136/WX=7 from `hud.json`),
  draws the PLAYER first (OAM entries 0.., highest priority): `--mario X Y
  FRAME` expands the bank-1 player frame table (`sml2_core::player`; base =
  `(marioX-camX+96, marioY-camY+98)` raw OAM, cells = base + offsets, attr
  as stored, no forced priority), and
  draws objects: `--sprites` takes `[{sprite, x, y, flip}]` (world position,
  sprite id, `sSpriteFlipFlags`), expands the OAM mappings like `$C000`
  (offset lands in the RAW OAM byte: screen = `obj + offset - (8,16)`;
  OAM attr bit5 = X flip, bit6 = Y flip, bit4 = OBP select), and applies the
  GB priority order (smaller X first, BG-over-OBJ bit).
  The `screen_top_left = (camera - 80, camera - 72)` relation is the START
  position only: mid-level, the live origin = `sScrollX/Y` and the follow
  camera LEADS Mario when running — port the engine's camera target logic,
  not a Mario-centred follow.
  Verified 2026-10-11: BG+HUD+anim+PLAYER parity — four static live poses
  (stand right/left, duck right/left) render at 100.000% exact shade-rank
  (0 residue px), with the player's OAM shadow matching byte-for-byte;
  OBJ positions/flips/tile-addressing verified against the live OAM (drawn
  bbox == OAM cells; Mario's `$4E` resolves to `$84E0`, i.e. the `$8000`
  tile mode). Caveat: the engine can draw either of two adjacent stand
  variants on a given frame (hvel micro-state; 32 vs the walk-timer variant
  34), so a frame check picks the matching one.
  Note: entering a level through the hub map can place Mario at a doorway
  different from the header spawn (level 3 via node 33 starts at ~2572,132,
  not the header's 49,468) — map nodes carry their own entry points; the
  header camera still shows the level's designed start.

Regenerate everything with `cargo run -p sml2-extract` from this repo root
(reads `$SML2_DECOMP_ROOT` / `../sml2decomp`, writes `extracted/`).

## Standing goals (design constraints, not wishlist)

Four user-facing features must stay cheap to add at any point. All are
satisfied by keeping the seams below intact — do not bake these away:

### 1. Texture packs (replaceable art)

- **All art resolves through logical ids, never through baked pixels.**
  Tiles are 2bpp data + 8-bit ids; metatiles are 4 tile ids; sprites are
  `[y, x, tile, attr]` mapping entries; collision/behavior is a function of
  ids only. The extractor's PNGs are *reference renders*, not the runtime
  format.
- **Palettes are data, resolved at draw time** (BGP/OBP0/OBP1 per level header,
  OBJ palette select from `sSpriteFlipFlags` bit 4). Never bake shades into
  stored tiles — a pack must be able to override tiles, palettes, or both
  independently.
- **Asset resolution seam:** the renderer loads art through a single resolver,
  `AssetId -> Texture`, with sources ordered `rom-default -> pack(dir)`.
  A texture pack is just a directory that mirrors the extracted structure
  (e.g. `pack/tilesets/treezone1.png`, RGBA, same tile grid layout as
  `extracted/gfx/`, plus optional `palettes.json`). No rebuild, no code
  changes to add one.
- Keep `extracted/` reproducible from the ROM so packs can always diff
  against the reference.

### 2. Frame interpolation for high-refresh-rate displays

- **The simulation stays a fixed 60 Hz tick** (`tick()` = one GB frame; game
  logic, collisions, spawn cursors, timers are tick-quantised).
  The render loop runs free at refresh rate (60/90/120/144…), each frame
  drawing at `alpha = (now - lastTick) / tickDuration`.
- **Interpolatable state keeps a previous snapshot.** For camera, Mario, and
  every active object slot, store `prev` + `cur` positions (and any other
  tweened scalar) and render `lerp(prev, cur, alpha)`. The engine's own
  subpixel accumulators (`XSub`, velocities) give exact in-tick positions
  when needed.
- **Never let rendering feed back into logic**: draw code may read state and
  alphas only. Collisions, spawn/despawn, and tilemap writes happen on ticks.
- Non-interpolated things (tile animations, sprite frame swaps, HUD) step at
  60 Hz — that's correct and expected.
- Practical note: OAM shadow state is already staged per tick
  (`$A100`/`$FF8D`); the interpolating renderer builds its own display list
  from slot snapshots rather than drawing the staged OAM directly.

### 3. Variable screen sizes & the display pipeline

- **The viewport is a parameter, not a constant.** `--size WxH` renders any
  world window centred on the camera, clamped so the viewport stays inside
  the level (16x16 up to the full 4096x768 world). The native 160x144 is the
  default AND the parity anchor: the HUD strip stays in the native rect
  (centred within a larger viewport), so the centre 160x144 slice of *any*
  larger frame is pixel-identical to the native frame — that invariant is the
  display regression test, re-assert it after any renderer change. A future
  "modern UI" mode may instead pin the HUD to the viewport edge; keep the two
  choices switchable, not baked.
- **Window mode: integer scale + letterbox.** The window is resizable; each
  frame picks `s = min(win_w/W, win_h/H) >= 1` and letterboxes. Never blend or
  non-integer-stretch by default (a filtered mode can be a user option later).
- **Keep the two size knobs distinct, and add the third:** `--size` = viewport
  (how much world is visible, in logical pixels); `--scale` = output pixels per
  logical pixel (crisp upscale). Texture packs need an **internal resolution**
  N: art sampled at fractional logical coordinates (`x_int / N`), so a 2x pack
  renders at 320x288 internal without tile ids changing. Plan: fixed-point
  sampling `(tile_id, u_frac, v_frac)` through the single asset resolver —
  widescreen and supersampling then share one path. Keep ids/palettes as data
  (already true); the per-pixel loops in `render_view` are the only place that
  changes.
- The display pipeline stays independent of the 60 Hz simulation (goal 2):
  resizing or scaling never feeds back into game state.

### 4. SML2 DX compatibility (clean-room BYOR)

- **Never redistribute patch code or modified assets.** Compatibility follows a
  Bring Your Own ROM (BYOR) model: the user supplies their own patched `sml2dx.gbc`.
- **Launcher identification:** The launcher/CLI identifies a DX ROM via Game Boy
  header flags (CGB flag `$80`/`$C0` at `$0143`, expanded size `≥1MB` at `$0148`)
  and exact version SHA-1/CRC32 matches (v1.5, v1.8, etc.).
- **Dynamic extraction seam:** An ingestion module parses the known table offsets
  in Toruzz's expanded banks locally, extracting GBC BGR555 palettes, VRAM
  bank 1 attribute maps, and Luigi sprite/physics tables into a local cache
  conforming to the core asset schema.
- **Runtime integration:** Palettes and attributes hook into the texture pack /
  palette resolver (Goal 1), while Luigi mechanics plug into `player_sim` via an
  alternate character physics profile (`--character luigi`).

## Current milestone state

Done: byte-identical disassembly (gate: `make clean && make compare` +
sha1 `bba4085…`), full asset extraction (headers, metatiles, layouts,
collision classes, spawn streams, object taxonomy incl. OAM mappings and
per-anim behavior profiles, PNG reference renders), all live-verified against
PyBoy where possible. The runtime exists as a full-frame renderer (BG + tile
animation + HUD window + OBJ pass), verified against live frames at 98-99.3%
shade-rank with residue confined to moving-actor capture skew and animation
phase timing.

Variable-size display shipped 2026-10-11: `--size WxH` (16x16 up to the full
4096x768 world, clamped), resizable letterboxed window. Verified: default
160x144 output byte-identical to the pre-change renderer; centre-slice
equality vs native at 320x144 / 160x224 / 320x224 / 640x288; edge-clamp view
(BG rows match native, HUD stays in the native rect); live parity unchanged
(98.034% on the reference frame).

Player (Mario) draw shipped 2026-10-11: decoded from bank-1 `main.asm`
`$5267`-`$5520` (frame lattice `$5377`/`$53CF`, powerup offsets `$5447`-
`$549F`, geometry `$54A7`, expander `$5297`), extracted as
`objects.json player_mappings` (242 frames, schema `sml2-objects/3`),
implemented in `sml2_core::player` + runtime `--mario X Y FRAME`. Verified
against live: OAM shadow byte-for-byte on four poses; full frames at
100.000% exact shade-rank (stand right/left, duck right/left).

Player physics shipped 2026-10-11: `sml2_core::player_sim` — a faithful port
of the bank-0 in-level update (`home.asm $0F71`): accel +8 / friction −4,
`hvel` $80-centred speed index, signed subpixel integration through the
`MarioSpeedTableH/V` family (extracted byte-identical to the ROM), floor/
ceiling/wall probes at the engine's exact offsets, jump (75 / 48 downgrade /
72 swim), jump-cut to 33, gravity with the $30→24 apex snap, landing align,
screen-edge wrap, walk-anim timer. Verified: two captured live trajectories
(882 frames of walk/run/jump/jump-cut/skid/duck/air-control/wall contact)
replayed through `--replay` match the console frame-for-frame on x, y, hvel,
vvel, on-ground, timer, facing — zero divergence, with a detected negative
control.

Camera + play mode shipped 2026-10-11: `sml2_core::camera` (`home.asm
$0B2A`): the player's speed becomes the scroll delta when the draw-set
enable for that side is on (Mario past screen X `$60`), with the ±1
lead-correction parking him at screen X `$3F/$40` (right) / `$70/$71`
(left), the `$B0`/`$50` column locks + pull-backs from the per-sector
`sScreenEdgeMap` (extracted from the ROM for all 32 levels as
`levels/*.edge.bin` — as of 2026-10-13; see the correction below), and the
vertical follow with the `$50`/`$28($40)`
thresholds and `$C0`/`$48` row locks. The `$FFC6` frame selection is ported
as `player_sim::frame_index`. Both verified: the same 882-frame replay now
also matches camx, camy and both scroll enables exactly (12 fields, zero
divergence) — including the frame where the camera unlocks at screen X `$60`
and the `$50`-column pull-back. `--play` wires sim + camera + renderer into
the interactive window; `--filmstrip` renders replayed sim states to PNGs.

**Correction (2026-10-13): the `levels/*.edge.bin` claim above was false when
written.** This file asserted the 48-byte table was "extracted from the ROM for
all 32 levels" while `sml2-extract` never wrote it, and the replay harness hid
the gap by *injecting* the console's sampled `$FF98`
(`cam.edge_now = Some(f.edge)`) rather than deriving it — so the gate exercised
the edge branches with the console's own bytes, and `--play`, the only mode that
had to derive them, ran with `edge_map = [0; 48]`, i.e. every edge bit clear.
The visible effect is a camera that creeps **down**: `$0C7D`'s down-follow uses
`sy >= $50` when the cell's bit3 (hard down edge) is clear and `sy >= $58` when
it is set, so each descent scrolled 8 px early and the ratchet walked the view
below the level floor — "Mario centred in the frame, seeing under the map". The
`$B0`/`$50` column locks were dead in play mode for the same reason.

**The parallel `sSceneMap` is extracted too, as of 2026-10-14.** `ram.asm:716`
calls `sSceneMap`/`sScreenEdgeMap` "two parallel 48-byte 3×16 maps of the level
cells", and the `$03F1` loader copies both — the edge map from the header
pointer `[$4000]`, the scene map from `[$4002]`, each at
`+ (sLevelLayoutIndex & $0F) * 48`. `sml2-extract` writes them through one code
path (`levelNN.edge.bin`, `levelNN.scene.bin`) so they cannot drift, and both
were re-derived from `baserom.gb` for all 32 levels with zero mismatches; 14
levels have a nonzero scene map. `sSceneMap` is the input to `$0F0B`, which
latches `sSceneId` — the cell under Mario, index `(Yhi & $0F) * 16 + (Xhi & $0F)`
— and `sSceneId` indexes the bank-5 scene script table at `$4125`. The port now
fills `CameraSim::scene_map` in both loops and seeds `sSceneState` from the
level header's +18 high nibble (`$0609`); measured, every level in this ROM has
`0` there, so no level starts inside a scene. Level 1's scene map is all zeros,
which is why no level-1 capture can reach any of this.

Fixed the same day: the extractor writes `levelNN.edge.bin` (bank = header +13
`sLevelDataBank`; base = the little-endian pointer at `[bank:$4000]`; source =
`base + (layout_index & $0F) * 48`, 48 bytes — `$03F1`, home.asm 539-571); the
replay path loads it, *derives* `$FF98` and checks the captured byte (`edge`),
and play mode loads the same file, printing a `note:` if it is missing so
neither mode can run edge-less silently. The lookup index is the ROM's
`(Yhi & $0F) * 16 + (Xhi & $0F)` (the `swap a`/`or` at home.asm 1522-1532) —
the port had `& $03` on the Y nibble, which only differs for a row above 3.
Verified: the extracted record reproduces the captured `$FF98` on **49,288 of
49,288** gate-capture frames, with a one-frame lag — the console samples `$FF98`
after its camera update, so on a crossing frame it still holds the *pre-move*
cell's value, which is why the check reads the port's camera before `cam.step`.
The gate is all-zero on all 16 captures including the new `edge` field, and
`camera::tests::the_down_follow_waits_for_the_hard_edge_threshold` pins the
8 px branch difference directly.

Object spawner shipped 2026-10-11: `sml2_core::objects` — the bank-2 `$8000`
object engine's spawn half: camera windows (act ±`$60`, scan ±`$70`, alive
±`$A0`, min-clamped), the three-way scan direction/edge (`$8064`; camera
unchanged stores its low byte as "dir" with a `$FF` spawn hi — both quirks
needed for parity), the 16-slot table (`$AD00`, `$20` stride, state byte
+8) with the $FFF0 odd/even phase split, the standby→active promotion
(`$81F7`), and the stream scan (`$8C31`/`$8CEB`): 8-aligned edge match,
mode-byte gate (easy/normal) with the post-spawn `2` marker, first-free-slot
alloc. Verified: 381-frame capture (run to just short of the level's first
pipe; spawns record 0 at x=472 + activates it) replays with all 19 fields
zero — slot states/defs/sprites/record pointers, standby/active counts,
stream pointer, scan edge/dir, phase parity. (Per-def handlers and pipes
were still unimplemented at that point — see the next entry.)

Object handlers + Mario interaction shipped 2026-10-11: the full bank-16
`$58000` dispatch (interaction flags `$85AA`, slot-parity gate vs
`sObjectUpdatePhase`, `sClearStageFlags`, hit-reaction `$58101`) and the
first two handlers — the walker (anim 9 → `$58201`: `$5B3C6` face-Mario
init, `$5B51A` sprite toggle every 4th `$FFF0`, states 1/2 walk left/right
with the `$84EA`/`$843C` wall probes, 3/4 fall with the `$7488` curve) and
the squashed variant (anim 10 → `$582A9`: flash/phase-32/type-0 init,
countdown, removal via `$5B2D7` + the `$5B31D` coin credit). The Mario
interaction pass (home.asm `$2D7D`/`$2EDD`/`$2DC6`) is ported too: the box
test on the slot's `+9/+10` screen pos vs `mario − cam + (96, 98)`, the
`+23` flags, the stomp bounce (`sObjectBounce` → vvel 48/75, the `$1DFE`
ceiling undo, the direct `$FFC0` Y nudges), the harmful `sMarioDead = 15`,
and the star paths. Slots are now raw 32-byte records; the replay compares
every byte of all 16 slots plus `sMoveFlags`/`sObjectFacing`/`sMarioDead`/
`sMoneyBagCount`/`$FFF0`/`sObjectUpdatePhase`. Verified: 468-frame capture
(walker spawns f207, activates f218, walks 472→443, Mario stomps it f337 →
anim 10 → squashed countdown → removed f401 + coin credited) replays with
zero divergence on all 28 checks — including Mario's post-stomp bounce
trajectory. The 725-frame capture now replays clean to frame 291, where
Mario dies to the walker's side contact: the old "pipe entry" reading was
wrong — `vvel = $5F` is `$31D1`'s death value (sMarioDead `$FF` + walk
timer 0), so that capture's boundary is the unimplemented death sequence,
not a pipe. Captures must record `$FFF0` (`ff0`) + `sObjectUpdatePhase`
(`uphase`) or the sprite-toggle phase can't be reproduced.

More handlers + the pipe entry/travel shipped 2026-10-11: anim 1
(`$582D1`, the fall-then-walk enemy: `$5B585` sprite bob cycling
`[FE,01,FF,02]` through `+26` on even `$FFF0`, type `$56`, `$582DD` fall
until the ground probe lands it, `$582EE` face-Mario init, then states 1/2
walk — even-`$FFF0` move + `$84EA`/`$843C` wall turn, odd-`$FFF0` `$58339`
ledge reverse), anim 2 (`$5834F`, the stunned variant: type `$06`, `+28` =
192 countdown falling, then `$58380` spins `sprite ^= $0B` every 4th
`$FFF0` until `+28` = 8, then back to anim 1 via `$3AA0`), and anims 17/21
(`$58712`, the hopper: state 0 poses as an item — type `$80`, `+12` = `$80`,
done; state 1 rises 8 px; states 2-5 `jp` straight into the walker's states
1-4, so the walker's state bodies are now gate-free functions shared by both
handlers). Pipes: the `$134F` landing arms `sPipeTravelDirection = 1` on
`$08/$09` (0 otherwise, every grounded frame), the `$1366` Y-snap aligns to
`sPipeAlignTarget`; `$1189` enters on DOWN + `(x & $10)` + `(x & $0F) - 5 <
7` (a 7-px window, `$11C0` for UP/`$20`, `$11FE`/`$1244` for the `$0C-$0F`
side classes) → dir `$10` + hvel `$80`; the frame loop's `$0F71`/`$0FBB`
branch replaces the WHOLE player update with `$10EC` (y ±2/frame while the
`$1DFE` ceiling or `$1E36` ungated floor probe still hits, else dir 0), and
`$05E4` skips the `$2D7D` interaction while travelling.

The screen transition shipped 2026-10-11 (later): the four `$0B2A` triggers
(`$B0`+screenX `$A1` → 1 right; `$50`+screenX < `$0F` → 2 left, warping Mario
to x=0/hi+1; `$48`+screenY < `$1B` → 4 up, warping Mario to Y=0 with the
camera's hi; `$C0`+screenY `$90` → 8 down) each latch `sSceneId` via `$0F0B`
from the 48-byte `sSceneMap`, and while `sScreenTransition != 0` the camera
update *is* `$0DED` — a ±4 px/frame scroll along the axis with Mario moved
∓1 (`+1/+2` dithered on odd `$FF97` frames for the down arm) until the target
low byte (`$50`/`$B0`/`$B8`/`$48`), then `$0EB0` clears it (the scene-load
branch needs `sSceneState & $0F != 0` — not ported; level 1's scene map is
all zeros so its pipes skip it) and the frame loop skips the player update
throughout, which is what pauses the pipe travel. Verified: the 2348-frame
capture replays zero on all 29 checks (`tr` included) — the trigger frame,
34 transition frames, the resumed travel ending at dir 0, the fall and
landing at y=612 in the underground, and the post-transition camera
catch-up from the stale `sPrevMarioY`.

The frame selection
(`main.asm $530D`) short-circuits while travelling: `$FFBB` = 0 and the base
frame = 16 (the spin frame) for `$10`/`$20`, the walk cycles `cell+1`/`cell+5`
for `$30`/`$40`, then the usual powerup offset. Verified by two captures,
both zero divergence: the 838-frame one (walker stomp, then record 1's anim-1
enemy spawns f394, activates f406, lands, paces its 2-tile platform 575↔608
with both ledge reverses at f444/f576/f708, sprite bob 4/5/6; record 2's
anim-17 hopper spawns f459/activates f470 and poses as an item — 28 checks)
and the 2126-frame one (both walkers, the block staircase climbed with four
drift-controlled hops, the raised floor, the walker falling off its edge
exercising the walker's fall states, the `$08/$09` arming on the pipe
landing, the 7-px X-window entry at x=1141 → dir `$10`, and 38 travel frames
descending 420→542 with the spin frame — 29 checks incl. `pipe`). The
capture is trimmed at frame 2127, the exact frame the descent trips the
vertical scene transition (`sScreenTransition = 8`), which is not modelled.

Mario's death shipped 2026-10-11 (later): `$319F` (the update's tail) turns a
15/`$80` hit into `$31D1` (dead `$FF`, walk timer 0, vvel 95) or, with a
powerup, a tier loss + 128 i-frames; `$315C` (the frame loop's `$0640`
branch — the camera freezes, only the object engine still runs) applies
`MarioSpeedTableDead[vvel]` with an **8-bit add — no carry into `$FFC1`** (the
low byte wraps; verified live: y 510 → 257), floors vvel at 1, and enters the
fade (`$3186`: `$FF9B` 9, fade timer 192) when the draw's screen Y
(`y - camY + 98`) `& $F0 == $B0`; the dead branch draws the fixed frame 107
(`$5550`) and the frame selection (`$2D0A`) still runs *before* the
interaction, so the contact frame shows the grounded pose. Verified: the
537-frame `death1.json` capture replays zero on all 29 checks — the walk, the
contact, the dispatch, the whole arc including the y wrap, and the fade.

The full death cycle shipped 2026-10-12: the fade's last frame (`$311A`)
decrements `sLives` (BCD) and the game runs `$14` (lives/save/map setup) →
the overworld map (`$0C`, waits for input — unmodelled) → re-entry (`$0B` →
`$02` → `$03`) reloads the level. The reload: `$0F2A` puts Mario back at the
header spawn (hvel $80, vvel 24, facing right; death/powerup cleared;
`on_ground`/`in_air`/phase untouched), the camera resets to the header start
(prev-Y latch = respawn Y), and `$EBBB`'s tail (`$EC40` + `$EC17` + `$EC31`)
clears the object scratch ($AF00-$AFFF: spawn X, update phase, prev-camera
latch, stream, slots) — but **`$FFF0` persists** (HRAM) and `$FFC6` keeps the
dead frame 107 until the first post-respawn frame selection. The fade-trigger
frame still runs the object engine (`$0640` continues into `$0652`) — the
frame-loop gate is `dead_fade` *at entry*, not after the step. Verified: the
838-frame `death2.json` replays zero on all 30 checks — the arc + fade (the
192-frame model checked against the captured `$FF9B` duration; the map's 136
of the 329-tick gap skipped) and the respawn: the walker re-spawns from the
reloaded stream (standby f807 → active f823) with every slot byte matching.

The game-over continuation of that tail landed 2026-10-13. `$311A` sets
`$FF9B = $14` and *then* does `sub 1 / daa` on `sLives`, branching to `$314B`
(`ld a, 5 / ldh [$FF9B], a`) when the subtraction carries. That carry is the
`sub 1`'s borrow out of bit 8 — `daa`'s low-nibble correction (`0x06`, taken
whenever H is set) never touches C — so **the branch fires on `sLives == 0`
exactly**, and a ten like `$10` decrements to `$09` with the game continuing.
(The `$05` write is what survives the `$14` that preceded it.) State `$05`
(`$27AF`) is the GAME OVER screen: the `$4C92` graphics and `$6492` tilemap
loads, `sBGPalette = $E4`, `sSavedLCDC = $C3`, **`sLives` back to 5**, the
coin/money counters cleared, the `sLevelTally` penalty (`sub c / daa`, c = the
set high bits of the six `sGoldenCoin1-6` bytes at `$A84D-$A865` — map-side save
bytes with no in-level writer, so 0 here), `sFadeTimerLow = 255`,
`sMusicRequest = 7`, then `$FF9B = 7`. State `$07` (`$287E`) calls the bank-12
`$30884` (which draws the `$2874` "GAME OVER" string), polls `sFadeTimer` to 0,
then waits for START (`hKeysPressed == $08`) and hands back to `$FF9B = 0`
(`Init`) with `sMusicRequest = $FF`. Modelled as `PlayerSim::game_over` +
`game_over_enter`/`game_over_step`; the art, the bank-12 `$2934` save and the
string draw are not (see the omissions list above). No capture reaches zero
lives — every gate capture survives — so the whole path is unit-pinned
(`death_fade_*`, `game_over_*`), and the replay harness's death-cycle gap takes
the `$05` branch instead of the level reload when the flag is set.

`traj10` closed 2026-10-12 (5130 records, x 472 → 3094, 30 checks zero).
Three divergences, each pinned against the ROM rather than guessed: the
`$129F` running ledge-launch (the "no floor" arm is not a plain fall — with
`sMarioOnGround != 0` and `(hvel - 72) >= $78` in *byte* arithmetic, i.e. hvel
`$C0-$FF` or `$00-$47`, `vvel` becomes 28); the `$2DC6` stomp bounce's `$1DFE`
ceiling probe reading the **shifted** Y (`$2E23` does `$FFC0 -= y_delta + 6`
before the call, so a hit adds `b` back — probing the un-shifted Y carried a
500-frame tail); and the `$FF97 & 7` dither cadence (the engine `inc`s `$FF97`
once per frame in the VBlank wait, so the `$181C` index always reads "previous
residue + 1" — the capture samples at the frame boundary and slips an extra
increment on 3 of 5130 frames, so the replay keeps the record and substitutes
the cadence value only where the record skips a step; `--play` ticks the phase
per frame like the engine). Full suite after: 9 captures × up to 30 checks,
all zero (`traj4_ff0` keeps its documented legacy `$FFF0` divergence).

**Capture pitfall — lag frames**: during the dead branch the game overruns
the VBlank budget and runs 2 logic frames in one VBlank every ~8 frames, so
tick-based sampling (traj4's method) merges them and cannot be replayed 1:1
(symptom: y deltas double the table's with vvel advancing one step; a per-tick
`$315C` hook shows counts alternating 0/2). `death_capture1.py` hooks `$05E4`
and samples per logic frame; `apply_pairing.py` then re-pairs the records —
the hook fires at the frame's START, so record k = end-of-frame-(k-1) state +
frame k's inputs, and the sim's frame i needs the record-i state with
record-(i-1) keys and phase (the edge stays with its own record: it is the
camera's own lookup at that frame's start). The capture also records `pcx`
(the `$AF24/$AF25` latch — 0 at the level load, so the first frame scans
forward) and seeds the phase bool from the frame-0 `uphase` (0 at the load).

Neither `anim 8` (crawler) nor `anim 12` (bobber), `anim 20` (item pose) or
`anim 23` (pose 87) is a no-op any more — earlier revisions of this note said
so, measured otherwise 2026-10-12: the dispatch's unported cases are the
`$58040`/`$59C07` targets with no ported body (mode-A anims
0/3/4/7/13/14/26/30-34/40-48/51-58/61-69/77-90/92-95, and their mode-B
counterparts), plus `$58100`, which is a bare `ret` — a real no-op handler to
dispatch explicitly rather than flag. Level 1's 23 spawn records need none of
them: they resolve to walkers, fallwalkers, hoppers, crawler, bobber, the item
poses, poses 86/87, `marker85` and the anim-16 pickup (`$586F5`, ported
2026-10-13 — see the item-pickup bullet below).

Open: the scene system proper (levels
whose `sSceneMap` is nonzero load a scene at the transition finish), and the
carry/item type-`$06` paths. Display goals above ride on top unchanged.

On the scene system, the ROM's chain — because an earlier revision of this note
got it wrong twice. **240 is written to `sSceneState` (`= $F0`), not `$FF9B`**:
the `$FF9B` jump table ends at `$23` (home.asm:330), so 240 could never index
it. And `$3C94` is a **bank-0 trampoline to bank 2** `$8D9C`, not "bank-1".
The real sequence: a hard-edge trigger in `$0B2A` sets `sScreenTransition` and
calls `$0F0B` to latch `sSceneId` from `sSceneMap`; when the scroll completes,
`$0EB0` (home.asm:2009-2050) tests `sSceneState & $0F` and, if nonzero, sets
`sSceneState = $F0`, then calls `$3C94` → bank 2 `$8D9C` — the boss/template
object set, with `$8E6C` as the template loader (which writes complete 18-byte
slot images into `$AD00`, bypassing the spawn stream `objects.rs` models).
What is missing is therefore not a "state 240" but the bank-5 **script VM**
(`$20A4`, one byte per frame, opcode table at home.asm:4311-4429), the `$0EB0`
`sSceneState & $0F` branch, and the `$8D9C` path. `sSceneMap` is extracted the
same way as the edge map but from the pointer at `[bank:$4002]` (home.asm:572-604);
**14 of 32 levels are nonzero, and level 1 is all zeros**, which is why nothing
in the captures touches it.

Next in line, in measured order: the item ids with no capture
yet (1-4/6/8/9 — the `$3451` writes are ported but unverified; the anim-16
post-touch arm that consumes them is ported and unit-tested as of 2026-10-13,
still capture-less), the **bounce-block cases no capture reaches**: the
single-slot `$00`/`$03` bonk (whose `66` draw is the one `traj15`'s two-slot
`$01` bump cannot show, because the type-4 coin slot draws after it — level 1
has `$00` cells at x ≥ 576 behind the first walker and `$03` cells only at the
tower), the big-Mario types 1/5 (a powerup), the fireball's `$06` break
(`$3415` — a type-1 bump from the projectile), the `$1413` push-out with no
capture yet (see below), the scene system, and the
per-anim handlers with no
ported body (list above). Bounce blocks themselves are no longer in this list —
the model landed 2026-10-13 (wp7g) and `traj15` verifies it end to end. Nor is
the coin touch: `$1D75`/`$1DA5` landed the same day and `traj16` verifies it
(the coin-touch bullet below).

**The "ledge-side wall push-out" was not a physics divergence** — it was the
tick-sampled capture, and it is closed (2026-10-12, wp7f). `traj13_full`'s tail
diverges from f4652 with a constant x +2 because two of its records are
*mid-frame* snapshots rather than logic frames: the record-16 stream spawn at
x=3016 makes that frame overrun its VBlank slot, so the tick boundary lands
inside it (`probe_frame_cadence.py`: tick 4651 is sampled just before the
spawn's `$8C8D`, tick 4652 enters `$05E4` not at all). The console's x cadence
there is the ordinary +1/+2 alternation of hvel `$F3`, there is no wall at 2880
(cols 180+ rows 26/27 are `$60`, non-solid — Mario lands *on* the row-28 ledge
whose side is unprobeable), and the death 13 frames later is the record-14
fallwalker's harmful contact at ~x2920 (an object contact, so `$2D7D` writes
after `$319F` and it dispatches one frame later). Dropping those two records
replays the whole tail all-zero; `traj14` — the same route, hook-sampled per
logic frame — now carries it as a gate capture (4718 records, 31 checks, zero).
The lesson generalises: a tick-sampled record list is only 1:1 with logic frames
while no frame overruns, so `death_capture*`/`trajectory_capture14.py` take
records in a `$05E4` hook and pair them with `apply_pairing.py`.

The east extension (`traj11`, 6470 frames, x 3094 → 3586) landed 2026-10-12
for free once the bobber was read out of the ROM instead of nudged: the anim-15
marker (a static type `$85` no-op) and the walker at 3352 need nothing new, and
the anim-12 bobber at 3456 turned out to be unjumpable but harmless to *park
next to* (see the semantics list below). Four attempts died first — two to the
walker (a straight-up stomp at dx = −19 is a side contact: traj10's settle loop
lets the walker close in, and the ledge-jump variant drifts into it; jump
*over* it instead) and two to the bobber (any crossing height inside its 21 px
box harms, at both ends of its travel). The route now stops at x=3586, west of
the 496/480/464 tower at 3616 whose spikes (`$81` hazard tiles at 3696-3727)
need the `$1BFF` hazard probe that `player_sim` still omits.

`traj12` (6840 frames, x 3586 → 4026) carried the route over the tower the same
day (2026-10-12) and cost two port fixes, both read off the ROM rather than
guessed:

- **`$1BFF`'s hazard arm** (`player_sim::hazard_probe`): tiles `$50/$51`
  (`& $FE == $50`) at `(x+16, y+22)` for big Mario on his feet, else
  `(x+16, y+28)`; the engine writes `sMarioDead = 15` unless the invincibility or
  star timer runs, and then *overwrites* it with `$80` when `sMarioScreenY >=
  `$80` (the fall-through means that overwrite fires even while invincible). The
  byte lands **before** the `$319F` dispatch, so a hazard hit kills in the *same*
  frame — measured with `death3` (Mario alive at f5685, already `dead = $FF` with
  vvel `$5F` at f5686), and unlike an *object* contact, whose `$2D7D` write
  happens after `$319F` in the frame and therefore dispatches one frame later
  (that is what `death1`/`death2` pin). Nothing between `$1BFF` and `$319F` reads
  `sMarioDead`, so probe-point vs dispatch-point placement is equivalent. The
  `$4A/$4B` exit-tile arm is ported (see the exit-bell bullet below), and the
  big-Mario `$1C39` fall-through to the `y+28` hazard sample was added with it
  (no capture reaches either — every capture runs small).
- **The spawn scan's end-of-stream rule** (`objects::advance_stream`): the stream
  ends with a `$FF` first byte one record past the last, and `$8C56`'s
  `cp $FF / ret z` returns *before storing* the advanced pointer — so consuming
  the **last** record parks `sObjectStreamPtr` on it. The port used to walk one
  record further, and since level 1's last record *is* the cheap `$80`-type
  spawn at x=4008 the divergence began exactly at that record's spawn frame
  (`sptr sim=$AB90 exp=$AB8A`).

`death3` (5807 records, the level-0 route *plus* the tower) is the hazard
death's negative control: after the tower climb Mario walks east off the 448 tier
(row 28 has no cell at 3696-3727) and drops into the `$81` spike cells where the
`$1BFF` probe at `(x+16, y+28)` kills him — the level's only hazard run. It
replays all-zero on all 25 checks including `pcx`, and it is the capture that
caught the dispatch-timing error above: sampled per logic frame (`$05E4` hook +
`apply_pairing.py`), the console showed the same-frame dispatch.

`traj13` (4645 records, 27 checks) closed the item pickup on 2026-10-12. The
whole route replays clean and the pickup itself is checked: the hop crosses the
anim-23 `pose87` at (2792, 416), the console's `sSecretRouteFlag` flips to `$FF`
and so does the sim's. The capture also carries `pow`/`lives`/`inv`/`goal` as new
per-frame checks (all zero). Getting there cost one route insight — the object's
box sits *above* its slot y, so the hop must cross ~50 px higher than the floor
walk does — and one false start: the tick-sampled capture showed the sim spawning
the itempose record two frames early, which read as a 1-frame x step where the
route meets the 2880 ledge and got `traj13` trimmed at record 4645. It was *not*
physics: see the sampling paragraph above — two of the untrimmed run's records
are mid-frame snapshots around the record-16 spawn, and `traj14` now carries
that whole stretch (the spawn, the ledge landing, the fallwalker contact and the
death) as a gate capture.

`traj14` (4718 records, 31 checks) landed 2026-10-12 (wp7f): `trajectory_capture13.py`'s
route verbatim — its tick count, 4719, is the check that the run is the same —
recorded in a `$05E4` hook instead of per tick, i.e. one record per logic frame,
then `apply_pairing.py`. That is what makes the tail replayable at all: `sptr`,
the record-16 spawn frame, the record-14 fallwalker's harmful contact at ~x2920
and the death dispatch all match, and `pcx`/`odd`/`uphase`/`ff0`/`slots_raw` are
zero for the first time on that stretch. The `$1413` push-out itself is still
unexercised (see the push-out entry below).

Verified engine semantics to preserve in the port:

- **Item pickup is a two-frame handshake, and the touched-item cluster is a
  switch.** `$2DC6`'s bit-7 branch only stages `sPendingItemId = type & $7F`; the
  *next* frame's `$0F71` tail runs `$319F` and then `$3451`, which consumes it:
  id 1/2/3 → powerup 1/3/2 + `sInvincibilityTimer = 64` (`$3534`), 4 → star
  (4/64), 5 → 1-UP (BCD, capped at 99), 6 → `sGoalBell = 12` + `vvel = 24` when
  below `$18`, 7 → `sSecretRouteFlag = $FF`, 8 → `sMoonPhysics` bit 2, 9 → +80
  coins (BCD). Id 0 returns **without clearing** (the `and a / ret z`), so the
  levitating item pose and the bobber at rest are inert. Separately, the *touched*
  dispatch (`$8611`/`$868A`) switches on the type: `$86`-`$88` and `$8A`+ do
  nothing, while `$80`-`$85`/`$89` switch the anim (`+5`) to 16, clear the flip
  byte and run `$863C` (`$8240` sprite re-derive + `sObjectDone = 1`) — and the
  **order differs by type**, read off the ROM bytes: `$85` (`$86DA`) writes the
  anim *before* `$863C`, so its sprite is the anim-16 one, while `$80`-`$84`/`$89`
  (`$8695`) run `$863C` first and only then write the anim, so their sprite stays
  the **old** anim's. `$8240`'s 17-byte clear of `+14..+30` is what consumes the
  touch flag and resets the rise counter; the anim the switch installs is the
  pickup arm `$586F5` (ported 2026-10-13): per call it sets `+15 = 1` (arming the
  slot-parity gate from the next frame, so the calls come every other frame), runs
  the `$5B68B` flash, lifts Y by 2 and steps `+24` by 2; on the 16th call
  (`+24` = `$20`) it removes the slot via `$5B2D7` in **remove-mode 2** — the
  despawn keeps the record marker and credits nothing (unlike modes 0/1). Verified
  by `traj13` for the pickup path (id 7 → the `secret` check); the other ids, the
  anim-16 arm and the `$85`/`$89` variants have no capture yet, so they are pinned
  by the `objects.rs` unit tests instead.
- **The coin touch (`$1D75`/`$1DA5`) is ported and console-verified**
  (2026-10-13; `traj16`, measured with `probe_coin_touch.py`). The
  `$1E7C`/`$1E8D` side probes — `$1413`, `$181C` and the `$1189` side-pipe
  travel all call them — run `$1D75` for *every* probed tile at or above `$38`
  (`cp $38` → carry ends the probe as a wall hit, otherwise the touch runs and
  the next probe point follows, so one frame can collect at more than one point,
  in call order). `sCollisionTile & $FE == $48` (level 1's `$48`/`$49`, both
  class `coin`) requests sfx 7, runs `$1DA5` — the level-map write at the
  probe's own cell via `$2C0D` → bank-1 `$5CF2`: exactly `$48` → 96 (`$60`,
  empty air), the `& $FE`-passing `$49` → 127 (`$7F`); both non-solid, so
  physics is unchanged — and credits the BCD counter (`inc a`/`daa` + `adc 0`,
  `$0999`-clamped; the same tail as the `$01` bonk's `$591F`). The port runs it
  inside `PlayerSim::wall_tile`, its write lands in `layout_writes` (a shared
  cell helper with `$5CF1`), and every later probe's `tile()` sees the emptied
  cell, so a coin cannot double-credit. Measured on the console (the traj12
  route is the one that reads level 1's only `$48` cells, (24,231)/(24,232),
  floating in the air above the tower at x 3688-3719 / y 384-415): the counter
  goes `00` → `01` → `02` at the frames whose state is (3691, 383) / (3700,
  365), and the two cells are rewritten `$48` → `$60` on those same ticks — the
  port's `coin_touch` frames, count and writes match 1:1, and `traj16` (the same
  route, `$05E4`-hooked, carrying `coins`/`coinshi`) replays all-zero on all 33
  checks (the negative control — the touch disabled — turns it into 117 `coins`
  mismatches). `render_view` now takes the overlay as a parameter (the plain
  render and the window viewer pass `&[]`), so a collected coin's metatile
  disappears on screen: on the `--filmstrip` render of `traj16`, cells
  (24,231)/(24,232) change from the `$48` coin art to the blank `$60` metatile
  at the collection frames, pixel-identical to their `$60` neighbours.
- **The brick-bump ("bounce block") system is ported and verified** (2026-10-13,
  wp7g; `traj15`). `$1993` classifies the tile the `$1DFE` ceiling probe just
  bonked and `$5960`/`$2C6C` allocates into one of 8 `sBlockAnimSlots` ($AA80,
  `$10` bytes: type/timer/Y/X); `$2C82` → `$5997` ticks them **after** the frame
  selection (`$2D0A`) and before the object engine, so the last drawing slot's
  `$FFC6` is what the frame ends with (and what the `frame` check sees). Types,
  measured on level 1's `$01` block at col 22 with `probe_block_bump.py`: `$7E`/
  `$02` → 3, `$00` → 2 (small) / 1 (big), `$03` → 6 (small) / 5 (big), `$01` →
  the `sSpinJumpBounce` ($A220) flag, a type-3 slot, then `$2C77`/`$591F` +1
  coin (BCD, `$0999`-clamped) and a type-4 slot. So a *single* `$01` bonk
  allocates **two** slots: the type 3 (the `$5B06` bump, `$FFC6 = 66` at timers
  3-7, freed at 8) and the type 4 (the coin popping out, the `$5AF9` cycle
  `[$6D,$6E,$6F,$6E]` by `(timer & $0C) >> 2` until `$32`) — second in the scan,
  so the cycle's 109/110/111 is the frame that survives, and `66` is only
  observable from a *single*-slot bonk. The draws are positioned at the block's
  `$FFC4`/`$FFC5` (Y from the `$5AE0` table).
  The bump also rewrites the **level map**: `$5CF1` stores a tile into WRAM's
  copy at the slot's cell (derived from the rounded Y/X), so every later probe
  reads it. Measured: `(28,22)` `$01` → `$24` at timer 2 → `$07` at timer 8, so
  a second bonk there hits `$07` — a `$04`+ tile, i.e. a plain bonk that clears
  `sSpinJumpBounce` with no bump and no coin (exactly what `traj15`'s second and
  third jumps do). Per type: 1 → 96 ($60), 5 → 127 ($7F) — the block is
  *destroyed*, passable; 2 → 24 then 0; 3 → 24 then 7; 6 → 24 then 3; 4 never.
  Small Mario's `$00`/`$03` bumps therefore leave the block bumpable again; big
  Mario's break it. The engine's *other* bump writes — `$5C01`/`$5C2C`/`$5BD3`'s
  `$FF`/`$F4-$FB`/`$F8-$FB` 2x2 and `$200C`'s `$97E0`-based address — go to the
  BG map (VRAM) and are art only; the port does not model them.
  What *is* modelled is the draw itself. Every drawing path stages
  `$FFC5 = slotX - sScrollX` (types 1/5 also ∓ the timer) and
  `$FFC4 = slotY - sScrollY + offset[timer]` — three offset tables: `$5BB4`
  (types 2/3/6, 8 entries), `$5AE0` (type 4, 25 entries by `timer >> 1`),
  `$5A4A` (types 1/5, 40 entries, two sprites at `x ∓ timer`) — then `$5297`
  enqueues it, so a bump is an *extra* OAM entry drawn at the block while
  Mario's own sprite keeps his pose. `PlayerSim::block_draw` stages it as
  `(x, y, frame)` in the port's sprite convention; the anchors are
  `slotX - 16` and `slotY + offset - 26` (the engine's bump pair is
  scroll-relative with no player `+96/+98`, and its scroll sits at `cam - 80` /
  `cam - 72` — `render_view`'s native-rect origin — so the two conventions
  differ by exactly those amounts). Verified numerically against the console's
  own staging: `probe_block_bump.py` now logs `$FFC4`/`$FFC5`/`$FFC6`, and at
  the `traj15` coin's timer-2 frame the engine stages 80/71 for slot (472, 368)
  with camera (448, 377) — the port's base computes 80/71. `run_play` and the
  replay filmstrip push `block_draw` through the player-sprite path
  (`SpriteCtx::bumpers`), so the block bump and the coin pop are both on screen.
  `$076B`'s status-bar patch is modelled too (`hud_row_coins`), so the coin
  digits follow `sCoinLow`/`sCoinHigh` the frame the coin is credited. Still
  unmodelled and knowingly visible: the BG 2x2 art (`$FF`/`$F4-$FB`/`$F8-$FB`,
  so a bumped block's *tile* art never changes after the animation) and the
  death fade's palette steps. Away from the
  bump, `$530D`'s own arms can only produce the values the port already
  reproduces (`1+cell`/`5+cell` walk, 8/10/11 air, 12/13 skid, 14/15 duck, 16
  pipe, 18/20/21 moon) plus one of `+0`/`+32`/`+69`, `+26` more with the powerup-2
  flutter bit, `+112` for powerup 3 — the +26 needs `sCurPowerup == 2`, so a
  "small" frame vocabulary (`33`-`36`) rules it out.
- **The below-hit system (`$2F72` → `$85AA` bit5 → `$86A0`) is ported**
  (2026-10-13). The flag's only writer is `$2F72`, the tail of the block-bump
  slot alloc: it runs *only* on a frame whose ceiling bonk allocated a slot
  (the `cp $00 / ret z` walk bails before it), rewrites `$1DFE`'s world probe
  pair into screen bytes — `(Y & $F0) + 8 − sScrollY − 16`, `((X + 8) & $F0) −
  sScrollX − 8`, i.e. the bonked *tile's centre* — and boxes each idle
  object's staged `+9/+10` against it (|dy| < `$10`, |dx| < `$14`, modular;
  a flagged or type-0 slot is skipped). A type-`$80` pose inside the window
  raises `sObjectBumpSpecial`, which `$19D2` reads *before* the coin: the
  `$01` bonk withholds its +1 coin and its type-4 slot. The pass runs inside
  `$0F71` — before `$0B2A`/`$2062` recompute the scroll, so it reads the
  *previous* frame's `sScroll` — while its special flag is player-side and its
  slot walk object-side, so the frame loop resolves the two halves
  (`ObjectSim::below_bonk` + `PlayerSim::resolve_bump_coin`) between the
  player step and `$8000`; `$19D2`'s coin is therefore deferred by one
  call in the port. The same frame's object engine then turns the flag into
  `$86A0` (`sClearStageFlags = 1`), where a bit-7 type takes `$86E4`: low
  bits non-zero (`$86` the bell, `$87`/`$88`, `$8A`+) → `$8721`, the type held
  across `$863C`'s `+14..+30` clear and `+27 = 2` arming the *revive*
  reaction (`$5816C`'s landing branch re-derives and drops the object into
  frame state 2/3); `$80` alone → `$8708` (anim `$11` with a powerup first
  rewrites the spawn record's anim byte to 18 and stages anim 18 / sprite 26;
  then type = anim + `$70`, `+15 = 1`, Y += 16, `+2 = $10`). The plain types
  keep the old arms: `+15 = 0`, `$86C6`'s anim ± 1 for the `$10` class (bit3
  set → anim − 1), the `$85F0` defeat for `$08`. Fixed on the way: `$86D2`'s
  arm was ported as "sfx 3 only" — the ROM decrements the anim there
  (`$85D7`/`$85DC`) and re-derives the sprite. No capture in the suite
  reaches a below-hit (the window wants an object *on* the bonked tile —
  traj15's walker is 100+ px away), so the arms are pinned by a console
  measurement instead: `captures/probe_below_hit.py` pokes the flag onto an
  active anim-22/type-`$86` slot and shows the sprite re-derive (`$7F` →
  `$1F`), the surviving type, `+27` = 2, and then the face-Mario/flip and the
  `$411A` knockback — the same sequence the `objects.rs` test
  `stepped_below_hit_on_the_bell_re_derives_and_faces_mario` replays. traj15's
  all-zero `coins`/`bslots` is what pins the deferred coin as behavior-neutral.
- **The spin jump (`$126F`) and its landing bonk/bounce (`$1327` → `$1993`)
  are ported** (2026-10-13, `spinjump_block_destroy`; no capture yet —
  unit-pinned). `$126F` runs between the ducking and jump stages and engages
  while DOWN is held (`hKeysHeld` bit 7), `sCurPowerup` is odd (bit 0 —
  mushroom/fire, not small or bunny), Mario is airborne and there is no moon
  physics; the engage edge requests sfx 14, anything else clears the flag. The
  landing arm runs *after* the `$12CE` `$04/$05` side-push code and *before*
  `$134F`: with `sSpinJump` set, the floor hit calls `$1993` on the landed
  tile, and when the classification set `sSpinJumpBounce` the landing becomes
  a bounce — `sMarioInAir = $FF`, vvel 48 (75 while A is held — `hKeysHeld`
  bit 0), on-ground cleared, and the whole `$134F` tail (on-ground = $FF, the
  pipe re-arm, the `$1366` align nudge) is skipped by the `ret`. So a spinning
  Mario *destroys* a `$00` brick underfoot (the `$19B0` big arm: `sfx2` + the
  type-1 bump whose `$5CF1` write turns the cell into `$60`) or rattles it as
  small (type 2); `$03` maps the same way (6 small / 5 big); a non-class tile
  (plain ground) leaves the bounce flag clear and falls through to a normal
  landing. No route in the suite holds DOWN mid-air on an
  odd powerup (every capture runs `pow = 0`, and `run_replay`/`run_play` seed
  `sCurPowerup = 0`), so the branch is pinned by the `spin_jump_*` /
  `*_landing_*` unit tests rather than a replay; `MarioContact.spin_jump` now
  carries the real byte to the object engine's stomp dispatch (`$85C4`'s
  defeat arm — also capture-less, same engagement gate). The `$532D` spin
  *pose* is still unmodelled: its table index is `$FF97 & $0C` and captures
  record only `& 7`.
- **`sMarioOnObject` comes from the object pass, never from the floor probe**
  (closed 2026-10-13). `$2D7D` clears it at the top of the interaction pass and
  `$2DF7`/`$2E1A` raises it to 255 when Mario's contact box meets an object
  whose `sObjectType & $23` has bit 5 — the stand-on walkers/platforms. It is a
  cross-frame byte: the pass writes it, and the *next* frame's `$129F` reads it
  to short-circuit to `$1378` (`sMarioOnGround = 255`, `sPipeTravelDirection`
  cleared, the floor probe skipped entirely), while `$5375`/`$53FE`'s frame
  test and `$307D`'s pan gate read it too. A pass the frame loop skips (pipe
  travel) leaves the byte at its previous value, so the port only writes it
  when the pass actually runs. Until this was threaded, all three readers saw
  `false` forever — and no capture could have shown it: level 1 has no object
  type with bit 5 in `& $23` (`$4E` ×38 and `$56` ×1 are the only types Mario
  ever contacts, both `& $23 == $02`), so the branch is unit-pinned and its
  capture is quarantined in the queue as `obj_on_object_capture`.
- **The two frame loops share one `MarioContact` constructor** (`build_contact`,
  same date). They used to assemble it separately, and the play loop passed
  defaults for `star_high`, `invincibility` and `on_object` while the replay
  path passed the sim's values — plus the play loop never performed the
  `on_ground_cleared` write-back. Three silent divergences no capture can show,
  because only `--play` had them; this is the class queued as
  `infra_play_parity_harness`. `pan`, `scene_state` and `flutter` are still 0 in
  both paths: nothing drives `sCameraPanTimer`, `sSceneState` or
  `sFlutterTimer` yet.
- **The bell strike saves no respawn coordinates** (measured 2026-10-13,
  `captures/probe_bell_checkpoint.py`). The "midpoint bell saves the
  checkpoint" premise is not in the ROM: `$34EC` (item id 6) writes only
  `sSfxRequest0`/`sGoalBell`/`sLowTimeFlag`/`sVVelocityIndex`, and the only
  writers of `$A800-$A803` are `LoadLevelHeader` (`$5589`/`$559F`, the
  header's 20-byte block) and the secret-route overlay (`$55C0`, the per-level
  8-byte records at `UnknownData_0x377B`). On the console, poking
  `sPendingItemId = 6` mid-level sets `sGoalBell = 12` and lifts vvel, while
  `$A800-$A807` stay byte-identical across 119 logic frames and ~85 px of
  walking — `$0F2A` only *reads* the block, so the respawn point is the
  header's spawn (or the secret route's) and the bell never moves it. The
  bell's *strike* is the touch branch, ported since traj13: `$2DC6`'s bit-7
  arm stages `type & $7F` = 6 and the next frame's `$3451` sets `sGoalBell`.
- **The `$1BFF` exit bell (`$1CBA`-`$1D01`) is ported and unit-verified**
  (2026-10-13). The same `(x+16, y+28)` sample the water arm reads also
  carries the `$4A/$4B` exit pair (`sCollisionTile & $FE == $4A`): `$4A` is the
  plain exit, `$4B` the "route already cleared" twin that sets
  `sExitTileFlag = $FF`. The arm clears `sLowTimeFlag`, requests the
  level-clear music (`sMusicRequest = 3`), clears the SFX request bytes,
  sets `$FF9B = 24`, arms the 192-frame fade (`sFadeTimer`), and clears
  `sSecretRouteFlag` + `sTileAnimConfig`. `$FF9B = 24` (state `$18`,
  `$3547`) replaces the in-level loop exactly like the death fade's
  `$FF9B = 9`, so `PlayerSim::exit_phase` freezes the port the same way
  `dead_fade` does (the `$1BFF` call itself still runs to the end of its own
  frame). The hazard arm wins outright — it `jp`s the tail — so a `$50/$51`
  tile at the big-Mario `y+22` probe blocks the bell even with a `$4A` one
  row below. Level 1's only bell is the `$4A` at col 254 / row 30 (world
  x 4064), past every capture's east end (traj12 stops at 4026), so the arm
  is pinned by unit tests (`exit_bell_*`, `water_beats_the_exit_arm_*`)
  rather than a replay; the hazard-vs-bell interaction and the big-Mario
  `$1C39` fall-through are pinned the same way. **The fade's tail is ported
  too** (`PlayerSim::exit_fade_step`, 2026-10-13): VBlank keeps decrementing
  `sFadeTimer` ($A266/$A267, saturating at 0) while state `$18` polls it, so
  every post-arm frame ticks the port's timer once; the call that reaches 0
  is `$3547`'s tail — `sLevelTally` ($A2C5) is incremented in BCD
  (`add 1 / daa`) unless `sLevelProgressFlags` ($A27D) bit 7 (the stage was
  already cleared) or `sExitTileFlag` is set, then `$FF9B` moves to 20
  (`$14`, `PlayerSim::exit_completed`). Still open: the `$14` hand-off's
  banked body (`$3EF2B`: the palette swap, the map-node advance and the
  save) — the port marks the hand-off and keeps freezing. Unit-pinned
  (`exit_fade_*`; level 1's bell is past every capture's east end).
- **The bobber (anim 12, `$584F3`/`$5850C`) is a harmless item until you leave.**
  State 0 writes type `$80` (`sObjectType`) and then `call $5B4B4 / cp $18 /
  ret c` — it *returns while Mario is within 24 px*, so it cannot arm while he
  stands next to it. Outside 24 px it sets state 1, `+24` = `+25` = 32 and type
  `$44`, then travels 1 px/frame: 32 frames up, 32 down, 24 rest. Type `$44` has
  bit 6 set, so `$2E6F` kills, and `$44 & $23 == 0` sends **both** arms of the
  `$2DC6` response (the `y_delta >= $0C` arm and the type arm) to the harm path —
  there is no stomp. Its box is x ∈ [objX−19, objX+2), y ∈ [objY−51, objY−25),
  which overlaps a walker standing on the 448 ledge (Mario's y=436) at the top
  *and* the bottom of the 28 px travel, so no jump height or jump timing clears
  it. The crossing is: park inside the 24 px trigger but outside the 21 px box
  (x ∈ (objX−24, objX−19)), wait for state 0, then walk through — the contact
  takes the `$80` branch, writing `sPendingItemId = 0`, which the `$3451`
  consumer drops.
- **Pause flashes the palettes** — ported 2026-10-13. It is game state
  `$FF9B = 8` (`$273E` entry at the frame tail of `$05D5`/`$0640`, handler
  `$2756`), not a separate mode: the whole state-`$04` body is replaced, so the
  player, camera and object engine stop while VBlank, `$FF97`, the sound
  update and the scene script interpreter keep running. The flash is **bit 4 of
  the `$FF97` counter read every frame** — `$E7` when clear, `$93` when set,
  i.e. 16 frames of each, a 32-frame cycle with no timer and no reload — and
  the *same* byte is written to `sBGPalette` **and** `sOAMPalette1`
  (`home.asm:5204-5206`), so BG and OBJ0 flash together while `sOAMPalette2`
  (i.e. `OBP1`) is never touched. Entry needs `hKeysPressed == $08` exactly
  plus `sScreenTransition == 0`; the in-level loop blanks both key bytes while
  `sFadeTimer != 0`, so START is ignored during a fade. Resume is any frame
  with the START press bit, restoring `sBGPalette`/`sOAMPalette1`/
  `sOAMPalette2` from the `sLevel*` copies at `$A80F-$A811`. SELECT with
  `sLevelProgressFlags` bit 7 set abandons the level through `$FF9B = 20` →
  `$14` instead. Ported as `PlayerSim::paused` + `pause_enter`/`pause_exit`/
  `pause_flash`/`pause_select_exit`, with `frame_counter` kept as the full
  `$FF97` byte (the port used to keep only `phase`, its low three bits, which
  cannot see bit 4); `--play` maps ENTER/BACKSPACE and freezes the frame the
  same way a fade does. `sSoundMode = 1/2`, `sPauseState` (`$A23D`, write-only
  in the ROM), `sMarioDead = 2` and `sFastMusic` are flagged as unmodelled.
- **`--play` seeds its per-level camera flags from the manifest** (2026-10-13):
  `sMapScrollLimit` is a per-level byte from `LevelPropertiesUNK` (written only
  at `$04F1`, `home.asm:741-747`) and `sAutoScroll` from
  `LevelPropertiesScroll`, both already in the manifest as
  `properties.unknown` / `properties.auto_scroll`. The play path used to leave
  them at their defaults while `run_replay` took them from a capture's frame 0
  — the same shape of gap the missing edge map had. Level 1 is `$00` in both;
  the three autoscroll levels still print the not-modelled note, since `$0DAF`
  is unported.
- **Sprites**: mapping offsets land in the RAW OAM byte (screen =
  `obj + offset - (8,16)`); OAM attr bit5 = X flip, bit6 = Y flip, bit4 =
  OBP 0/1 select; OBP select also comes through `sSpriteFlipFlags` bit 4.
  OBJ tiles use the `$8000` mode at play (`$8000 + t*16`).
- **The follow camera leads Mario** when running (origin = sScroll; the
  `camera - 80` relation holds at spawn only).
- `$38-$47` are flat-topped cells (no per-pixel slope): walking across
  different slope tiles keeps Y constant; sides don't block. The "diagonals"
  are staircase arrangements + art, so the collision model is the shipped
  class function alone.
- **Player physics invariants**: `hvel` is a speed INDEX centred at `$80`
  (>`$80` moving right, <`$80` left; per frame the engine applies accel ±8
  AND friction ∓4 whenever grounded, so the net ramp is ±4 with caps `$BF`
  walk / `$F3` run). `vvel` is a vertical index (75 jump, 33 after jump-cut,
  low = near-ground slow, 0 = terminal fall, `$30`→`$24` apex snap). Both
  positions move by *signed* speed bytes where `$F0-$FF` mean −16..−1 (the
  `add b / sub 16` trick); probe points: floor (x+12/x+20, y+44, tile <
  `$48`), ceiling (x+16, y+18/+28 small, hits on `$7E` or < `$38`), walls
  (x+11/x+20, y+20/+32/+40 for big, /+32/+40 for small/duck, tile < `$38`).
  Walk-anim timer advances +1/frame (+2 with B) and wraps to 0 at `$0C`.
- **The `$1413` push-out writes x *and* `sMarioXSub`, so a pushed-out frame
  moves 2 px** (ROM-verified, no capture yet — it is unreachable in level 1).
  `$1432` probes left (x+11) then right (x+20); on a hit `$144B`/`$148D` do
  `x ± 1` (16-bit), `sMarioXSub ± 1` and `sMarioXPushed = $FF` (`$A218` is
  written nowhere else). `$181C` then adds the *whole* new `sMarioXSub` to x
  (`$3404-$3415`: `add b` on the accumulated byte, then the `add 16`/`sub 16`
  signed-add trick — verified against the ROM bytes, not just the listing), so
  the ±1 lands a second time: `x += ±1` then `x += speed ± 1`. A frame that
  then trips the wall probe reverts to `sMarioX`/`sMarioSubSaved` (the
  post-push-out values), which is the 1-px "click" out of a wall. The port used
  to add only the speed byte in `$181C`; fixed 2026-10-12. Level 1 never fires
  it: 0 push-out frames traced across traj4_ff0/traj8-13/death1-3 (`$A218`
  is set only there), and nothing in the level can reach that state — no `$0C-$0F`
  side-pipe or `$04/$05` side-push tiles, no `$0FBF` camera clamp and no
  horizontal screen transition — the only ways into a solid column's x-range.
  To exercise it, find a level with a side pipe (or a `$04/$05` landing next to
  a wall) and capture the entry: `$143D` arms the side pipe from the push-out
  itself.
- **Camera invariants**: the camera only scrolls a direction while its
  enable is set; the draw sets the enable for the side Mario is on (screen X
  ≥ `$60` → right, < `$60` → left) and a scroll clears the opposite. The
  applied delta = the player's per-frame speed; after it, a ±1 nudge parks
  Mario at screen X `$3F/$40` (right) or `$70/$71` (left) — the dead zones
  the replays key on. Column locks (`$B0`-row bit0, `$50`-column bit1) and
  row locks (`$48` up / `$C0` down) come from the per-sector
  `sScreenEdgeMap` (`levels/*.edge.bin`, extracted only as of 2026-10-13 — the
  correction above; the replay check `edge` now derives `$FF98` from it and
  compares the capture); with bit1 set, a camera below the
  `$50` column creeps back +1/frame (observed live and required for
  parity). Vertical: follows the player's Y change only outside the
  [`$28`(`$40` with `sMapScrollLimit`), `$50`] dead zone.
- **Autoscroll levels replace that whole horizontal section** (`$0DAF`,
  ported 2026-10-13; the levels are 20, 26 and 31 by file name, i.e.
  `--level 19/25/30`). With `sAutoScroll != 0` the `$0B2A` dispatcher calls
  `$0DAF` instead of the right/left chains and then jumps straight to `$0C7D`,
  so the vertical follow is shared but the horizontal rules are entirely
  different: `$0DAF` never applies the player-speed latches, never reads
  either enable, has no ±1 park nudge and no `$FF99` pull-back, and no left
  edge lock or left transition. It does exactly three things — a right-edge
  trigger (`>= $B0` where `$0B5E` tests `== $B0`, and a screen X below `$A1`
  `ret`s outright rather than falling through), a **one-pixel-per-two-frames**
  scroll (`$FF97 & 1` must be clear, and the enabled-by-default left scroll is
  cleared with `sMapScrollFlags` bit 4 set), and nothing else. Mario is not
  driven by the camera at all on those levels: he keeps normal physics and
  drifts toward the left of the screen. Because `$0DAF` jumps to `$0C7D`, it
  skips `$0C76`'s `sScrollDelta*` clear — the port derives its deltas from
  `x_sub` every frame, so that has no analogue here. No capture in the suite is
  an autoscroll level, so this path is unit-pinned only (6 tests) plus a
  play-mode smoke run; the capture is queued as `autoscroll_capture`.
