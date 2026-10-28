# Expanded levels

Created 2026-10-27. Keep status here; do not create more tracking documents.

## Goal

Let levels be larger than the ROM's 16×3 screens, so stitched levels can be
rebuilt as one contiguous map and new levels have room. Original play stays
ROM-exact and remains the verified default.

## Why it is possible

Positions are already `u16`, and object slots already store a full high byte.
The ROM uses 4 bits of the high byte for X (`& 0x0FFF`, 16 screens) and
a 48-row layout for Y (3 screens). Using the whole high byte allows up to 256
screens on X. Y must stay below `$FF00`, which the engine treats as "above the
level" (`player_sim.rs`, `while self.y >> 8 == 0xFF`). This is about capacity,
not speed: the masks cost nothing.

Stitched levels today use `$4x` scene teleports (`scene.rs`) between screens.
That happens in level IDs 2, 6, 7, 10, 15, 17, 20 and 24. In Tree Zone 2 (ID 2),
screens (y1, x5) and (y0, x4) are the two ends of one pipe. Rebuilt, that
level needs four screen rows, so it cannot fit the ROM world.

## Rules

- ROM levels keep their 16×3 world and wrap behavior. Every existing unit test,
  ROM byte test and `captures/sweep.sh` passes unchanged after each step.
- Expanded content is a separate content mode (on by default, see Decisions). It is chosen at level entry, the same way
  DX layouts are substituted in `level_content` (`sml2-runtime/src/session.rs`).
  It is never mixed into vanilla data.
- Collision, actors and scene scripts read world size through one type, not
  through constants scattered across the code.

## Work order

1. **Inventory — complete (see [Inventory](#inventory)).** List every place that assumes the ROM world: `& 0x0FFF`/`hi &
   0x0F` masks, `LAYOUT_WIDTH`/`LAYOUT_HEIGHT`/`LAYOUT_SIZE`, the 48-byte edge and
   scene maps, layout indexing (`$B000 + row*$100 + col`) and object streaming
   by screen. Record each one as wrap-relevant or size-only.
2. **World size type.** Introduce a `World` that holds the width and height in
   screens, the layout stride and the wrap rule. ROM levels construct
   `World::ROM` (16×3, 12-bit wrap). Route the inventory through it without
   changing behavior, in bounded steps: camera, then layout/collision, then
   scene/edge tables, then object streaming.
   - Camera — complete. `sml2-core/src/world.rs` defines `World`; `CameraSim`
     wraps, indexes its tables and bounds rooms through `cam.world`.
   - Player and enemy probes — complete. `World::cell`, `echo_row` and
     `cell_key` replace the `& $FF` cells, the WRAM echo and the
     `row*256+col` keys; `PlayerSim` and `ObjectSim` carry `world`. The
     session must set all three `world` fields together (step 3).
3. **Sized content.** `LevelLayout`, the edge map and the scene map take their
   dimensions from the `World`. Extraction still writes 256×48 for ROM levels.
   Define a versioned expanded-level *recipe*. ROM and DX data are never
   redistributed, so a recipe stores only authored operations: world size, region
   moves from the loaded layout, authored metatile cells, lock and scene edits,
   spawn moves and the start position. It applies at load to the player's own
   vanilla or DX extraction.
4. **Expanded mode.** Load expanded levels through the `level_content` seam,
   with a setting and save-safe selection at level entry. Renderers, widescreen
   room clamping and scenery overlays use the level's size.
5. **Editor level tools.** The Scenery Editor gains terrain, lock, spawn and
   start-position editing for expanded levels, with collision checks and
   playtest.
6. **First rebuild: Tree Zone 2.** Lay the sections out contiguously, remove the
   two pipe teleports, set the locks and move the spawns. Play it in normal and
   DX graphics.
7. **Remaining stitched levels.** Rebuild the other seven teleport levels one at
   a time.

## Inventory

Non-test code as of `f483b5b`. **Wrap** means original play depends on the
exact ROM wrap or aliasing and must keep it under `World::ROM`. **Size** means
a fixed 256×48 or 16×3 size that only needs to become per-level.

| Area | Location | Kind | What it assumes |
|---|---|---|---|
| Camera position | `camera.rs:274-287, 414-694` (20 `& 0x0FFF`) | Wrap | Scroll, slides and room spans wrap at 4096 px |
| Camera screen index | `camera.rs:258-260, 297-311, 773` | Wrap | `(hi & $0F)` aliases screens 16+ onto 0-15 for edge, room and scene lookups |
| Mario screen warps | `camera.rs:564` (`(hi+1) & $0F`), `camera.rs:629` | Wrap | Transitions rewrite Mario's high byte within 16 screens |
| Camera tables | `camera.rs:53-54, 88, 114, 311` | Size | `Room::WORLD` from layout constants; `[u8; 48]` edge map, 48-entry scene map, 16-wide rows |
| Player collision | `player_sim.rs:653-673` | Wrap | Stride 256, `& $FF` row/col, rows 48-77 mirror rows 16-45 (WRAM echo), 0 (solid) outside |
| Player cell writes | `player_sim.rs:666, 1116-1143` | Size | `row*256+col` keys, `u8` row/col, `& $FF` |
| Above the level | `player_sim.rs:1944` | Wrap | Y high byte `$FF` means above the top, so the last Y screen is reserved |
| Enemy terrain probes | `objects/helpers.rs:13-21` | Wrap | `(x>>4) & $FF`, `(y>>4) & $FF`, 0 outside, no echo rows |
| Enemy spawn stream | `objects/staging.rs:8-9`, `objects/helpers.rs:24, 43`, `objects/record_origin.rs:42` | Size | Records addressed as `$AB06 + 6i`, high byte `$AD`+ = not a record: at most 84 spawns per level. Screen comparisons already use the full high byte |
| Scene teleports | `scene.rs:437-441` | Size | `$4x` operands hold 4-bit screen numbers; rebuilt levels drop these scenes, and new scenes need a wider form |
| Layout type | `level.rs:37-41, 99-113` | Size | Constants; `from_bytes` requires exactly `$3000` bytes |
| Level content | `content.rs:59-89`, `session/level.rs:101-105`, `play.rs:82-101`, `viewer.rs:57-58` | Size | `Option<[u8; 48]>` edge map and fixed world pixels |
| Renderers | `render.rs:210-211, 260, 305`, `render/dx.rs:975-976, 1001`, `render/margin.rs:71-110`, `scene_scenery.rs:243-244` | Size | World pixels from constants, `cy*256+cx` write keys, 16-screen rows |
| Scenery overlay | `scenery.rs:78-167` | Size | Dense in-layout grid sized by constants (`i16` paint coordinates already reach 32,767 cells) |
| Editor | `targets.rs:94-98`, `state.rs:166`, `canvas.rs:107, 204` | Size | Canvas and layout dimensions from constants |

Stays ROM-only and unchanged: extraction and DX layout decoding (`sml2-extract`
`vanilla.rs:507-543`, `assets.rs:43-45`; `sml2-dx` `presentation.rs:676-685`)
and the capture tools (`cli.rs:330-502`, `replay.rs:291-367, 757-767`), which
compare against ROM captures.

Outside world size but still limits for new levels: 128 metatiles per level
table and the level's VRAM tile budget. Rebuilds reuse the level's own tiles,
so they are unaffected.

Step 2 order from this: camera (the largest wrap block), then one shared
`World` cell lookup for player and enemy probes, then the camera tables and
content types, then renderers and editor. The 84-spawn cap is a separate step
before any level needs more spawns.

## Decisions

- Rebuilds are designed and checked against DX layouts first, then vanilla.
  DX edits layouts in place (see the [DX audit](../sml2dx-audit.md)), so a
  recipe that moves whole regions carries DX's own edits with it.
- Expanded levels ship on by default, with a setting that restores the ROM layouts.
