# Completion and route evidence

Pending work is authoritative in `PORT_QUEUE.json`, not historical status tables:

```sh
python3 -c 'import json; print([(e["id"], e["title"]) for e in json.load(open("PORT_QUEUE.json"))["queue"]])'
```

Close shared subsystem dependencies before their handlers, then dynamic spawn
closure, scene integration, played routes, visual/audio comparisons and final
progression. Implementation, source verification and normal-play completion
are independent claims. Follow [PORTING.md](../PORTING.md).

## Source matrix

Measured 2026-10-18. Regenerate using `python3 tools/route_matrix.py --table`;
the tool reads manifests, layouts, scene maps, map nodes and the ROM's `$377B`
secret-entry overlay. Level numbers below are 1-based; runtime `--level` IDs are
zero-based. Coordinates are layout cells; secret entries are world pixels.
Mechanic flags describe source data, not proof that a route has been played.

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

There are 35 bell cells across 25 levels (28 plain $4A, 7 alternate $4B), including
stacked two-cell bells in levels 26/27/31. Boss levels 6/10/14/17/20/24 and Wario 25
have no bell. Wario has no ordinary map node. Level 32's secret-entry record runs
into padding/code and is not a usable start. Autoscroll: 20/26/31; timed float:
19/29; hold-A float:20. Preserve all 32 indexed secret records at extraction,
even zero/overlapping ones; do not infer valid starts from nonzero bytes alone.

## What has been verified

| Coverage | Evidence / limit |
|---|---|
| Declared handlers | Coverage tool inventories both level tables and boss table; declaration is not behavior parity |
| Dynamic anim closure | 2026-10-18 audit: 477 level/anim pairs, 106 anim values, no out-of-range/unported targets. Includes spawns, morphs, record rewrites and boss writes; over-approximated reachability, not behavior |
| Every exit cell | Core `route_matrix` tests activate all 35 source $4A/$4B cells and assert exit flag, music and 192-frame fade |
| Map entry | Runtime session cases cover 64 header/secret-entry combinations at 32 ordinary origins plus Wario castle-door entry |
| Reviewed ordinary rows | Levels 1–5,12,19,28,29; other mechanic/route narratives need review |
| Played ordinary routes | Level 28 walk, 29 timed float, and 19 header/secret starts to both bells. Fixed inputs assert movement, float use, fade/handoff/tally/save bytes |
| Continuous map/save routes | Six approaches across 19/28/29 run actual map selection, completion, return, host save and reload |
| Boss routes | Levels 6/10/14 have played header approaches through defeat/reward/map return. All nine definitions accept scene-driven arena entry; direct placement is not a played approach |
| Graphics | [Visual parity](visual-parity.md) records independent evidence and accepted differences; smoke rendering proves neither |
| Remaining completion | Other normal/secret approaches, bosses 17/20/24, Wario's full route, continuous hub navigation, audio evidence and uninterrupted full-game save/reload remain queue work |

`PlayTape::Buttons` supports simultaneous held buttons through the same live
loop; CLI `--keys` keeps its ASCII grammar. Tests compare both representations.
Neither a table count nor a headless smoke budget establishes level completion.
Re-run current tools/tests before relying on historical counts:

```sh
cargo test -p sml2-core route_matrix
cargo test -p sml2-runtime played_routes
python3 tools/handler_coverage.py
python3 tools/anim_closure.py --self-test
```

Audio acceptance needs register-write/PCM evidence; final integration needs fresh
extraction/build and a continuous new-game -> six coins -> Wario -> credits run
with mid-game save/reload. The frozen sweep remains regression evidence only.
