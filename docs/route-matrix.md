# Level route matrix

**Status: first slice measured 2026-10-18 — levels 12, 19, 28, 29** (the four
the handler tail finished). Tool:
[`tools/route_matrix.py`](../tools/route_matrix.py); exit-cell acceptance:
`crates/sml2-core/src/route_matrix_tests.rs`.

A row is source-derived, not transcribed: the tool reads the extracted manifest,
the decompressed layout, the 48-byte scene map, the overworld node table, and
the ROM's `$377B` secret-entry overlay. The matrix answers the five questions the
completion workflow asks of a level: **entry, exit, secret route, required
mechanic, boss**.

## Slice

Level numbers are manifest numbers; the runtime takes `--level <id>` (`levelNN`
is id NN−1), so this slice is `--level 11, 18, 27, 28`.

| level | id | node / slot | entry | exit(s) | secret entry | boss | mechanics |
|---|---:|---|---|---|---|---|---|
| 12 | 11 | 81 / 27 | (56, 452), cam (80, 448), time 4 | `$4A` cell (238,29) = (3808, 464) | (2097, 404), cam (2130, 442), lands row 27 | none | Mario Zone 2 tileset; hazards, one pipe tile |
| 19 | 18 | 60 / 16 | (45, 436), cam (80, 432), time 7 | `$4A` (190,29) = (3040, 464); `$4B` (190,11) = (3040, 176) | (1905, 436), cam (1938, 445), lands row 29 | none | Space Zone tileset; moon physics, scroll limit, hazards, pipe shaft, **alternate exit** |
| 28 | 27 | 120 / 38 | (51, 260), cam (80, 246), time 4 | `$4A` (78,30) = (1248, 480) | none | none | Tree Zone tileset; pipes |
| 29 | 28 | 68 / 41 | (40, 452), cam (80, 438), time 4 | `$4A` (94,28) = (1504, 448) | none | none | Space Zone tileset; moon physics, scroll limit, hazards |

Coordinates are pixels; the exit cell is the metatile the `$1BFF` probe samples
at `(x+16, y+28)`. "Lands row" is the first non-air (`$60`) cell below the
secret drop-in point.

## Per level

### Level 12 (id 11)

- **Entry.** Node 81 (progress slot 27), header spawn (56, 452) on layout 1;
  Mario Zone 2 tileset (`GFX_TilesetMarioZone2`).
- **Exit.** Single `$4A` bell at cell (238, 29), just left of the `$1D` wall that
  closes the level (`(3808, 464)` in pixels). The floor run into the bell is the
  `$50/$51` hazard strip at row 29, columns 43–232 — it is the level's dominant
  traversal hazard (44 cells).
- **Secret route.** The ROM overlay drops Mario at (2097, 404) — pixel column
  131, four rows above the row-27 landing (`$18`, a solid). Data only: the port
  does not apply the overlay (see gaps).
- **Objects.** 31 records; anims 9/15/17/19/20/22/23/25/27/28/29/58/68/77.
  Distinctive: cannon ant (anim 77, `$5A773`), `$5A545` (anim 68), `$591F9`
  (anim 58). No boss arena scene.

### Level 19 (id 18)

- **Entry.** Node 60 (progress slot 16), header spawn (45, 436), time limit 7;
  Space Zone tileset (`GFX_TilesetSpaceZone`).
- **Exit.** Two bells in the same column, 288 px apart:
  - plain `$4A` on the floor at (190, 29) → (3040, 464);
  - the alternate `$4B` at (190, 11) → (3040, 176), reached from the row-12 pipe
    corridor that runs west to the vertical `$07` pipe shaft at column 191.
  `$4B` is the "route already cleared" twin: it leaves `sExitTileFlag = $FF`,
  which the `$3547` tally guard reads.
- **Secret route.** Overlay drops Mario at (1905, 436) — inside the level, one
  cell above the row-29 floor. Data only (unported overlay).
- **Mechanics.** `space_physics = $08` (permanent low gravity) and
  `properties = $08` (scroll limit) — the same pair level 29 carries.
- **Objects.** 25 records; cannon ant (77), `$5AFD7` (anim 88),
  `$5B0D5` (anim 90). No boss arena scene.

### Level 28 (id 27)

- **Entry.** Node 120 (progress slot 38), header spawn (51, 260), layout 3;
  Tree Zone 1 tileset (`GFX_TilesetTreeZone1`).
- **Exit.** Single `$4A` bell at (78, 30) → (1248, 480), on the row-31 solid
  floor, adjacent to a `$07` pipe tile at (79, 30).
- **Secret route.** None: the `$377B` record is all zero, and the layout has no
  `$4B` cell.
- **Objects.** 14 records; anims 9/19/30/58. Distinctive: `$586EB` (anim 30)
  and `$591F9` (anim 58). No boss arena scene.

### Level 29 (id 28)

- **Entry.** Node 68 (progress slot 41), header spawn (40, 452), layout 4;
  Space Zone tileset (`GFX_TilesetSpaceZone`) — the same zone as level 19.
- **Exit.** Single `$4A` bell at (94, 28) → (1504, 448), beside a `$07` pipe tile
  at (94, 28)+1.
- **Secret route.** None.
- **Mechanics.** `space_physics = $08` (low gravity) and `properties = $08`.
- **Objects.** 6 records; anims 15/21/90 — the smallest stream in the slice;
  `$5B0D5` (anim 90) is the only distinctive handler. No boss arena scene.

## Acceptance evidence

`route_matrix_tests` loads each extracted level, asserts the source cell really
holds `$4A`/`$4B`, stands Mario where the probe samples it, and requires
`exit_phase`, the 192-frame fade, `music_request = 3`, and the `$4B` flag:

```bash
cargo test -p sml2-core route_matrix
# 4 passed
```

This is the "exit" acceptance for the slice. It does not yet play the approach —
the smoke harness runs 1200 frames and never reaches these bells.

## Gaps

1. **The secret-route entry overlay is not ported.** `$55C0` copies the level's
   8-byte record from ROM `$377B` over the header's Y/X/camera block when
   `sLevelProgressFlags` bit 3 (or the secret-route item) set the flag. The
   extraction has no field for it and the runtime always seeds the header spawn,
   so levels 12 and 19 cannot be entered from their secret points. Promoted as
   `secret_route_entry_overlay`.
2. **No boss in this slice.** None of the four is in the `$8DFD` ladder
   (`$05,$09,$0D,$10,$13,$17`), and none has an arena scene (`$0B`–`$10`). Boss
   arena rows belong with the boss levels and `boss_def0_arena_entry`.
3. **The `$4B` semantics are source-inferred, not console-verified for this
   level.** The code path is measured (`$1CBA` writes `sExitTileFlag = $FF`) and
   the map return rewrites `$4B` nodes, but no capture of level 19's upper bell
   exists; the route to it (pipe shaft → row-12 corridor) is unplayed.
4. **Time limits and music tracks are reported, not yet asserted.** They come
   from the header and are seeded by the runtime (`seed_level_timer`), but no
   test pins per-level values.
5. **The rest of the matrix is unmeasured.** Running
   `python3 tools/route_matrix.py --table` now covers all 32 levels; only these
   four have been reviewed and test-pinned.
