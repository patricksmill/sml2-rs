# Level route matrix

**Status: all 32 rows measured 2026-10-18; the first slice (12, 19, 28, 29) is
reviewed, every exit cell is test-pinned, levels 28/29 have played header
approaches, and level 19 has played both its header and secret starts to
both bells.** Tool:
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
  131, four rows above the row-27 landing (`$18`, a solid). The loader applies
  this entry when the node progress byte has bit 3 set.
- **Objects.** 31 records; anims 9/15/17/19/20/22/23/25/27/28/29/58/68/77.
  Distinctive: cannon ant (anim 77, `$5A773`), `$5A545` (anim 68), `$591F9`
  (anim 58). No boss arena scene.

### Level 19 (id 18)

- **Entry.** Node 60 (progress slot 16), header spawn (45, 436), time limit 7;
  Space Zone tileset (`GFX_TilesetSpaceZone`). The source header at `$5773`
  begins `B4 01 2D 00 B0 01 50 00`: camera (80,432), normal progress `$00`.
- **Exit.** Two bells in the same column, 288 px apart:
  - plain `$4A` on the floor at (190, 29) → (3040, 464);
  - the alternate `$4B` at (190, 11) → (3040, 176), reached from the row-12 pipe
    corridor that runs west to the vertical `$07` pipe shaft at column 191.
  `$4B` is the "route already cleared" twin: it leaves `sExitTileFlag = $FF`,
  which the `$3547` tally guard reads. The corridor is solid `$07` terrain:
  this level has no `$05` or `$08..$0F` pipe-mouth cells and its scene map is
  zero throughout. There is no scene-connected pipe warp to the upper bell.
- **Secret route.** Overlay drops Mario at (1905, 436) — inside the level, one
  cell above the row-29 floor. The loader applies it for progress bit 3.
- **Mechanics.** `space_physics = $08` (moon bit 3, the timed float) and
  `properties = $08` (scroll limit) — the same pair level 29 carries. Three
  hidden `$7E` blocks at column 121, rows 23/19/15 form the upper-route ladder.
  A native ceiling bonk reveals each as solid `$07`, giving Mario footholds
  at Y 340/276/212 before the row-12 corridor at Y 164.
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

This establishes exit-cell activation for the whole game. The smoke harness
alone does not require a bell or a completed approach.

### Played route: level 28

`cargo test -p sml2-runtime played_routes` runs the same `run_play` loop as
interactive and headless play, with the original level28 header, layout, object
stream, edge map and scene map. Its fixed tape holds right for 1,400 frames.
Mario starts small at (51,260), camera (80,246), without invincibility or later
position writes. The route descends from the upper ledge to the lower corridor;
normal objects are active during the approach.

The test pins native bell arming on frame 1,077 at (1242,468), sampling the
source `$4A` cell (78,30), fade 192, music 3 and exit flag 0. Fade completion is
frame 1,269 and handoff frame 1,270; tally is 1 and Mario remains alive. The
source node is 120 (progress slot 38): `$3F104` rewrites it to 46, selects screen
1/music 30, and makes two saves. Progress and its serialized save byte are
`$80`. Disabling bell arming makes this acceptance test fail.

The live loop returns typed observations for these assertions; gameplay is not
reimplemented in the test. Direct level selection now derives an overworld
origin when the source table has exactly one matching node. Level id 17 has
two nodes (5 and 26), and castle id 24 has none; the CLI cannot yet infer their
route context. Full overworld entry remains separate work.

### Played route: level 29

`level29_timed_float_header_spawn_to_bell_through_live_play` uses the same live
loop and original level29 assets. Its 3,400-frame tape is 100 right-held frames,
then 22 repetitions of 50 A-held frames and 100 right-held frames. Horizontal
inertia persists during the A-only spans; there is no new input syntax or
position/physics rewrite. Mario starts small at (40,452), camera (80,438), with
the source `$08` moon property and node 68/progress slot 41.

The first observed airborne float use is frame 102: the budget changes from
50 to 49 while A is held. The run consumes 784 budget frames and processes
active objects during the approach. It arms the source `$4A` cell (94,28) on
frame 2,584 at (1507,436), alive, with fade 192/music 3/exit flag 0. Fade ends on
2,776 and the map handoff runs on 2,777; tally and serialized progress are 1
and `$80`. The return selects node 69, screen 4/music 28, and two saves.
`$3D56B/$3D573` intentionally sets `sMarioDead = $FF` after rewriting node 68;
the test separately requires zero gameplay death frames and `dead = 0` at the
bell, so that post-clear map latch cannot conceal a failed approach. Removing
only the timed-float velocity extension makes this tape stall before the bell
and fails the test; the original behavior is restored afterward.

### Played route: level 19 header start to the lower bell

`level19_header_spawn_to_plain_bell_through_live_play` starts at the source
header `$5773`, progress `$00`, route latch 0, Mario (45,436) and camera
(80,432). It uses all 25 original object records and the source timed-float
physics. Its fixed 4,838-frame tape crosses the first broad spike bed via the
column-60 foothold, refreshes the grounded float budget on narrow platforms,
and releases A early over the later thorn gap to avoid the enemy above it.
There are no position, physics, object or invincibility overrides after entry.

With zero gameplay death frames, `$4A` arms on frame 4,611 at (3043,452),
fade 192, music 3, exit flag 0. Fade completes on 4,803 and map handoff runs
on 4,804. The final tally is 1, node 60, screen 4, music 28, with two saves.
Progress slot 16 and its save byte are `$80`; entry flags remain `$00`.
Disabling the grounded float budget rejects both the existing core float
contract test and this played approach.

### Played route: level 19 header start to the alternate bell

`level19_header_spawn_hidden_ladder_to_alternate_bell_through_live_play`
starts from the original header with progress `$00` and runs continuously
through the early thorn beds, the three hidden-block footholds, and the upper
platforms. Its fixed 7,254-frame tape uses movement inputs to approach the
ladder after crossing the first half of the level. Original camera, object,
clock and physics state carry through the entire route.

The test pins all three source `$7E` cells and their native `$07` overlays,
zero gameplay death frames, and the `$4B` arm on frame 6,922 at (3033,164),
fade 192, music 3, flag `$FF`. Fade completes on 7,114; handoff is 7,115.
Tally stays 0, and node 67/screen 4/music 28 receive one save. Progress slot 16
and its save byte are `$01`: starting from `$00`, the alternate path adds bit
0 without setting the checkpoint bit. Entry progress flags remain `$00`.

Removing the hidden-block bump rejects the core foothold test and this route.
Removing the Space Zone path-bit write rejects the core map handoff test and
this route's progress assertion. Both source starts now have native played
approaches to both bells; the live overworld loop remains outside these tests.

### Played route: level 19 secret entry to the lower bell

`level19_secret_entry_to_plain_bell_through_live_play` starts from the genuine
progress-bit-3 entry. The persistent node-60 progress byte (slot 16) is `$08`
before `LevelHeader::entry` runs. The test pins the copied `$A800-$A807` block
as `B4 01 71 07 BD 01 92 07`: Mario (1905,436), camera (1938,445), route latch
`$FF`, and moon property `$08`. Original layout and all 25 object records run
through the live loop; the test never rewrites position or physics afterward.

Its 1,400-frame typed tape has 23 fixed spans of movement, A, Up+A and idle.
These are native button holds, with ordinary press-edge handling. The source
`$1675..$16AE` branch retains velocity 75 when Up is held with A; ordinary
walking-band jumps select 48. `held_up_preserves_high_jump_across_the_walking_speed_band`
pins both speed-band boundaries and the same-frame gravity outputs (74/47)
in memory. Ignoring Up makes both that core test and this route fail.

The route consumes the timed-float budget (first observed on frame 22, 50→49)
and has zero gameplay death frames. It reaches the source `$4A` cell (190,29)
on frame 1,173 at (3043,452), with fade 192, music 3 and exit flag 0. Fade
completion is 1,365; handoff is 1,366. Tally is 1, node remains 60, screen is 4,
music is 28, and the handoff makes two saves. The live route latch clears;
persistent slot 16 and its serialized save byte become `$80`, clearing the
entry checkpoint bit.


### Played route: level 19 secret entry to the alternate bell

`level19_secret_entry_hidden_ladder_to_alternate_bell_through_live_play` uses
that same genuine `$08` entry and original object data. Its fixed 3,798-frame
button tape reveals the three `$7E` blocks at (121,23), (121,19), (121,15),
climbs their `$07` overlays, and crosses the original upper platforms. The
source layout stays unchanged. Core test
`hidden_block_bonk_produces_a_solid_foothold_for_the_floor_probe` separately
checks that native ceiling contact produces `$07` and the floor probe can
land on it; an unrevealed `$7E` cannot support that landing.

With zero gameplay death frames, the route arms `$4B` at (190,11) on frame
3,466, Mario (3033,164), fade 192, music 3, exit flag `$FF`. Fade completion
is 3,658 and map handoff is 3,659. Tally stays 0. The source map path rewrites
node 60 to slotless node 64 before progress marking, then selects node 67,
screen 4 and music 28. Slot 16 and its save byte become `$09`: the checkpoint
bit remains and the alternate path adds bit 0. The handoff makes one save.
The route latch clears, while entry progress flags remain `$08`.

Removing the hidden-block bump branch rejects both the core foothold test
and this played route. Removing the alternate-exit tally guard rejects the
played route's tally assertion.

## Gaps

1. **Secret-route entries are implemented; live overworld integration remains.**
   Extraction preserves all 32 indexed `$377B` records in `secret_entry`.
   `LevelHeader::entry` copies the eight Y/X/camera bytes when progress bit 3 is
   set; `$5579` resets the route latch from that bit, so a previous item pickup
   cannot choose the next entry. `secret_route_entry_overwrites_only_a800_a807_and_resets_from_progress`
   pins the bytes, and runtime test
   `secret_route_entry_consumes_map_progress_and_reloads_stored_start` connects
   `MapSim` selection to Mario/camera initialization, object route state, and
   reload. Run `sml2-runtime --level 11 --level-progress 8 --headless 1` for
   level 12's alternate entry. The runtime still lacks a live overworld loop;
   the CLI supplies its entry progress byte. Level 19 now has played
   secret-entry approaches to both bells; other levels remain pending.
2. **All nine boss definitions now accept arena entry.** Level 6 (TreeZone5,
   scene `$0B`, definition 0) is covered by runtime
   `tree_boss_real_level_scene_entry_selects_portable_definition`; its source
   scene map and script drive `$0EB0` entry. Core
   `tree_arena_all_divider_choices_defeat_reward_map_and_reentry` covers all
   sweep states, collision-generated stomps, the appended reward, one golden
   coin, save serialization, map node `$20`/screen 2, and clean re-entry. These
   are direct-placement acceptance tests; played approaches remain pending.
3. **Most approaches remain unplayed.** Levels 28/29 header starts and level
   19's header and secret starts to both bells have native input tests. Other
   ordinary/secret starts and approaches to boss entrances still need played
   tests, including the castle/night levels.
4. **The 28 unreviewed rows are tool-measured, not hand-checked.** Their zone,
   node, exit and secret columns are trustworthy by construction; the
   "required mechanic" summary and any route narrative still need a review pass.
5. **Most time limits and music tracks are reported, not asserted.** The level
   19 header route pins time 7, music 19, and their seeded runtime values. Other
   per-level values still come from the header without explicit acceptance.
