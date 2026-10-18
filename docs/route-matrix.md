# Level route matrix

**Status: all 32 rows measured 2026-10-18; the first slice (12, 19, 28, 29) is
reviewed, and every exit cell in the game is test-pinned.** Tool:
[`tools/route_matrix.py`](../tools/route_matrix.py); acceptance:
`crates/sml2-core/src/route_matrix_tests.rs`.

A row is source-derived, not transcribed: the tool reads the extracted manifest,
the decompressed layout, the 48-byte scene map, the overworld node table, and
the ROM's `$377B` secret-entry overlay. It answers the five questions the
completion workflow asks of a level: **entry, exit, secret route, required
mechanic, boss**.

## Full matrix

Level numbers are manifest numbers (labels `Level_NN`); the runtime takes
`--level <id>`, where id = NN−1. Zone is the header's tileset label with the
`GFX_Tileset` prefix removed. Node is the overworld `$4218` index that stores
this level; `—` means no map node (castle/arena entry). Exit cells are
`(col,row)$tile` in the 256×48 layout; secret entry is the `$377B` overlay in
pixels. Mechanics are the tool's flags: `float` = the level's `space_physics`
float bit (bit 3 = timed float, bit 0 = hold-A float), `moonzones` = `$52..$57`
tiles, `scroll` = scroll limit,
`auto` = autoscroll, `water`, `hazard`, `pipe`, `sidepipe`, `alt-exit` = a
`$4B` bell.

| level | id | zone | node | exit cell(s) | secret entry | boss | mechanics |
|---|---:|---|---:|---|---|---|---|
| 1 | 0 | Level1 | 0 | (254,30)$4A | (2778,420) | — | hazard,pipe |
| 2 | 1 | TreeZone1 | 35 | (253,30)$4A | (2144,468) | — | moonzones,pipe |
| 3 | 2 | TreeZone2 | 34 | (238,10)$4B (238,30)$4A | (1807,132) | — | scroll,water,hazard,pipe,sidepipe,alt-exit |
| 4 | 3 | TreeZone3 | 33 | (222,30)$4A | (2572,132) | — | scroll,hazard,pipe,sidepipe |
| 5 | 4 | TreeZone4 | 36 | (78,14)$4A | — | — | scroll,hazard,pipe |
| 6 | 5 | TreeZone5 | 32 | — | (1704,596) | def 0 | scroll,hazard,pipe,sidepipe |
| 7 | 6 | PumpkinZone1 | 40 | (190,30)$4A | (1722,468) | — | scroll,hazard,pipe |
| 8 | 7 | PumpkinZone2 | 41 | (222,30)$4A (158,46)$4B | (1621,356) | — | moonzones,hazard,pipe,sidepipe,alt-exit |
| 9 | 8 | PumpkinZone3 | 42 | (238,29)$4B (238,45)$4A | (2340,452) | — | hazard,pipe,alt-exit |
| 10 | 9 | PumpkinZone4 | 43 | — | (1902,468) | def 1 | hazard,pipe,sidepipe |
| 11 | 10 | MarioZone1 | 80 | (174,46)$4A | (1744,628) | — | scroll,hazard,pipe,sidepipe |
| 12 | 11 | MarioZone2 | 81 | (238,29)$4A | (2097,404) | — | hazard,pipe |
| 13 | 12 | MarioZone3 | 82 | (254,30)$4A | (1995,436) | — | hazard,pipe |
| 14 | 13 | MarioZone4 | 83 | — | (2452,212) | def 2 | hazard,pipe,sidepipe |
| 15 | 14 | TurtleZone1 | 70 | (190,40)$4A | (1699,660) | — | moonzones,pipe |
| 16 | 15 | TurtleZone2 | 72 | (238,45)$4B (174,46)$4A | (1677,308) | — | scroll,moonzones,hazard,pipe,sidepipe,alt-exit |
| 17 | 16 | TurtleZone3 | 73 | — | (1369,404) | def 3 | water,moonzones,hazard,sidepipe |
| 18 | 17 | TurtleZone1 | 5 | (190,11)$4A (190,15)$4B | — | — | moonzones,pipe,alt-exit |
| 19 | 18 | SpaceZone | 60 | (190,11)$4B (190,29)$4A | (1905,436) | — | float,scroll,hazard,pipe,alt-exit |
| 20 | 19 | SpaceZone | 61 | — | (3463,180) | def 4 | float,scroll,auto,hazard,pipe,sidepipe |
| 21 | 20 | MacroZone1 | 50 | (238,29)$4B (254,45)$4A | (2613,708) | — | hazard,pipe,sidepipe,alt-exit |
| 22 | 21 | MacroZone2 | 51 | (222,30)$4A | (2549,628) | — | moonzones,pipe |
| 23 | 22 | MacroZone3 | 52 | (222,8)$4A | (2291,468) | — | moonzones,hazard,pipe,sidepipe |
| 24 | 23 | MacroZone4 | 53 | — | (2067,468) | def 5 | hazard,pipe,sidepipe |
| 25 | 24 | WarioCastle | — | — | — | def 6 | scroll,moonzones,hazard,sidepipe |
| 26 | 25 | Level1 | 4 | (126,29)$4A (126,30)$4A | — | — | auto,pipe |
| 27 | 26 | Level1 | 102 | (126,29)$4A (126,30)$4A | — | — | hazard,pipe |
| 28 | 27 | TreeZone1 | 120 | (78,30)$4A | — | — | pipe |
| 29 | 28 | SpaceZone | 68 | (94,28)$4A | — | — | float,scroll,hazard,pipe |
| 30 | 29 | TreeZone1 | 38 | (174,23)$4A | — | — | — |
| 31 | 30 | TreeZone5 | 106 | (126,26)$4A (126,27)$4A | — | — | auto,hazard,pipe |
| 32 | 31 | TreeZone2 | 122 | (174,9)$4A | — | — | scroll,hazard,pipe |

## Anomalies and classification

- **35 bells across 25 levels**: 28 plain `$4A` and 7 `$4B` twins (levels 3, 8,
  9, 16, 18, 19, 21). Levels 6, 10, 14, 17, 20, 24 and 25 have no bell at all —
  the six boss arenas and the castle end through a boss clear (`$8DFD` ladder:
  defs 0–5) or the Wario phase exits (defs 6/7/8), not the `$4A` arm.
- **Three levels have a two-cell bell** — 26, 27 and 31 each carry `$4A` at
  (126,29)+(126,30) or (126,26)+(126,27), a vertically stacked bell graphic.
  The 35-cell count includes both cells; each arms independently.
- **Level 25 (`WarioCastle`, id 24) has no overworld node and no bell.** It is
  entered from the castle node and ends through Wario's three forms, which is
  why its `$377B` record is zero and the arena is scene-driven.
- **Level 18 (id 17) has two bells 4 rows apart** — `$4A` at (190,11) and `$4B`
  at (190,15), the tightest pair in the game. Level 19 (the reviewed slice
  member) has the same column 288 px apart.
- **Zone re-use in the last eight levels.** Levels 26–32 reuse the Level1, Tree
  and Space tilesets rather than carrying their own; level 25 is the only
  `WarioCastle` tileset. The zone column is the header's graphics label, so it
  is evidence rather than a naming guess.
- **Level 32's `$377B` slot is padding** — the record decodes past the 48-row
  layout, so the tool reports no secret entry; meaningful records exist only
  through level 31.
- **Autoscroll levels**: 20, 26 and 31 (`auto_scroll = 1`); level 20 also sets
  moon bit 0 (the hold-A float), making it the only autoscroll + float level.
- **Float levels**: bit 3 (timed float) on 19 and 29; bit 0 (hold-A float)
  on 20. Those are the only three levels with a non-zero `space_physics` byte.

## Reviewed slice (levels 12, 19, 28, 29)

The four levels the handler tail finished, reviewed line by line and pinned by
named tests.

| level | id | entry | exit(s) | secret entry | boss |
|---|---:|---|---|---|---|
| 12 | 11 | node 81, (56,452), cam (80,448), time 4 | `$4A` (238,29) = (3808,464) | (2097,404), lands row 27 | none |
| 19 | 18 | node 60, (45,436), cam (80,432), time 7 | `$4A` (190,29) = (3040,464); `$4B` (190,11) = (3040,176) | (1905,436), lands row 29 | none |
| 28 | 27 | node 120, (51,260), cam (80,246), time 4 | `$4A` (78,30) = (1248,480) | none | none |
| 29 | 28 | node 68, (40,452), cam (80,438), time 4 | `$4A` (94,28) = (1504,448) | none | none |

### Level 12 (id 11)

- **Entry.** Node 81 (progress slot 27), header spawn (56, 452) on layout 1;
  Mario Zone 2 tileset (`GFX_TilesetMarioZone2`).
- **Exit.** Single `$4A` bell at cell (238, 29), just left of the `$1D` wall that
  closes the level (`(3808, 464)` in pixels). The floor run into the bell is the
  `$50/$51` hazard strip at row 29, columns 43–232 — the level's dominant
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
- **Mechanics.** `space_physics = $08` (moon bit 3, the timed float) and
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
  at (95, 28).
- **Secret route.** None.
- **Mechanics.** `space_physics = $08` (moon bit 3, timed float) and
  `properties = $08`.
- **Objects.** 6 records; anims 15/21/90 — the smallest stream in the game;
  `$5B0D5` (anim 90) is the only distinctive handler. No boss arena scene.

## Acceptance evidence

```bash
cargo test -p sml2-core route_matrix
# 5 passed: the four named slice tests + every_source_exit_cell_arms_the_bell
```

`every_source_exit_cell_arms_the_bell` loads all 32 extracted levels, finds
every `$4A`/`$4B` cell (35 across 25 levels, count pinned), stands Mario where
the `$1BFF` probe samples it, and requires `exit_phase`, the 192-frame fade, the
level-clear music request, and the `$4B` flag. The four named tests additionally
assert the slice's exact source cells.

This is the "exit" acceptance for the whole game. It does not yet play the
approach — the smoke harness runs 1200 frames and never reaches a bell.

## Gaps

1. **The secret-route entry overlay is not ported.** `$55C0` copies the level's
   8-byte record from ROM `$377B` over the header's Y/X/camera block when
   `sLevelProgressFlags` bit 3 (or the secret-route item) set the flag. The
   extraction has no field for it and the runtime always seeds the header spawn,
   so the 22 levels with a non-zero record cannot be entered from their secret
   points. Promoted as `secret_route_entry_overlay`.
2. **Boss definition 0 is blocked** — level 6 (TreeZone5, scene `$0B`) resolves
   to def 0, which `boss_def_portable` excludes. Promoted as
   `boss_def0_arena_entry`.
3. **The approach routes are unplayed.** Every bell arms when stood on, but no
   test walks from a spawn (or secret entry) to a bell. Level 19's upper bell and
   the castle/night levels' bells are the obvious first played routes.
4. **The 28 unreviewed rows are tool-measured, not hand-checked.** Their zone,
   node, exit and secret columns are trustworthy by construction; the
   "required mechanic" summary and any route narrative still need a review pass.
5. **Time limits and music tracks are reported, not asserted** — they come from
   the header and are seeded by `seed_level_timer`, but no test pins per-level
   values.
