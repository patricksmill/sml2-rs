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
- **Presentation latency is not a parity target.** The console shows a
  tick's result one frame later (it draws from the previous frame's VBlank
  shadows); the native renderer shows it on the tick that produced it. That
  lower latency is intentional — this is a native port, not an emulator.
  ROM-versus-port pixel comparisons pair ROM frame N with port frame N-1
  (`tools/compare_tas.py --pixel-report`'s `diff`).

### 3. Variable screen sizes & the display pipeline

- **The viewport is a parameter, not a constant.** `--size WxH` renders any
  world window centred on the camera, clamped so the viewport stays inside
  the level (16x16 up to the full 4096x768 world). The native 160x144 is the
  default AND the parity anchor. At 160×144 the HUD strip is in the native
  rect; in any larger view it is bottom-centred (the desktop's Widescreen and
  Camera zoom mods). With the status bar excluded, the centre 160×144 slice of
  a larger level frame is pixel-identical to the native frame — re-assert
  `crates/sml2-runtime/src/tests/widescreen.rs` after any renderer change.
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

**A nonzero `sSceneId` makes the level run a script, one byte per frame**
(2026-10-14). The main loop calls `$20A4` once per pass, *after* the in-level
frame pass (`home.asm:274`), and every opcode in it ends at `$21E8` — a `$2728`
VBlank wait — before the next fetch. The table at bank 5 `$4125` is 32 `dw`
entries pointing into the 128 bytes just above it; `sml2-extract` writes the
whole 192-byte region as `extracted/scenes/script_table.bin` and
`crate::scene` resolves the pointers against it. `$FF` ends the script through
the `$21EE` epilogue, which latches `sSceneTransitionFlag` into
`sSceneTransitionLatch` and zeroes `sSceneId`, `sSceneIdHi`, the flag and
`sSceneDoneFlag` — this is also where `$20A4`'s no-scene guard jumps, so the
epilogue runs on every scene-less pass, idempotently.

The port implements `$4x` and `$8x`. `$4x` — "transition to screen (Y,X)" — is a
**teleport of Mario *and* the camera**: the operand's high nibble becomes their
Y page (`$FFC9`/`$FFC1`), its low nibble their X page (`$FFCB`/`$FFC3`), and
only those high bytes are written, so the pair lands on a 256-px cell boundary
with the low bytes untouched. `$8x` arms `sBossDefeated = $F0 | nibble`. Every
other class is stepped over *with its real operand length* and reported, so a
script still reaches its `$FF`; a class with no handler at all (`$7x`, `$9x`,
`$Cx`-`$Fx`) makes the ROM re-read the same byte forever, reproduced as a
no-advance stall. The operand lengths are validated by the data: all 32 retail
scripts parse into whole instructions landing exactly on their `$FF`. Two are
easy to get wrong and both were, when this was first researched: **`$6x` takes
no operand** (its `$25AF` does `ld a,[hli]` with HL on the opcode, so the nibble
*is* the tileset index) and `$Bx` takes three.

The `$4x` handler's direction streamers (`$2359`/`$23E4`/`$2524`/`$246F`, run
only while a transition is in progress) write `$FFCC`-`$FFCF` and call
`$0969`/`$0A35`, which fill the BG window at `$9800` with the incoming screen's
strips. They are deliberately not modelled and not flagged: this port renders
from the resident decompressed layout, so they have no observable effect here.
The transition scroll itself (`$0DED`) uses hardcoded targets (`cp $50` etc.)
and never reads those bytes.

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
bytes with no in-level writer, so 0 here), the six golden-coin bytes then
**wiped**, `sFadeTimerLow = 255`, `sMusicRequest = 7`, and a `$2934` save, then
`$FF9B = 7`. State `$07` (`$287E`) calls the bank-12 `$30884` (which stages the
six golden-coin *icons*), polls `sFadeTimer` to 0, then waits for START and hands
back to `$FF9B = 0` (`Init`) with `sMusicRequest = $FF`. Two corrections worth
carrying: the START test is an **exact** compare (`ldh a,[hKeysPressed] /
cp $08`), so START held with A or SELECT does not advance; and the `$2874`
"GAME OVER" string (`db $90,$8A,$96,$8E,$FF,$98,$9F,$8E,$9B,$80`) is **dead
data** — no code in the ROM reads it, and the screen's actual title is an OAM
mural (tiles `$50-$79`, mappings 227/228 alternating on `$FF97` bit 4, sliding up
during the fade's first 64 frames). Modelled as `PlayerSim::game_over` +
`game_over_enter`/`game_over_step`; the art, the screen's own `$2934` save (the
record builder is the same one the hand-off uses) and the mural are not (see the
omissions list above). No capture reaches zero
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
What remained missing was not a "state 240" but the bank-5 **script VM**
(`$20A4`, one byte per frame, opcode table at home.asm:4311-4429), the `$0EB0`
`sSceneState & $0F` branch, and the `$8D9C` path. As of 2026-10-14,
`crate::scene` ports the `$20A4` VM with `$2x`, `$3x`, `$4x`, `$6x`, and `$8x`
implemented and operand lengths parsed for all 32 scripts, leaving `$0EB0`'s
`$3C94` → bank 2 `$8D9C`/`$8E6C` boss/template loader, remaining opcode handlers,
and capture verification (`scene_capture`). `sSceneMap` is extracted the
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
- **The `$1BFF` exit bell (`$1CBA`-`$1D01`) is ported and CONSOLE-verified**
  (2026-10-14, `captures/traj17.json`; unit-verified since 2026-10-13). The same
  `(x+16, y+28)` sample the water arm reads also
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
  x 4064); `traj17` walks east from the tower onto it (the arm fires with
  Mario at x=4057, `sCollisionTile = $4A`), so the trigger, the arm's writes
  and the fade are now replay-verified, and the unit tests
  (`exit_bell_*`, `water_beats_the_exit_arm_*`) keep the big-Mario
  `$1C39` fall-through and the `$4B` twin pinned — no capture reaches either.
  **The fade's tail is ported too** (`PlayerSim::exit_fade_step`, 2026-10-13):
  VBlank keeps decrementing `sFadeTimer` ($A266/$A267, saturating at 0) while
  state `$18` polls it, so every post-arm frame ticks the port's timer once;
  the call that reaches 0 is `$3547`'s tail — `sLevelTally` ($A2C5) is
  incremented in BCD (`add 1 / daa`) unless `sLevelProgressFlags` ($A27D)
  bit 7 (the stage was already cleared) or `sExitTileFlag` is set, then
  `$FF9B` moves to 20 (`$14`, `PlayerSim::exit_completed`). `traj17` carries
  all 192 fade frames (state `$18` holds for 191 further ticks) and the tally
  charge `00` → `01`, checked per frame through the new `fade`/`tally`/`exit`/
  `lowtime` record fields; its one `exit arm:` line adds what a per-frame table
  cannot state (the trigger's geometry and the fade's span, measured outside
  the in-level hook because `$05E4` stops firing once the state leaves 4 — see
  the two-hook note in `captures/README.md`). The `$14` hand-off's
  banked body (`$3EF2B`: the palette swap, the map-node advance and the
  save) was ported 2026-10-14 (`map_handoff_14_body`), unit-tested across
  `map.rs` and `player_sim.rs`; `traj17` reaches the hand-off frame itself,
  though post-hand-off frames remain unexercised in captures.
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
  autoscroll (`$0DAF`) was ported 2026-10-13 (`camera_autoscroll_path`) and is
  gate-verified as of 2026-10-14: `captures/traj19.json` (level 20, replayed at
  `--level 19`) reproduces it all-zero across 1308 frames, so the capture-less
  `note:` line both loops used to print is retired.
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
- **`sPrevMarioY` is latched from the *live* `$FFC0`** (2026-10-14). The `$0C7D`
  body ends at `$0D97` with `ldh a,[$FFC0] / ld [sPrevMarioY],a`, so on the one
  frame the body itself moves Mario — the up-lock warp writes `$FFC0 = 0`
  (`home.asm:1786`) and falls into that same tail — the latch picks up the
  *warped* value, and the next frame's `dv = $FFC0 − sPrevMarioY` is measured
  from it. The port latched the value the body entered with, leaving `dv` short
  by the pre-warp low byte on every frame after an up transition. Measured
  before changing it: body-warps fire in 8 of the 18 captures, the `dv` *branch*
  never flips in any of them, and the gate is byte-identical with the live latch
  — so this is a fidelity fix with no level-1 decision behind it.
  One conflation remains, deliberately: `prev_mario_y_lo` is still one field for
  two ROM bytes. The movement wants `$A20C` (this latch, now correct); the
  `$C0`/`$48` hard-edge reads want the draw's `sMarioScreenY` (`$A23B`), which
  the port models as `sy98 = prev_mario_y_lo − cam_y_lo + $62`. Those two bytes
  differ only on the frames the register is not consulted (the first frame after
  `$0F2A`, transition frames, parked frames), and the register's own readers are
  the two lock rows the transition ends never straddle with a stale value — so
  the proxy is right on every frame it is read.
- **The `$14` level-clear hand-off** (ported 2026-10-14). `$2C23` maps bank 15
  in and jumps to its `$6F2B` (file offset `$3EF2B`), and the body runs **twice**
  on a bell clear. Pass 1, only when `sGoalBell` is set (item id 6, `$34EC`),
  jumps to `$3F4B1` (even node) or `$3F5A4` (odd) — the post-clear walk cutscene
  (bank-25 gfx, the castle RAM block, `sMapScreenId` 13/14) — which **clears
  `sGoalBell`** and hands to state `$0C`; the cutscene's own end calls `$3EF2B`
  again for pass 2. Pass 2 is the map return: the `$4B` secret-exit node rewrite
  (live Mario only — `$48→$47`, `$05`/`$12`/`$1A→$69`, `$29→$2C`, `$2A→$2F`,
  `$3C→$40`), then `$3EEF0` marking the node's **`sLevelProgress`** entry, then
  the node tail and the hub setup, ending at `$FF9B = $0C` with `sMapScreenId` 0.
  The port has no cutscene, so it collapses the two passes into one call, which
  it must: skipping pass 2 would mean a bell clear never marks progress.
- **`sLevelProgress` (`$A848`) is not `sLevelProgressFlags` (`$A27D`).** The
  hand-off writes the *persisted* per-level array, indexed by the node's
  `$650E` progress slot: `set 7` + `res 3` on a plain clear, `set 3` for a dead
  Mario on the secret route, `res 3` for the `$4B` exit. Bit 7 is "this level has
  been cleared" and bit 3 "entered by the secret route". `$A27D` is the *working
  copy* the map takes from that array when a level is entered, and it is what the
  `$3547` tally guard and the `$2756` SELECT abandon read — which is why the port
  keeps both fields. The node tail's `or a / jr nz` keys on the node being **0**
  (a plain level, level 1 included), and that is the branch that marks the stage
  cleared and moves `sMapNode` to `$14`; the `$05`/`$1A` nodes move to the castle
  node `$3F`, and nodes of `$1E` and up take map screens this port does not
  model.
- **The `$2934` save record** (modelled and validated, not persisted). 0x50 bytes
  to `sSaveSlot0` (`$A000`) + `sSaveFileIndex` * `$50`: the 64-byte image at
  `$A840` (`sMapNode`, the cursor, `sLevelProgress`, the golden-coin flags, the
  map's gates), then `sCoinLow`, `sCoinHigh`, `sLevelTally`, `sLives`,
  `sMoneyBagCount`, and the sum of those 69 bytes as a checksum. `$28EC` accepts a
  slot only if that sum matches *and* the five signature bytes (`$12 $34 $56 $78
  $78`, written by the fresh-file init rather than by `$2934`) are intact. The
  port builds and checks the record ([`PlayerSim::save_record`],
  `map::save_record_valid`) but stores nothing — it has no SRAM.
- **What the hand-off's unmodelled remainder is:** the palette swap to the
  overworld's `$E4`/`$D0`/`$39` (this port renders the level from the extracted
  manifest and has no map to draw), the cutscene and the hub/GFX setup at
  `$3F156`, and the `$1E`-and-up map-screen branches. Reachability: `captures/traj17.json`
  (closing `exit_tile_arm_capture`) reaches the exit bell at x=4057 and completes
  the fade to land on the `$14` hand-off frame itself, though the post-hand-off
  frames are unexercised in captures (verified via unit tests).
- **The overworld's node tables** are extracted as `map/nodes.bin`: two parallel
  130-byte arrays, `$4218[node]` = the level index the map stores into
  `sCurLevel` (`$FE`/`$FF` = a pure path node; 32 nodes carry a level) and
  `$650E[node]` = the `sLevelProgress` offset (and the `$6591` key-mask index; 34
  nodes carry one, with the six castle slots at nodes `$20`-`$24`). Node 0 — level
  1 — is `0` in both. A node is a map *position*, not a level: the same level can
  be reachable from several nodes. (`crate::map::MapNodes`.)
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

## The overworld map (game state `$0C`) — ported 2026-10-14

`sml2_core::map_sim` is the map's own simulation: `MapSim::tick(input, nodes, walk)`
is one frame of `$3C000` (bank 15), and it hands back the writes the ROM makes
outside the map's RAM block as `MapEntry` — `sCurLevel`, `sLevelProgressFlags`,
`sGameState`, the music/sfx requests, the save write and the node transition. It
is not wired into the replay harness: no capture ever runs state `$0C`, so the
package is unit-tested (14 tests) against the ROM's own bytes.

**The walk graph.** `map/walk.bin` is the 128-node pointer table at flat `$6163A`
plus every stream its pointers reach, byte-identical to the ROM. The base is
`ld hl, $563A` with bank 24 banked in (`$563A` is a bank-24 *window* address, so
flat `$6163A`; main.asm:18936) and the disassembly's `UnknownData_0x61A40` label
is what the pointers point *into*, 1024 bytes later — which is what fixes the
table at 128 entries rather than the 130 the other node tables carry: node
`$80`/`$81` read past it into the same stream data. 116 of the 128 nodes carry a
path, and every stream ends on a node below `$80`. A stream is one-byte moves
(low nibble 1/2/4/8 = R/L/U/D, with combinations; bit 4 halves the step budget),
`$00`, then the node the walk arrives at.

**The cursor.** One pixel per *move frame*, and the stream decides which frames
those are: `$3C7EC` rotates `sMapCursorAnim` ($A878) right and skips a frame when
the rotated-out bit was set. That byte is part of the 64-byte save image, not a
timer, so the ROM's own value depends on the last transition that wrote it —
`$3E3BA` (the walk-out) writes `$92`, which gives **five move frames out of every
eight**; the hub loader and `$3EA9A` write `0`, which moves *every* frame. The
step timer is 9 frames for the first step and 8 after (4/5 when the move byte's
bit 4 is set), and the walk-start frame is itself a move frame. Pinning one real
stream (node 1 right: R,R,D,D,D,R×6 → arrival 19) with the `$92` cadence takes
142 frames and moves the cursor from (8,8) to (72,32).

The cursor's bounds are the ROM's own comparisons and are **not** symmetric:
Right and Down stop exactly at `$94`/`$8A` (`cp`/`adc`), while Left and Up
decrement whenever the coordinate is still `>= $08` (`cp $08`/`ccf`/`sbc 0`), so
a cursor at `$08` stepping Left or Up lands on `$07` and only then stops. The
pose blocks are 4 wide (L 0-3, R 4-7, U 8-11, D 12-15) and the idle pose uses
Down's block — the ROM's own `$3C8F4` computes 12-15 from `(frame & $18) >> 3`.

**The gate and the entry.** `$3E1D7` (now a free function, `map::map_enterable`)
answers the ROM's 0 for "this node's own action runs": screen != 0, `$4218[node]`
!= `$FE`, the node's `$6591` key mask ANDs the *held* direction byte (`$FF80` is
held, `$FF81` pressed — the gate reads held, the walk direction reads pressed),
and the stage is not yet cleared. The hub short-circuits to "walk" before looking
at anything. `$3C169` is the A/START entry: it is swallowed while a walk is in
progress, needs `hKeysPressed & $09`, and then enters `$4218[node]` — passing
`$FF` through, rejecting only `$FE` — copying the node's progress byte into
`sLevelProgressFlags` and moving the state straight to `$16` (the game's own
machine runs the fade and the load). A cleared level re-enters unless its
progress slot is one of the six castle slots (`$05/$09/$0E/$17/$1D/$11`), and a
node with no level arms `$3C201`'s `sMapSpecialNodeLatch` instead.

**Also ported:** the zone screens' door cluster ($3C6B0-$3C7A3, eleven nodes
that put the cursor back on the hub at another node and move the state to `$0C`),
the `$3C4E7` secret-route arm (nodes `$0C`/`$1E`, A with nothing else held, which
walks a *hardcoded* stream at `$5BF6` to node `$0D`), the hub's B-button
scroll-only latch, and `$3C13E`'s frame counter / 40-frame tick / `$FF04`-seeded
latches.

**Not modelled, and reported rather than skipped:** the A/START node actions
behind `$3C60B` (the castle animation `$3D0D7`, the zone cutscenes), the node
flag helpers `$3C2xx`, the hub's own cursor effects `$3CA31`-`$3CAB1`, and the
whole screen layer — the camera latch `$3ED4C`, tiles, sprites, the hub's
18-entry decoration list `$3CFC8`, the window raster and the cursor's OAM.
`MapEntry::unported` carries a stable id for each so a caller can print a `note:`
line instead of silently doing nothing. The blocking screen transition `$3E2F7`
is art, but its *effects* (new node, hub screen, move byte, state `$0C`) are
modelled.

`MapSim` owns the map's live `$A840` block, including `sLevelProgress`; that
array is also the save record's first 64 bytes, so `MapSim::from_player` /
`to_player` bridge it to `PlayerSim`'s image (which the `$14` hand-off and
`save_record` need) rather than duplicating it silently.

## The map drives itself: the node chain, the node actions and the node records — 2026-10-14

The second map package ports what the map does *without* the player pressing
anything, which turns out to be most of how it moves between screens.

**The synthetic-input chain** (`$3C38C`, the forty `cp $XX / call z` comparisons
at main.asm:18756-18877). Each one **overwrites `hKeysPressed`** with a single key
— `$3C29A` Right, `$3C2A0` Left, `$3C2A6` Down, `$3C2AC` Up, `$3C2B2` Start — and
the same frame's `$3C52B` then turns that press into an auto-walk along the node's
walk-graph entry. Three of the forty *retarget the node* instead: `$3C2B8` (A →
node `$36` while Macro path 2 bit 7 is set, else B → node `$37` on the Macro gate),
`$3C2E7` (Select → `$39` on Macro path 0 bit 0) and `$3C2FE` (A → `$6C`). The port
walks the chain as a table in ROM order and re-reads `sMapNode` at every step,
because a retarget changes what the later comparisons match.

`$3C32D` (every node `$1E` up) presses the node's own `$6591` mask **once per
screen**, with `sMarioDead` as the one-shot latch; a cleared stage never
auto-walks. `$3C316`: node 0 with level 1 cleared hands the cursor to node `$14`,
whose Up class then walks it off. Two consequences the port now shows and the ROM
really has: the synthetic press *replaces* the player's, so on the frame an arrival
walk fires an A press cannot also enter the level (only the next frame can); and
nodes `$0C`/`$1E` are Start-class, so the secret-route arm is automatic there.

**`$3C60B`, the A/START node actions**, are a table now. Nine zone doors ($02/$19 →
Tree screen 2, $09/$1C → Pumpkin-A 1, $06/$1B → Macro 3, $12 → Space 4, $67/$0D →
Turtle 5, $0B/$1D → Mario 6, $11 → Wario's Castle 7, $15 → screen 8) each write the cursor,
the node, `sMarioDead = $FF`, the music request and the `$3E3BA` walk-out direction.
Four coin-operated doors (`$71`/`$70`/`$6F`/`$6E` → bonus screens 9/10/11/12) run
`$3F38F`'s 16-bit **BCD** price check ($0999/$0200/$0050/$0030) and are the only
doors that can be *refused* — a refusal clicks sfx 19 and drops into `$3C7D9`'s
idle path with the press intact. The zone loaders clear `hKeysPressed`
(`xor a / ldh [$FF81]`, verified at `$3D0FF`) so an open door cannot also enter a
level on the same frame; the zone→hub doors do *not* clear it, so pressing A on the
untested door nodes (node `$30` → node `$7A`, which *has* a level) can open a door
and reach a level in one frame.

**`$3EEA4` and the node records.** `map/node_records.bin` is the 1024-byte table at
`$61EE6` (byte-identical to the ROM; the extractor asserts that bytes 6-7 are `$00`
in all 128 records and non-zero from node `$80`, which is the table's own
boundary). A record is X hi/X lo, Y hi/Y lo — the map camera *staging*
(`$FFBA:B9`/`$FFB8:B7`) — then `sMapCursorX/Y`. `MapSim::load_node` seeds them where
the ROM does: on a *screen* change (`$3C7CA` on a zone door, `$3C518` when B is
released), never on a walk arrival.

**The hub pan** (`$3CA31`/`$3CA37`/`$3CA5D`/`$3CA63`/`$3CA84`/`$3CA8A`/`$3CAAB`/
`$3CAB1`) is mechanical, not decoration: the position gates fire when the cursor
reaches `$4C` (Right), `$4A` (Left), `$4B` (Up) or `$4D` (Down), the camera takes
the pixel, and — unless the B-held latch is set — the cursor is pushed back the
other way, which is what holds it near the screen centre through a walk. Holding B
on the hub calls the same bodies with no gate, so the camera pans under a still
cursor.

Verified: 12 more tests (171 total), the door table fail-checked by changing a
destination and watching it go red; the replay gate green. Filed
`map_handoff_3EF2B` for the one ROM disagreement found — `$3C32D` alone indexes
`$3C364` before reading `sLevelProgress`, where the writer (`$3EEF0`) and the gate
(`$3E1D7`) both use the `$650E` byte directly; the port follows the two that agree
and the item records why.

## The view is not clamped to the level — 2026-10-14

Reported as "when the camera is manually panned, mario floats in the air". The pan
was innocent: `render_view` clamped the view origin into the level's own bounds
(`clamp_axis(cam_y - vh/2, vh, world_h)`), which the engine never does. The camera
stays where it is and a 256-px BG tilemap window (32x32 tiles) slides under it, so a
camera near an edge lets the view overhang the level and the window's rows come back
around.

Level 5's start camera sits at the bottom of its 768-px world — the header says
`(46, 708)` with the camera at `(80, 704)`, and the console agrees (PyBoy through
`captures/probe_level_select.py`: node 32 -> `sCurLevel` 5, first state-4 frame
`mario=(46,708) cam=(80,704)`) — so the view's top row is 632, past `768 - 144 = 624`.
The clamp pinned it to 624 and drew every tile 8 px low, while Mario's OAM kept using
the raw camera: he stood in mid-air above the block row on every frame, pan or no pan.
That is why it showed "from the start" on level 5 (its camera starts at the level's
bottom) and on other levels only once a pan pushed the camera near an edge.

Fixed by leaving the view origin unclamped and mapping each screen row/column through
the window: `window_origin` = the window's base (clamped into the level, because the
window has to hold real level rows), `window_coord` = identity inside the level, the
window's rows again outside it. A 256-px window is a whole number of 16-px metatiles
and of 8-px tiles, so the wrap keeps both grids and the fine scroll; the overhang
lands under the status bar, which is what hides it on the console too.

Pinned by two tests in `crates/sml2-runtime/src/main.rs`, each asserting a row the
console draws: level 5 at `cam (80, 704)` puts its block row's top on row 118 (the old
clamp gave 126 on the port and 118 on the console), level 1 at `cam (80, 448)` on row
120 (unchanged — the clamp never fired mid-level, which is why the gate and 171 unit
tests never saw this). Whole-frame diffs against PyBoy at those cameras: 273 px and
268 px of 23040, all of it animated tiles, the sprite's frame and the status-bar
digits.

## The space-physics byte is seeded, and `$0FF2` is ported — 2026-10-14

`LevelPropertiesSpace` (`$1F91`, indexed by `sCurLevel`) reaches RAM only through the
level-init state handler (`$04DD`, `home.asm:661`): `ld a,[hl] / ld [$A287],a`. The port
extracted the byte into every manifest (`properties.space_physics`) and seeded the other
two property bytes (`sMapScrollLimit`, `sAutoScroll`) in both frame loops — but never this
one. Same defect class as the missing edge map and the clamped view: only three levels are
nonzero (index 18 → `$08`, 19 → `$01`, 28 → `$08`; levels 19, 20, 29) and level 1 is `$00`
in all three, so no capture could see it.

The byte is a **bitfield**, never a bool: bit 0 = the level-20 hold-A float, bit 1 = inside
a moon zone (runtime only, written by `$1BFF`), bit 2 = the moon item is held (`$34E2`, which
the port already sets), bit 3 = the timed float of the two `$08` levels.

Both frame loops now call the ONE construction `PlayerSim::seed_moon_physics(byte)`, fed from
`manifest.properties.space_physics`. The replay path DERIVES it — it is not taken from the
record; traj19's `moon` field is checked against the derivation instead (it records `$01` on
all 1309 frames of level 20, the console's own `sMoonPhysics`).

`$0FF2` (`$0F71`'s first player call, ahead of `$1BFF`, so its bit-1 test reads the zone
state the previous frame left) is ported whole as `PlayerSim::moon_jump_extension`:

| arm | test | effect |
|---|---|---|
| head | `sGoalBell != 0` → out; `moon == 0` → out | a plain level is a no-op |
| bit 0 (`$106F`) | airborne + A **held** | `vvel += 4` capped `$48`; sfx 18 on the press edge |
| bit 1 (`$1055`) | A **pressed** | `vvel = 52`, `sMoonJumpPoseTimer = 8`, sfx 3 (the 8 is one-shot) |
| bit 3 (`$1029`) | `sMoonFloatTimer` armed to 50 on the ground, 0 when A is released | `vvel += 2` capped `$4A`, one budget frame spent per float frame |
| — (`$1008`) | bit 2 alone: airborne + A held | `vvel += 2` capped `$4A`, sfx 18 on the press, no timer |

Two new sim bytes: `moon_float_timer` (`$A2B2`) and `moon_jump_pose_timer` (`$A285`). Every
arm returns while grounded (`sMarioOnGround`), except bit 1's.

Verified: 7 new `player_sim` tests (the seed copies the raw byte; the float runs inside
`step()` and the default byte kills it; one per arm, each with its cap/edge/one-shot case)
plus a runtime test asserting the three known levels' manifest bytes and the seeded result.
Every one was shown to fail by reverting the line under test. The replay gate is green with
traj4_ff0's documented numbers unchanged.

Not in this package (still open, filed as `moon_physics_remaining_arms`): `$1BFF`'s bit-1
zone machinery, `$14FA`'s `sFlutterTimer` arm (the port has no such field), `$1541`'s
`$154E` moon arm, `$2E7C` (bit 2 must be cleared and grant 128 frames of invincibility
instead of today's note-and-survive), the bank-1 pose/draw arms (`$5354`/`$53CF`/`$54FB`)
and `$181C`'s missing `sGoalBell → normal table` guard.

**The gate could not see a mid-table regression.** `captures/sweep.sh` tested the mismatches
line with `grep -q "=0$"` — the LAST field only, which is `coinshi=0` for most captures. A
check that regressed anywhere else printed no `REGRESSION` line. It now fails on ANY nonzero
field (the documented traj4_ff0 exception is still printed verbatim). Under the stricter test
every other capture in the suite is all-zero, so nothing was hiding behind the old test — and
the new `moon` check was proven to go red (`REGRESSION traj19 nonzero: moon=1308`) with the
seed removed.

## The level→map hand-off is reconciled with `$3EF2B` — 2026-10-14

`PlayerSim::map_handoff` was written before `$3EF2B` was read. Reading it (main.asm:24126)
found three defects and one non-defect, all now fixed or settled.

**The bell branch RETURNS.** `$3EF2B`'s first test is `sGoalBell`: a ringing bell calls
`$3F4B1` (or `$3F5A4` when `sMapNode` bit 0 is set — the castle screens) and leaves at
`$3EF4F` with `ld a,12 / ldh [$FF9B],a / ei / ret`. There is no node rewrite, no `$3EEF0`,
no tail and no `$2934` save on that path. The port fell through into the map return, and the
test that covered it passed `saves == 0` for node `$48` **only because the `$1E`-and-up tail
was unmodelled** — a green test hiding a wrong branch, which is why the branch now has its
own test asserting `saves == 0` and `map_node == 0` for the right reason.

**The tail is a band tree, not a two-arm match.** `$3F003` splits on `sMapNode >= $1E`
FIRST: at or above it `$3F04D`-`$3F14C` selects a zone screen per band (and, at four nodes,
rewrites the node and sets a gate byte), below it only `$05`/`$1A` (→ the castle node `$3F`
+ screen 4) and node 0 (→ `sLevelProgress[0] bit 7` + node `$14`) have a branch. Every one of
those paths ends in exactly ONE `call $2934`; the port saved only while the node was still
below `$1E` at that point, so it dropped the save for every zone node and for the castle
move. Bands: `$1E-$27`→screen 2 (`$3D0FF`), `$28-$31`→1 (`$3D208`), `$32-$3B`→3 (`$3D371`),
`$3C-$45`→4 (`$3D49F`), `$46-$4F`→5 (`$3D61F`), `$50-$59`→6 (`$3D79A`), `$5A-$63`→7
(`$3D6EB`, with `$5B`→`$5C`), `$64-$6D` (`$66`→`$68` + `set 7` on `sMapGateMario` `$A86D`;
`$6A`/`$6B`/`$6C` → screen 3), `$6E-$77`→8 (`$3D880`), `$78`→`$2E` + `set 7` on
`sMapGatePumpkinB` `$A86E` → screen 1, `$7A`→`$31` + `set 7` on `sMapGatePumpkinC` `$A86F` →
screen 1, and `$3F142` for everything else (no screen, the tail save). The three gate writes
and the node rewrites need a LIVE Mario; the screen selection and `$5B`'s rewrite do not.

**The screen loaders are ported (`$3D0FF`/`$3D208`/`$3D371`/`$3D49F`/`$3D61F`/`$3D79A`/`$3D6EB`/`$3D880`).**
`MapSim::load_screen` and `PlayerSim::map_handoff` implement the zone screen selection bands
and screen loader routines. When entering a zone screen, scroll is reset to (0, 0), walking
and scroll-only latches are cleared, move animation ($92) is armed, and zone-specific node
retargets and path-unlock bits (Tree secret node 37 + path 1, Tree node $26->39, Pumpkin
secret path 1/2, Macro secret node 107 + path 0, Macro node $6A gate/retarget, Space secret
node 67 + path 0, Space node $44->69, Turtle secret path 1) are executed alongside the
loader-requested music track.

**The `$650E`/`$3C364` question is settled: the byte is the `sLevelProgress` index.** Three
sites read it and two agree: `$3EEF0` (the writer, main.asm:24089) and `$3E1D7` (the entry
gate, 22451) both `ld hl,$650E / add hl,bc` and then use the byte *directly* as the
`sLevelProgress` offset — and both also use it as the `$6591` key-mask index, which is what
makes it one slot space (the port's `MapNodes::progress_slot`). `$3EF2B`'s own node-action
dispatch compares the same byte against `$05`/`$09`/`$0E`/`$11`/`$17`/`$1D`. The only site
that inserts the `$3C364` table is `$3C32D` — the map's auto-walk, whose job is to fake the
node's own direction press (`ldh [$FF81],a`) — and `$3C364` is non-monotonic
(`$3C364[$13] = $10` while `$3C364[$12] = $13`), so it is not a re-indexing of the same
space. `$304F1` is a *per-level* table (indexed by `sCurLevel`) and disagrees with `$3C364`
on 18 of the 25 level nodes, so it is not the same mapping either. The port keeps the direct
convention, and a test now couples the two consumers: a level cleared through
`map_handoff` is refused by `map_can_enter` on the same byte, so switching either side to the
other convention fails instead of silently shifting every flag by one.

## Level 3's three handler classes, and the bump arms' verification — 2026-10-14

**`level03_handler_classes` (delegated writer, `crates/sml2-core/src/objects.rs`).** Level index 2's
spawn stream reaches eight handler classes, three of which had no body. All three are in now:
`0x59BC5` (`handler_groundwalker88`, anim 88 — the ground walker: `$7585` bob head, an inline
`rst $28` over `$5BD1`/`$5D05`/`$5D41`, state 0 writing `B_FLASH = $10` + type `$4E` + facing Mario,
states 1/2 probing `$3C1C`/`$3BB8` and flipping through `$5D2D`, then the two-phase counter = 16 idle
calls and 48 one-pixel steps); `0x594EB` (`handler_fallwalker52`, anim 68 — literally the ported
`$582D1` body with `$54F4`'s type `$52` instead of `$582D9`'s `$56`, so `handler_fallwalker` became a
wrapper over one `fallwalker_body(ty)`); and `0x58DD1` (`handler_carrier46`, anim 46 — the object that
carries Mario: state 0 sets type `$20` and state 2, waits on `B_FLAGS` bit 3, then falls with the
`$7451` picker, adds `b` into Mario's 16-bit Y through the `$3CDF` carry and removes itself with
`$72D7` on the ground). Verified against the ROM bytes rather than the report: `$594EB` is
`CD 85 75 / F0 DF / A7 / C2 F5 42 / 3E 52` (the `$7585` head, the shared `$42F5` dispatch, the `$52`
immediate) where `$582D1` has `20 1C` (a `jr`) and `3E 56`; `$59BC5`'s head is
`CD 85 75 / F0 DF / EF / D1 5B 05 5D 41 5D` with `3E 10 E0 DE` for state 0, and `$59D05`/`$59D41` are
the `$3C1C`/`$3BB8` wall probes; `$58DD1` is `F0 DF / A7 / 20 09 / 3E 20 / E0 E2 / 3E 02 / E0`.
Ten new tests, and one of the child's eleven fail-checks was reproduced here independently
(`B_TYPE = 0x20 → 0x21` fails `carrier46_state0_sets_type20_and_enters_the_wait` with the reported
`left: (33, 2) right: (32, 2)`).

**`bonk_single_slot_draw` + `bonk_big_mario_types_1_5` (four new tests, no code change).** Both
items' gates were already unit tests; the code was there and the verification was not. The tests
carry the ROM bytes themselves — `$5A4A` (40), `$5AE0` (25), `$5BB4` (8) as hex strings, compared
against the port's arrays — so a hand-edit cannot drift, and they pin: the `$19B0` big arm
(`$00` → sfx2 + a type-1 slot, cell 96 at timer 2 and again at `$28`, the `$5A07` pair 101 then 102
at `slotX - 16 ∓ the timer`, 102 surviving in `$FFC6`); the size split for `$00`/`$03`
(types 2/1/6/5 with cells 24→0, 96, 24→3, 127); and the single-slot case traj15's two-slot `$01`
bump cannot show — a lone type-3 slot leaving 66 in `$FFC6` (`$5B8A`'s `3E 42 E0 C6`, `$5BB4[timer]`,
no timer term on X), with a later type-4 coin slot drawing after it and taking the frame. One
correction came out of writing them: type 6's used tile is 3 (`$5BC8`: `3E 03 / call $5CF1`), not 7.

Whole tree after both: 197 core + 3 runtime tests, `cargo build --release` clean, `bash
captures/sweep.sh` → `gate done` with traj4_ff0 byte-identical and no `REGRESSION`, and no `note:`.

## The projectile family: spawn, arc, break — 2026-10-14

**`fireball_paths` + `fireball_type06_break` (`crates/sml2-core/src/player_sim.rs`,
`crates/sml2-core/src/objects.rs`).** The queue had recorded `$2FED` as the fireball *spawn*;
it is the object pass. The family is four consecutive tail calls of `$0F71` — `$0FAB` is
`CD 9F 31 / CD 51 34 / CD C1 32 / CD 1C 33 / CD 4F 32 / C9`, i.e. `$319F` → `$3451` → `$32C1`
(spawn) → `$331C` (move) → `$324F` (pass) — so a ball created on the press frame also moves and
collides on that frame, and an item consumed on frame N (`$3451`) satisfies the spawn's tier gate
on frame N.

Ported, mirroring the branch structure: **`fire_spawn`** (`$32C1` — the B new-press edge,
`sCurPowerup == 3`, no ducking, no goal bell, first of the two `$A880`/`$A890` slots free, and a
press with both live eaten silently; the ball at `$FFC0` + 28 / `$FFC2` + 16 with carry, facing
`sMarioDirection` — or `($FF97 & 2) - 1` mid spin jump — phase 0, throw timer 8, sfx0 11);
**`fire_move`** (`$331C`/`$332E` — ±3 px on X from the facing byte's `sla`, Y by `$3439[phase]`,
rising at `phase >= $0C`; probe 1 at `$1EFA`'s `(Y-16, X-8)` is terminal, `$06` breaks and any
other tile below `$38` dies; probe 2 pops a *descending* ball to phase `$18` — `$33F1` then
decrements it — on any tile below `$48` that is not `$06`; write-back is `+1..+4` and `+6`,
never `+5`); **`fire_break_or_die`** (`$3415`/`$341C`/`$3430` — sfx2 1, sfx0 2, the *hardcoded*
type-1 bump at the ball's own cell, `sSpinJumpBounce = $FF`, then `+0` cleared, and no write-back
of the move on either kill path); **`fire_pass`** (`$324F`/`$3261` — the pair is
`(X - camX + 80, Y - camY + 70)`, a `$C0`-high-nibble cull kills silently, otherwise the pair is
staged for the object pass); and **`ObjectSim::fire_hit`** (`$2FED`/`$3007`, the object half —
gate `B_FLAGS == 0` with bit 3 *included*, unlike `$2D7D`'s `and $F7`; type 0 and bit-7 types are
transparent; the ±6-expanded `B_HB_*` box is tested byte-wise against the staged pair; a bit-2
type gets `B_FLAGS` bit 1, which `$85AA`'s dispatch turns into `defeat()`, and every other type
takes the sfx arm; both leave `sObjectHitResult` non-zero, which is what kills the ball).

Two ROM corrections (fireball defeat path): the defeat
test is `bit 2, a` — a type needs `$04` set, so the spec's section-7 example (type `$20`) is
wrong and takes the sfx arm instead; and `$3261` computes the pair as
`ldh a,[$FFCA] / ld b,a / ld a,[sFireballX] / sub b / add 80`, exactly the `+80`/`+70` its prose
gives (hex `$50`/`$46`). The phase table is byte-verified as the 24 bytes in its appendix.

Split at the object seam, in the ROM's order (`$0F71` runs the player's `$2F72`, then the ball's,
then the pass): the frame loops run `ObjectSim::below_bonk` for `sim.below_bonk` (the ceiling
bonk) and for `sim.fire_below_bonk` (a break — only the player's arm reaches `resolve_bump_coin`,
since only the `$19D2` bump owes the coin), then `fire_hit` per staged `sim.fire_scan` pair and
`sim.resolve_fire_hit`. The ball's own sprite (`$2CF4`, mapping `$B0`/`$B4` + `($FF97 & 6) >> 1`)
and Mario's throw pose are draw-side and unmodelled, so a live ball latches
`sim.fire_draw_unported` and the replay summary prints it as a `note:` — `$A880` is not among the
fields a capture compares, so without that line a capture that fires would replay green.

Sixteen new unit tests (thirteen `player_sim`, three `objects`), each constructing the slot/state in
memory and asserting the resulting bytes, and eleven one-line mutations of the ported logic were
each shown to fail a named one of them before being restored. Whole tree: 213 core + 3 runtime
tests, `cargo test` green, and `bash captures/sweep.sh` → `gate done` with traj4_ff0
byte-identical, no `REGRESSION` and no `note:`.

## Play-test fixes: the probe staging, the shell, and the defeat animation — 2026-10-14

Three reports from a `--play` session, and the ROM bytes behind each.

**1. A spinning Mario's landing bonk broke the wrong cell ("spin jumping on breakable
blocks doesn't break them", plus a stray blank patch above him).** `$FFB7`/`$FFB9` is *one*
staging pair: `$1DFE` (ceiling) writes it, and so does `$1E3B` (floor — `$1E48` stages
`Y = $FFC0 + 44`, `X = $FFC2 + 12`, then re-stages `X + 20` for its second read), and `$5960`'s
alloc stores the bump slot's cell *from that pair*. The port modelled only the ceiling half
(`ceiling_probe`) and allocated from it, so a spinning landing (`$1327` → `$1993`) allocated the
bump at a **stale ceiling point**: the brick under him never became `$60` and the shard sprites
drew at the stale cell. The field is now `probe_point` (the shared staging), the floor probe
stages it before each read and stages nothing when the `(y-4) & $F` gate bails (the ROM's
`cp $05 / jr nc`), and the alloc reads it; ceiling bonks still get the ceiling point because
`$1DFE` runs later in the same frame.

**2. Fireballs were invisible.** The spawn/move/break half had landed; the *sprite* had not.
Now `fire_pass` stages `$3261`'s `$2CF4` mapping (`$B0` left / `$B4` right + `($FF97 & 6) >> 1`,
the rDiv flicker) anchored so `draw_cells`' `+96/+98` lands on the engine's staged
`X - camX + 80` / `Y - camY + 70` pair, and `frame_index` models `$548A`'s throw pose
(`+112`, plus a further 32 while `sMarioThrowTimer` counts down from the 8 the spawn sets).
The OAM-order question (a fresh ball *inside* Mario for a frame or two) is left open in the
code comment — it can only show while they overlap.

**3. "Enemies don't do death animations" / "a fireball hit enemy just stops" — one missing
handler.** `$85FC` (the defeat, reached from the stomp's `$85F0` and the fireball's bit-1 flag)
clears anim/state/flags/type and sets `sObjectDone`, and the **anim-0 handler `$581B1` is the
death animation**: state 0 hops the enemy 20 px up (`$3D2D`) and flips it upside down
(`+13` bit 6), state 1 rises on the `$74AB` 8-entry curve at 2 b a frame, state 2 falls on the
`$5B451` curve (also 2 b) and removes the slot through `$5B2D7` with remove-mode 2 once the
staged screen Y passes `$A0`. Without it the defeated slot kept its stale sprite, so the enemy
froze. Also ported with it: `$C014`'s draw gate — the bank-3 draw returns before drawing while
`sObjectDone` is set — which the renderer now mirrors per slot (`ObjectSim::slot_done`), so the
defeat frame itself draws nothing and the flip-and-fall frames draw again.

**4. The shell: carry, kick and KO (`$8611`, anims 4 and 3).** The dispatch's type-`$06` arm is
now the ROM's: B held → the `sActionLatch` one-shot, `anim += 2` (2 → 4, carried) and `$863C`
(whose `$8240` clears `+13..+29`, the type byte with them); B not held → `$8633` (`anim += 1`)
plus the `$864A` placement tail. `handler_carried` (`$58444`) is `$5B4C1`'s follow (Mario's X + 8,
Y + 5/14/20 by size and ducking), `$5B697`'s KO overlap, `$3B68`'s ceiling check, the pipe-travel
removal and the B-release drop to anim 3 + placement; `handler_kicked` (`$5839B`) is the four-state
walker (3 px steps, wall turns, the `$5B451` ledge falls) with `$5B607`'s sprite wobble — which is
`$5B545`'s routine with the same `+26` phase and table, so `spin_step` serves both. The KO scan
skips type 0, `$5F` (itself/another shell), bit 7 poses and bit 5 types, and sets the target's
`+17` bit 1 — the flag the dispatch turns into `defeat()`.

Corrections to what was in the tree before this pass: the carried follow is *Mario's position*
plus an offset, and the `$5B607` wobble moves the **sprite**, not the position (the earlier
`update_carried` added it to X and moved the object outside the animation system, which is why a
thrown shell hung where it was); a released shell only becomes the live type `$5F` from the
anim-3 state-0 arm (a shell kicked by a *stomp*) or from a wall placement, which is what the ROM
does; and `$3D2D`/`$3D3D` are Y −= b / Y += b.

Fourteen new unit tests in this pass (the spin-landing cell, the floor-probe staging, the
throw pose, the fireball's staged sprite, the pickup/carry/release/kick chain, the four walker
states, the KO window and its gates, the death animation's three states, the `sObjectDone` draw
gate, the extraction's anim-0 handler), each with at least one one-line mutation shown to fail it.
Whole tree: 234 core + 3 runtime tests, release build clean, `bash captures/sweep.sh` → `gate done`
with traj4_ff0 byte-identical, no `REGRESSION` and no `note:`.

## The boss chain: `$0EB0` -> `$3C94` -> bank 2 `$8D9C`, and `$0652`'s other pass — 2026-10-14

The boss fight is not an object in the level's object list; it is a *second object pass*
that replaces `$8000` for the duration. Four ROM pieces, all now ported at the frame
level:

1. **`$0EB0`'s scene branch** (`home.asm`): a screen transition that ends with a
   non-zero low nibble in `sSceneState` sets `sSceneState = $F0`, LCDC `$C3`,
   `sMusicRequest = sLevelSong = 32` (or `sBossFightPhase + 33` on level `$18`, the
   Wario castle), and calls `$3C94`. Its `sBossDefeated` arm is a **separate `if`**
   (`$0EE4`) and runs either way. The port splits the branch: `finish_transition`
   arms `scene_load_pending` and the frame loop finishes it through
   `begin_scene_load(level_id)`, because the tail needs `sCurLevel` (the level) and
   `sMusicRequest` (the player sim), neither of which the camera owns.
2. **`$3C94` -> bank 2 `$8D9C`**, four calls: bank 3 `$EC40` (clear `$AF00-$AFFF`),
   bank 3 `$EC31` (clear `$AD00-$AEFF`), `$8DFD` (build the def), `$8E6C` (stage the
   def images). `$8DFD` keys the def by level: `$05`->0, `$09`->1, `$0D`->2,
   `$10`->3, `$13`->4, `$17`->5, else->6, then adds `sBossFightPhase`. `$8E6C` copies
   the main 18-byte image to `sObject0Def` (`$AD05`, slot 0's byte 5) and `count`
   sub-images to `sObject1Def` (`$AD25`) with a **`+$10` stride**.
3. **`$0652`** picks the frame's object pass: `sSceneState & $F0 == 0` -> bank 2
   `$8000`; otherwise `$3C9F` -> bank 2 **`$8DA9`**, the boss driver.
4. **`$8DA9`** walks `sObjectDefCount + 1` slots from `$AD00` at a **`$20` stride** —
   *not* the `+$10` the images use. That mismatch is real and load-bearing: the first
   sub-image lands on slot 1's byte 5 (`$AD25`), the second starts at `$AD35` (slot 1's
   byte `$15`), so its tail lands in slot 2's bytes 0-6 and slot 2's anim byte
   (`$AD45`) stays 0 — the ROM leaves that page **inert**, and a faithful port must not
   "fix" it. Each walked slot is staged into `$FFD0` (`$81A7`, low byte masked `$F0`),
   and only when the staged anim byte (`$FFD5`) is non-zero does it dispatch the anim
   (`$92DF`), clear `$FFE7` (= `B_FLAGS`), fetch the `$505B` extent record (`$9048`,
   the *scene-state* table — the level tables are `$4737`/`$4935`) and copy the slot
   back (`$81BD`). `$FFF0` is bumped **unconditionally**, unlike `$8000`'s
   phase-toggle increment.

Landed with this pass: `scene::boss_def_index` (the ladder, tested per level),
`ObjectSim::{boss_def_index, boss_def_count, boss_hit_latch, boss_5290}`,
`spawn_boss` recording index/count and reporting a def index beyond 8 (`$904E` would
read past the 9-entry pointer tables), `ObjectSim::boss_engine`, `CameraSim::boss_mode`
(`$0652`'s test, one source of truth for both loops) and `begin_scene_load`; the
`$505B` table the extractor had already stored as `draw_records`/`draw_index` now has
its `draw_extent_for` accessor and is what `refresh_hitbox(.., boss = true)` reads.
`step` gained the `boss_mode` flag, so a parked frame runs the driver *instead of*
`$8000` — the check that keeps arming `$F0` from parking the port with nothing to
advance the scene.

Verified against the ROM before building on it: the nine defs' 3-byte records, their
`$4E9A`/`$4EAC` pointer tables and all 18-byte images are byte-exact — including
def 7's table-B pointer, which reads *past* the 7-entry table into the records area
(`$5036`) exactly as the ROM does, and def 8's count-0 case where `$8E6C` returns
before touching table B.

The anim dispatch is now the ROM's own: `$92DF` jumps through the inline `rst $28` table
at bank 2 `$92E8`, a *different* handler set from the level profiles' for the same anim id
(anim 16 -> `$A0B9` boss vs `$586F5` level). Extracted as `boss_anim_handlers` (schema
`sml2-objects/4`), length = the leading run of in-window values (29 — the ROM declares
nothing), and the driver reports those handlers as unported rather than silently running
the level's. `run_handler` split into the lookup plus a shared `dispatch_handler` so the
two paths cannot drift.

**Correction (found by a parallel reverse-engineering pass and re-verified here): this
table's entries are BANK-2 addresses, so file = `value + $4000` — the first extraction
added `BANK16_CPU_ADD` (`$54000`), which is right for the level tables `$58040`/`$59C07`
but off by `$50000` here.** `rst $28` jumps inside the bank `$3C94` mapped, which is 2,
and the whole `$8D9C`/`$8DA9` chain runs there. Checks: entry 0 (`$5322`) is a lone `C9`
at `$9322`, the same "inert anim" idiom as the level table's `$58100`; anim 16 (`$60B9`)
is `$A0B9`, a clean `call $5285` / `sObjectDone` / save-state body; with `+ $54000` 13 of
the 29 entries land mid-instruction. The decomp *does* label this family in `main.asm`'s
bank-02 section (`UnknownCall_0x9285` at 3802, `UnknownCall_0x92DF` at 3856), so the
handler port is easier than the earlier note claimed.

Also corrected in the same pass: **the boss path never stages `B_SCRY`/`B_SCRX` from world
coords.** Those bytes are the object's *screen* position: the def image carries them
(`$AD05`'s bytes 4/5 become `B_SCRY`/`B_SCRX` — def 0's image holds `$7C`/`$58`, def 4's
`$30`/`$90`) and the handlers move them with bank-2 `$B264`/`$B26A`/`$B270`/`$B276`
(8-bit, screen-space). `$66000` consumes them as-is. `boss_engine` was deriving them via
the level draw's `$C090` math, which would have clobbered every boss that moved; that
block is gone and a test pins the def's pair surviving a driver frame.

Still open, now measured: `$3A14`/`$66000` is the boss path's *render* pass — it computes
`sSpriteFlipFlags` (`$6087`, done here) and emits OAM entries from the bank-25 table at
file `$66092` (118 `dw`) plus blobs from `$6617F`, so the port needs that table extracted
rather than the `$C090` staging it used to do; the 29 handlers themselves, whose region and
helper library live in `crates/sml2-core/src/objects.rs` (boss handlers); `sBossFightPhase`'s only ROM
writer, the scene script's per-screen handlers (`$3C000`); and `sBossDefeated`, which
`$581E2`'s tail decrements as a counter while the port models it as a bool.

## The bank-2 boss helper library, and the first two handlers — 2026-10-14

With the region measured and the bank mapping fixed, the shared foundation is ported.
`$9285` is the hit pass every boss handler runs first: `B_FLAGS` bit 0 is the stomp hit
(2 HP, latches `sBossHitLatch`), bit 1 the projectile hit (1 HP), no bit means this frame
did not touch the boss. Every hit slot gets `sObjectDone` — which is why a struck boss
disappears for the frame the `$C014`-style gate is set — but only the *primary* slot (the
driver's pointer low byte `$00`, i.e. index 0) books the HP, the sfx and the death, and
`$92BC` treats 0 **or a borrow** as the kill, so a 1-HP boss dies to a 1-damage hit.

The rest of the library is small and self-contained: `$B14E` (fade-idle test), `$B157`/
`$B15B` (the boss's way out — drop the live def count, blank the staging, mark done; not
`remove_object`, which does the level path's record/counter/hitbox work), `$B264`/`$B26A`/
`$B270`/`$B276` (8-bit *screen* movers — the world-coord `mv_x_add`/`mv_y_add` are a
different pair), `$B2E8`-`$B32D` (eight blink entry points that differ only in
`(mask, xor)` over the `$FFF0` gate), and `$B224`, the def-record appender: a child lands
at `$AD05 + $20 * (count + 1)` — *slot* `count + 1`'s byte 5, so the first byte written is
that slot's anim — inherits the parent's `$FFD6-$FFDA`, takes the sprite at byte 6
(`B_SPRITE`) and the attr at byte 8 (`B_FLIP`, skipping the defeat byte), and increments
the live count so the driver walks the new slot.

**That last one corrects an earlier note in this repo**: a boss's live extra parts do *not*
come from `$3A14`/`$66000` — the *handlers* call `$B224`, and `$66000` is only the render
pass (OAM emit from the bank-25 table at `$66092`).

Ported with it: `handler_boss_def2` (`$9606`, boss anim 2 — its whole body is 35 bytes:
return unless the slot's remove-mode is 2, then spawn one anim-4/sprite-9 child and park at
sprite 7) and the dispatch arm for anim 0, whose handler at `$9322` is a bare `ret` — the
same inert-anim idiom as the level table's `$58100`, so it is dispatched silently rather
than reported as a gap. The two bank-2 data tables the handlers index are extracted too:
`$B1A3` (the 38-byte profile all the `$B173`-family pickers share, differing only in their
`c` limit) and `$9595` (the def-0 boss's sweep profile — a 27-byte 0-3 ramp then zeros).

The RAM the helpers touch needed plumbing, done the same way the item/stomp fields already
were: `MarioContact` gained `fade_timer` (input, `$B14E`), and `ObjectSim` gained the
object-written `sfx0/1/2`, `music_request`, `boss_clear_flag` and `fade_timer_set`, which
`apply_object_result` hands to the player sim that owns the live sound and fade bytes.

Six new tests (the hit pass with its borrow-kill and primary-slot rule, the finish/blank,
the 8-bit movers with their wrap, the blink gate, the child appender with its `$10` abort,
the def-2 spawner, and anim 0's silence), each with at least one mutation shown to fail it.
Still open: boss anims 1/3-28, the Mario-screen helpers (`$B296`/`$B2C2`/`$B27C`/`$B2AF`/
`$B2DB`) which need `sMarioScreenX/Y`'s ROM writer ported first (`home.asm:2096`,
`main.asm:446`/`557`), the shared tails `$9435`/`$9443`/`$9725`, and the `$66092` OAM table.

## Regression and fix: the `$0EB0` scene branch parked the camera — 2026-10-14

A play-test reported that sideways (scene) pipes do nothing. It was this session's own
change. `$0EB0`'s branch — taken when a transition ends with `sSceneState & $0F != 0` —
sets `sSceneState = $F0`, starts the boss song and calls `$3C94` (bank 2 `$8D9C`) to stage
a fight. `$F0` is also what `$0B2A` reads to park the camera and `$0652` reads to swap the
frame's object pass for the boss driver.

Taking it is only safe if something later clears that `$F0`. All six writers of
`sSceneState` that do so are in the bank-15 `$3C000` map/screen handlers, which are
**unported** (`map_zone_screen_loaders`) — so the port parked the camera for good
(`CameraSim::step` early-returns while `scene_state & $F0 != 0`), swapped the object pass
to a boss driver with no boss, and the pipe appeared to do nothing. The earlier code had
deliberately kept this a *non*-branch with the reason in its doc comment; this session
removed that guard while porting the boss chain, and its own tests passed because they
tested the branch in isolation, not the consequence.

The branch is now **reported, never taken**: `finish_transition` still arms the pending
flag, and the frame loops consume it through `CameraSim::take_blocked_scene_load` and hand
it to `ObjectSim::note_blocked_scene_load` (which reports `$0EB0` in the same `unported`
list the handlers use, so it is visible in play mode and cannot silently regress). The
scene VM (`$20A4`, which *is* ported) keeps handling transitions exactly as it did before.
The music writes and the `$3C94` hand-off stay in the code path they belong to.

**Correction to the first version of this note.** It blamed the bank-15 `$3C000` map
handlers for being the missing clearers. They are not: the three writers that clear the
park are **bank-2 boss-fight-end states** (`main.asm` 6735/7149/7477, each
`call $B15B` → `sBossFightPhase = 1/2/3` → `sSceneState = 0`), i.e. the boss handlers
themselves. The full `sSceneState` story, measured from its six writers:

- **the level load** (`home.asm:611`) seeds the *high* nibble from the level header's
  layout byte (`sLevelLayoutIndex & $F0`), and the port's extraction shows **no level seeds
  a non-zero value** — so every level starts unparked;
- **the scene VM's `$2x` opcode** (`home.asm:4315`'s `$20EB`) sets `sSceneState = $0F` —
  the *arming* half, and the port already has it (`scene.rs:344`, with a test);
- **`$0EB0`** sets `$F0` — the branch that is reported, not taken;
- **the three boss-fight-end states** clear it back to 0.

So the scene-state slice needed no bank-15 work, and the real dependency behind the reported
branch is the **boss handlers' end states** — the boss porting itself. That is also why
"sideways pipes" could only have been affected in a level whose script arms the arena path:
level 1 seeds `scene_state = 0`, so it was never touched.

## Boss anim 1 (`$93A7`), the shared prologue and the tails — 2026-10-14

The def-0 fight's own handler is in for its entry and intro, on top of the shared pieces
every boss handler uses. The **hit prologue** (`$93AA`-`$93D4`, inlined per handler in the
ROM, one helper here) banks the slot's sprite/state/type into `sObjectSpawnSprite`/
`sSavedFrameState`/`sSavedObjectType` and parks it in state 1 — its hit-stun — loading
`sTimerReload` into `B_T28` when the hit was a stomp. The hit-stun bodies restore from
those bytes, so `ObjectSim` gained `saved_frame_state`/`saved_type`, and
`sObjectSlotPtrLo` became a field the driver sets per slot (`index * $20`) because `$9285`
reads it to decide which slot is *primary*.

The tails: `$9725` (type 0 / state 1), `$9435` (the defeat fall — state 2, `$20` timer,
sprite from the def) and `$9443` (the morph/hand-off — flash, count `B_T28` **down then test
the new value**, clear the state, hand the slot to anim 3, stop the music on the primary
slot only). Anim 1's own body is then: the damage pass, the hit prologue when hit, and the
intro state (`$93D5`) — blink, wait out the boss-clear fade, then `B_TYPE = $45`,
`B_SPRITE = 3`, `B_FSTATE = 3` into the sweep. Its sweep states 1-7 (`$93FE`-`$9543`) are
**reported**, so a def-0 boss wakes up and then holds still rather than silently doing
nothing.

Four new tests (the prologue's bank/latch behaviour, the morph tail's decrement-then-test
and primary-slot music, anim 1's fade gate + wake-up + reported sweep state + hit-save) and
five mutations, all red.

## The def-2 fight (Mario Zone, level `$0D`) — anims 10/11/12 — 2026-10-14

The boss the play-test went looking for, and the same level whose script arms the arena
path (which is why my `$0EB0` change hit its sideways pipe).

The fight is three objects. **anim 12 is a controller with no HP at all** — its hit body
banks the slot and parks it without ever touching `$FFD8` — and it waits for the parts to
clear before spawning one child (anim `$0D`, sprite `$0E`). **anims 10 and 11 are the two
parts**, and they share one hit body (`$9C3F`): a stomp takes 2 HP, a projectile 1, and 0 or
a borrow sends the part to `$5435` (the defeat fall). Their HP lives in `B_STATE` (`$FFD8`,
slot byte 8 — normally the *level* engine's slot state) and is seeded straight from the def
image: **def 2's two sub-objects carry 6 there, its controller 0**, and `$9C72` is the only
writer of that byte anywhere in bank 2 — so the parts are the boss's health and the
controller is never damaged. The remaining shared bodies are `$9CAC` (the hit-stun: flash,
count `B_T28`, then restore the banked sprite/state/type or die), `$9C8B` (the intro's end:
type `$45`, state 3, sprite `$20`), `$9CDF` (the bob — `B_T24` picks the cap-15 ascent or
the cap-21 descent, whose end sets state 4 and `B_FLIP ^= $20`), `$9D3D` (a walk reaching
`$10`/`$A0` flips the state bit, the object, and re-announces the anim id), and the walk
speed `4 - HP/2`, i.e. the parts accelerate as they take damage.

Also extracted: the pickers' profile tables. Each picker is "`c` + a table" over the same
`b = table[B_PHASE]; advance unless B_PHASE == c` body the port already has as
`step_picker`, so the port needed only the data — `$B1A3` (38 bytes, cap 21) and `$B214`
(16, cap 15) — plus two new helper shapes: `$B402` (every 16th frame, `B_FLIP ^= $20`) and
`$B3D7` (the 8-step sprite cycle).

**Why the fight still cannot be reached in play**: `$0EB0` parks the camera for good unless
something clears `sSceneState`'s `$F0`. For def 2 the clearers are not boss handlers at all
— the fight hands the slot to anim 3, and the run ends through game state 29 (`$FF9B = 29`,
the boss-clear/level-end path) whose level reload reseeds `sSceneState` from the header. So
the branch becomes enableable once that ending path is ported; until then it stays reported,
and a def-2 boss can only be exercised through the object pass directly.

Five new tests (the part's HP/death, the stomp's two points and latch, the controller's
no-HP rule and its once-only child spawn, and the bob's two profiles).

## Closing the loop: the boss-clear ending, and the arena entry enabled — 2026-10-14

The fight needed an end before the arena could be entered, because `$0EB0` parks the camera
and only a level reload (which reseeds `sSceneState` from the header) clears it.

The ROM hangs the whole thing on one test, `main.asm:1307`:

```
ld a,[sMarioDead] / ld b,a
ld a,[sBossClearFlag] / ld c,a
ld a,[sGoalBell] / or b / or c / jp nz,$5E12
```

— the in-level pass diverts on *death, boss clear or bell*, into `$5E12`. The port already
models the bell and death arms; the third is now wired: `$9285`'s `$92C4` sets
`sBossClearFlag` (`$A2E3`), `apply_object_result` hands it to the player sim, and
`boss_clear_probe` takes the port's existing level-clear phase (the same one the bell arm
sets: `exit_phase`, a 192-frame fade, the level-clear music). That phase is what ends the
level, and the level reload is what clears the park. `$5E12`'s boss-specific tail — game
states 27/29 -> bank 12 `$45B6`/`$467A`, the castle/statue ending — stays unported and the
comment says so.

With an ending in place, `$0EB0`'s branch is **taken for real** for a def whose handlers are
all ported: `scene::boss_def_portable` (def 2 today — the Mario Zone, anims 10/11/12) gates
it, `CameraSim::begin_scene_load` sets the `$F0` park and returns the song, and the frame
loops run `$3C94`'s spawn (`ObjectSim::spawn_boss`) right after. Every other def still goes
through `take_blocked_scene_load`, which reports `$0EB0` instead of parking — the guard that
keeps an half-ported boss from freezing the camera for good.

So level `$0D`'s sideways pipe now: completes the transition, parks, stages the def-2 fight
and hands the frame to the boss driver; the parts take 6 HP each, the controller spawns its
child, the fight's hp reaching zero arms the clear flag, and the level clear ends the run
(the port then freezes on the unported overworld hand-off, exactly as it does at any other
level end).

Three new tests (the flag's divert and its death precedence, the flag crossing from the
object pass, and the branch taken for def 2 with the park + song, plus `boss_def_portable`'s
gate) and four mutations, all red.



## The boss render pass (`$3A14`), and what an arena actually needs — 2026-10-15

Three separate things stood between a staged def-2 fight and the boss room the console draws,
and only the first was the "unported `$3A14`" the queue had been reporting.

### 1. `$3A14` -> bank 25 `$66000`

The boss path does not draw through the level's `$C000`. `$8DA9`'s per-slot body ends in
`$3A14`, which maps bank 25 and calls `$66000`, a complete OAM emitter:

```
ld a,[sObjectDone] / and a / ret nz      ; a slot that died this frame is not drawn
call $66087                             ; sSpriteFlipFlags = $FFDC ^ $FFDD ^ $FFDE
ld hl,$6092 / ldh a,[$FFDB] / ...       ; sprite byte -> dw table at bank-25 $6092
ld h,161 / ldh_a_n $8D / ld l,a         ; OAM write cursor = $A100 + [$FF8D]
ldh a,[$FFD9] / ld b,a                  ; base Y (`B_SCRY`)
ldh a,[$FFDA] / ld c,a                  ; base X (`B_SCRX`)
```

then per cell `[dy, dx, tile, attr]`, `attr ^ sSpriteFlipFlags`, `dy`/`dx` mirrored as `~d - 7`
under flag bits 6/5, the Y byte dropped when `>= $A0` and *zeroed* when the X byte is `>= $A8`.

Measured facts: **118 entries is not declared anywhere** — `($617E - $6092) / 2 = 118`, where
`$617E` is the first record (`$80`, an empty list) and `$7498` the 118th pointer; the extractor
keeps the record bytes verbatim (terminator and `$88` origin markers included, because that
marker's operand shifts the base Y only while flip bit 6 is set — a runtime value), and all 118
re-derive byte-exact against `baserom.gb` (4974 bytes). The base pair is the object's **OAM**
position, not a world coordinate: the def image seeds `$FFD9`/`$FFDA` (def 2's controller
carries `$40`/`$58`) and the `$B264`-family movers change them. Every slot image the nine defs
install carries a sprite id inside the table, so no retail arena falls off it.

`ObjectSim::boss_render` fills `ObjectSim::boss_oam` (OAM byte space, slot order = the console's
`$FF8D` cursor order); the runtime blits that list *instead of* the level's instances on boss
frames — the level draw's `state == 2` filter must not be used there, because a boss slot's state
byte is its **HP** (the def image seeds 6, and a part at 2 HP would be double-drawn).

### 2. The `$6x` opcode is a graphics *and* room swap (`$25AF`)

Every arena intro script starts with `$6x`, whose own nibble is `sLevelGfxIndex`. `$25AF` then:

```
bank 7  $7800 -> $8E80   0x50 bytes          (index-independent, and dead in effect:
                                              the next copy spans $8B00-$8EFF over it)
bank    $2699[i*4]     -> $9200   0x600 bytes   from bank $268B[i*2]
bank    $2699[i*4 + 2] -> $8B00   0x400 bytes   from bank $268B[i*2+1]  (64 tiles — 0x20 past
                                              the 0x380-byte enemy region, on purpose)
$26B5[i] -> sMetatileMap                     the Level_TileMap_NN (maps 25-28) the index names
sLevelLayoutIndex = 3 ; sLevelBank = 19 ; $0361
```

That last line is the one that looked like a decomp mistake and is not: the ROM **reloads the
level data from bank 19 `$42F0[3]`**, one blob, the same for every arena, so the boss rooms are
all authored as screens inside that single layout and the per-arena look comes from the tileset
swap above it. Measured on hardware: with Mario in level `$0D`'s arena the console's
WRAM `$B000-$DFFF` is byte-identical to the layout of the one level whose header names
(bank 19, layout index 3), and its VRAM `$9200`/`$8B00` match the extracted tileset byte for
byte. The port had been rendering the *level's* cells at the arena's coordinates with the
arena's metatile definitions — which resolve to blank tiles, so a "fixed" arena came out white
with the boss floating on it.

Landed as `sml2-core/src/tileset.rs` (`Tilesets::apply`, the three pastes + the metatile swap +
the OBJ block's shared high half) plus `extracted/scenes/tilesets.json` and
`extracted/scenes/arena_layout.bin` (the extractor cross-checks the latter against the level it
also extracts). Play mode applies it on the frame the VM runs `$6x`, and it swaps the live
layout too, so collision, the object engine and the renderer all see the arena room.

### 3. `$8E6C`'s sub-image stride is +$20, not +$10

The queue carried a "correction" that read `e = ((e + 16) & $F0) + 5` as +$10 and concluded a
count-2 def's second sub-image lands inert. The instruction is right and the conclusion was
wrong: `ld a, e` reads E **after** `$3CAA` already walked it the 18 bytes it copied, so the step
is `(18 + 16)` folded into the low byte — `$25 -> $45 -> $65`, i.e. +$20, filling slots 1 and 2.
The console settles it: in level `$0D`'s arena `$AD05`/`$AD25`/`$AD45` hold anims `0C`/`0B`/`0A`
with `$60 88` and `$60 28` positions — all three parts live, which is why the room has three
houses. The port's old +$10 stride cost the fight one of them.

### 4. The fade timer ticks every in-level frame

With the room and the art right, the fight still did not *start*: the parts sat at sprite `$1E`,
unmoving, forever. `home.asm` ~115 decrements `sFadeTimer` in the frame's VBlank tail on **every**
frame — `sub 1` on the low byte, `sbc 0` on the high, clamped to zero — and the port only moved
the timer from the death and exit fades. So the arena intro's 352-frame fade (`$20EB`'s
`sFadeTimerHigh = 1` / `sFadeTimerLow = 96`) never expired, and both `$9C8B` (the parts' intro
end: type `$45`, state 3, sprite `$20`) and `$B14E` (the "fade is idle" gate in front of the
whole sequence) stayed shut. Console, level `$0D`'s arena: slot 2 picks up sprite `$20` and
starts moving around frame 440; the port does the same once the decrement is in the in-level
pass. The same gate is what lets the *end* of the fight run at all — `$92C4`'s 640-frame
boss-clear fade counts down through it, and every handler that waits on `$B14E` waits on it too.

### Verification

- The port's arena frame against the console's (PyBoy, map route into level 13, poke Mario and
  the camera to the pipe's approach, hold Right 220 / 700 frames): **96.5% of pixels identical**
  with 0 differing on the background and floor rows; the remaining rows differ only where the
  moving parts and Mario stand at slightly different fight frames.
- Slot staging at the park is byte-identical to the console's `$AD00-$AD5F` dump
  (`anim/state/sprite/scry/scrx` = `0C/00/1E/40/58`, `0B/06/1E/60/88`, `0A/06/1E/60/28`),
  with `boss_oam = 18` cells; `unported` is empty.
- The fight runs: after the fade expires the parts leave their intro (sprite `$20`), bob to
  `SCRY = $90`, and walk (`$9D3D`). **Correction (2026-10-15, frame-by-frame diff):** the
  "console frame 440 vs the port's 470" gap above is not a one-off — the port's whole arena
  cascade is **24 frames late**, and "the port's Mario dies to a walking part exactly as the
  console's does" is **wrong**: the console's Mario dies at f462 and the port's does not die at
  all on that tape. See "The arena cascade is 24 frames late" below.
- Tests: six for the render walk (six mutations red), three for the swap (three mutations red: a
  region offset, the OBJ block's high-half source, the metatile table), one for the fade
  decrement (mutation red), and the two `$8E6C` staging tests rewritten onto the +$20 stride.
  270 core + 3 runtime green. The capture gate cannot see any of this — no capture has ever
  entered a boss arena.

## The arena cascade is 24 frames late, and `$92DF` did not exist — 2026-10-15

Both of these came out of a frame-by-frame diff of the level 13 fight against PyBoy (route via
`captures/probe_level_select.py`, Mario and camera poked to the pipe approach, Right held), after
the user reported the boss still behaving unlike the console. The earlier "96.5% of pixels
identical" check compared **one frame of parked slots** — it could not see either problem.

### The console's own timeline (measured per frame, level 13)

| frame | console | port |
|---|---|---|
| f28 | `sPipeTravelDirection = $30` (Mario frozen at 3779, walking stops) | — (Mario keeps walking to 3825) |
| f52 | `sSceneId = $10` | `scene_id = $10` (same) |
| f54 | — | the scene VM runs `$2x`: `sSceneState = 0F`, `sFadeTimer = $0160` |
| f68 | the scene VM runs `$2x` — `sSceneState = 0F`, `sFadeTimer = $0160` → `$015F` | — (fade frozen at `$0160`) |
| f74 / f57 | `sSceneId` cleared (script `$FF`) | f57 |
| f75 / f92 | Mario placed at the destination (1523,104) | `cam.transition` ends |
| f420 / f444 | `sFadeTimer = 0`; the parts leave their intro on f421 | f444 |

So the *whole fight* runs 24 frames late. From f421 onward the port's per-part trajectories are
**byte-identical** to the console's (verified slot by slot from f440, once the 24-frame shift is
applied) — the parts do not act like each other; the port's records match the console's exactly,
including each part's own anim (`$0C`/`$0B`/`$0A` in slots 0/1/2), start position and bob phase.

Two measured causes, both still open (`boss_arena_phase_cascade` in `PORT_QUEUE.json`):

1. **The fade timer is frozen for the travel frames.** `PlayerSim::step` decrements `sFadeTimer`,
   and the runtime skips `step` while `cam.transition != 0`. The ROM's frame loop ticks it every
   frame (`home.asm` ~115, unconditionally). 39 travel frames of hold → 24 frames late.
2. **The scene VM reaches the arena's ops 14 frames early.** Both sides latch `sSceneId` at f52,
   but the console spends ~16 frames in `$20A4`'s prologue (`$2CFF` → bank 1 `$52E7`, `$2728` =
   the one-byte wait opcode, `call $FFA0`) before the first script byte; the port runs the first
   byte two frames after the latch. Fixing (1) alone therefore overshoots (≈15 frames *early*), so
   the two have to land together.

### `$92DF`'s Mario-dead guard (ported)

```text
$92DF:  ld a,[sMarioDead] / cp $FF / ret z     ; dead -> the handler is not called at all
```

The `ret z` returns to the *driver*, which then still renders the slot, clears `$FFE7`, runs
`$9048` and writes the slot back (`$81BD`) — so a frozen house stays on screen holding its last
values. Console-verified: on this tape all three records stop changing on f462 (the frame Mario
dies) and hold through the death animation, while the port's parts kept walking. Pinned by
`a_dead_mario_freezes_every_boss_slot` (mutation red).

### Evidence (scratch probes, re-runnable)

`arena_fade_probe.py` → `console_fade.log` (per-frame `sFadeTimer`, f20–460: the f68 set / f420
zero), `arena_transition_probe.py` → `console_trans.log` (per-frame slot-2 record, f395–465: the
byte-identical state-3 body, 23 frames late), `arena_fight_probe.py` → `console_fight.log` (16
marks f450–650: the slot-by-slot fight window), `arena_travel_probe.py` → `console_travel.log`
(f20–100: `sPipeTravelDirection`, `sSceneId`, Mario, scroll — the console's f28/f52/f68/f75
sequence). The port side of each is a temporary `eprintln!` trace on `frame_n` in
`crates/sml2-runtime/src/main.rs`, removed after every run.

### Still divergent after all this

- **The port's Mario does not die** where the console's does (f462): something in the fight damages
  Mario on hardware that the port does not reproduce — most likely the child the parts spawn, which
  is anim `$0D` (`$9EE3`, still unported), plus its siblings.
- **The fight cannot be completed.** With anim `$0D` unported the controller's child is inert and
  cannot be damaged, so `sObjectDefCount` never reaches 0 and the arena never clears — the
  "you can't beat the boss" report. The anim-3 exit (`$9323`, ported this session) is shared, so a
  defeated child leaves correctly the moment `$9EE3` takes hits.

## What actually ends the def-2 fight, and the cascade's real root — 2026-10-15

Two corrections to this session's earlier report, both from hardware rather than from reading.

### The child (anim `$0D`) is a teardown gate, not the win — and the win's last step is missing

`$9EE3` is now ported (it was the one unported anim the def-2 fight actually spawns), and its
body says what it is: `ld a,[sObjectRemoveMode] / cp $02 / ret nz`, then `SCRY = $90`,
`SCRX = $58`, then `jp $58F8` which rewrites the record to **anim 4 / sprite 9** with state,
phase and type cleared. So it waits for `sObjectRemoveMode == 2` — a value only a defeat-exit
leaves behind — and the controller *clears* that byte to 0 before it spawns it. Poking
`sObjectRemoveMode = 2` on the console tape proves both halves:

```
f560  (poke remove_mode = 2)  s1[anim=0D fst=00 y=40 x=58 spr=0E]
f561                          s1[anim=04 fst=00 y=90 x=58 spr=09]   <- the ported writes, exactly
f562  count 1 -> 2            (anim 4's state 0 spawns anim 5 / sprite $0C)
f820  (poke the $92C4 boss-clear: remove_mode = 1, fade = $0280)
      ... $FF9B = $1F, the fade counting down from $0280
```

So the *player's* win is the other path: `sBossHits` (`$AFC5`, seeded 6, 2 per stomp) reaching 0 →
`$92C4` sets `sObjectRemoveMode = 1` and `sFadeTimer = $0280` (the 640-frame boss-clear fade, which
the port models) → **`$FF9B = $1F`** (console-measured above) → the overworld hand-off, which is
still unported. **That hand-off is why "you can't beat the boss"**, not anim `$0D`: the port's
boss-clear fade expires and then has nowhere to go.

The `$1D` ending (the boss riding out of the room) is a *different* path — the anim-4 → anim-5
chain (`$9642`/`$9698`, ending at `$5688`: rise 4 px a frame, and under `SCRY = 8` set
`$FF9B = $1D`) which is reachable only while `sObjectRemoveMode == 2`. Its bodies are decoded in
`boss_anim_4_5_chain` in `PORT_QUEUE.json`; not ported yet.

### The cascade: the port's pipe entry is 16 frames long, and a naive VM gate is worse

Console vs port, level 13, Right held, per frame:

| | arm | `sSceneId=$10` | scene ops | Mario placed | fade set | fade 0 | parts act |
|---|---|---|---|---|---|---|---|
| console | f28 | f52 | f67–f70 | f75 | f68 | f420 | f421 |
| port | f29 | f52 | f53–f57 | f92 | f54 | f444 | f444 |

The arm itself is **right** — the port's `$188B` walk-in arm fires at f29 with Mario at world 3779,
the console's f28 at 3779. What differs is everything after it: the port takes 63 frames from the
arm to the placement (a 23-frame `pipe_dir = 3` slide plus a 40-frame camera transition) where the
console takes 47, and the console's scene ops run *inside* the travel — `sPipeTravelDirection` is
still `$30` at f67–f70, eight frames before Mario is placed. Gating the scene VM on "is Mario
travelling" (the port's `pipe_dir >= 0x10`) therefore does not fix anything: it moves the `$2x`
arm past the placement and lands Mario at 1308 instead of 1561. That gate was tried, measured,
and reverted; the real gap is the ROM's travelling *sub-phase*, which ends before
`sPipeTravelDirection` clears. `boss_arena_phase_cascade` in `PORT_QUEUE.json` carries the
numbers.
