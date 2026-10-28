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
- Expanded content is an opt-in mode. It is chosen at level entry, the same way
  DX layouts are substituted in `level_content` (`sml2-runtime/src/session.rs`).
  It is never mixed into vanilla data.
- Collision, actors and scene scripts read world size through one type, not
  through constants scattered across the code.

## Work order

1. **Inventory.** List every place that assumes the ROM world: `& 0x0FFF`/`hi &
   0x0F` masks, `LAYOUT_WIDTH`/`LAYOUT_HEIGHT`/`LAYOUT_SIZE`, the 48-byte edge and
   scene maps, layout indexing (`$B000 + row*$100 + col`) and object streaming
   by screen. Record each one as wrap-relevant or size-only.
2. **World size type.** Introduce a `World` that holds the width and height in
   screens, the layout stride and the wrap rule. ROM levels construct
   `World::ROM` (16×3, 12-bit wrap). Route the inventory through it without
   changing behavior, in bounded steps: camera, then layout/collision, then
   scene/edge tables, then object streaming.
3. **Sized content.** `LevelLayout`, the edge map and the scene map take their
   dimensions from the `World`. Extraction still writes 256×48 for ROM levels.
   Define a versioned expanded-level file (layout, metatile table reference, locks,
   scenes, spawns, start position).
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

## Open questions

- DX changes some layouts in place. Should rebuilds start from vanilla or DX data,
  or be built for each?
- Do expanded levels ship by default in a release, or stay a setting?
