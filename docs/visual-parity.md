# Visual parity: accepted differences

`f-visual-parity` compares the ROM (PyBoy recordings) and the port drawing the
same game state. `tools/compare_tas.py --pixel-report` pairs ROM frame N with
port frame N-1, and `--debug-state` prints Mario's position, the HUD enemy
counter, power-up and walk timer side by side.

Every difference that is left is one of those listed below. Anything else is a
bug.

## Accepted

| Difference | Why | Reference |
|---|---|---|
| The port shows each tick one frame earlier | The console draws from the previous frame's VBlank shadows. The native renderer draws the tick that produced the frame. | DESIGN.md, standing goal 2 |
| Animated tiles (water, sea, ground) run at a different point in their cycle | The tile clock's absolute value includes loads and console lag the port doesn't model. Each animation keeps the ROM's period, frames and order. | [frame-clock.md](frame-clock.md) |
| Screens arrive a few frames sooner | The port skips the title, file select → hub, level → map and door loads (3–33 frames each). | [frame-clock.md](frame-clock.md) |
| Map zone sprites show the previous frame's animation step | Pairing by `sMapFrameCounter`, Pumpkin Zone's witch and flames match the ROM image one counter value earlier. Mario Zone's background-tile patches match on the same value. The port draws everything from the current tick. | Pumpkin Zone: 86 of 87 frames exact in one visit, 81 of 141 in another, at that offset |
| On map phase changes, the ROM moves sprites a frame before background tiles | The zone's tile updates (`$3EC7D`/`$3ECF5`/`$3ED35`) are written by an interrupt in the frame after they're computed. The port draws both from the same tick. Every animation's frames and order still match. | Mario Zone: hands vs face at counter values 31, 63 and 95 |
| Map random draws (Tree Zone's bird) differ in comparisons | The comparison feeds the port the ROM's DIV reads for the same recording row. The port reaches each screen sooner, so a reroll of `sMapRandLatch` on the same game frame reads a different DIV value. | `docs/randomness.md` |
| `sMapTickTimer`'s phase relative to a fresh zone | The timer runs continuously across loads, while zone and room loaders zero the frame counter. After a chain of skipped loads (castle reveal, coin rooms, doors) its phase drifts like the tile-animation clock's. Tree Zone's bee steps on odd values, so it can land a step apart. After a level, the timer comes from the level's byte and matches. | Recording 3, Tree Zone at row 73740 |
| A coin-room press can land on a different prize | The wheel spins from the same start value as on the ROM. Because the room arrives sooner (8 frames in recording 3), the wheel is further along when the player sees it, so a press replayed on the same recording row hits a different slot. Pressing at the same point in the room's own timeline gives the same prize. | `bde3880` |

## Checked

| Screen | Recordings | Result |
|---|---|---|
| Levels | 1–3 | Position, enemy counter, power-up and walk timer match on every row. The exception is two isolated rows in recording 3, where the same change lands one row apart. Most rows match pixel for pixel, apart from tile-animation phase. |
| Hub return fade | 1–3 | Matches: a 12-frame fade-in. |
| Coin room | 3 | Matches frame for frame, offset by the 8-frame load. |
| Title | 1–3 | Matches on every row except the first two, where the screen appears. |
| File select | 1–3 | Matches, including the drop into the pipe (`54e238f`). The opening drop runs two frames earlier than on the ROM, a load-timing difference. |
| Map animation bytes | 1–3 | `sMapAnimPhase`, `sMapAnimIndexA`, the witch position, `sMapTickTimer` and `sMapRasterPhase` match the ROM's when paired by frame counter, on every zone visit and every hub visit (`8907f82`). Exceptions are the hub after file select, whose load and fade-in aren't modelled, and door reloads, which arrive 4 frames sooner. |
| Hub sea | 1 | After a bell room's return, the wave phase matches row for row. The sea's pixels match apart from a few single-pixel rows. |
| Tree Zone | 3 | After a level, counter-paired frames match on 40 of 51 and 75 of 130 (`sMapTreeAnimTick` fix). The rest are the bird's random frame and Mario's idle frame staged a frame apart from the zone sprites. |
| Coin-room wheels (screens 10, 11) | 3 | Match before the press. After it, the press lands at a different point in the spin (the accepted early arrival), and the prize, hand, Mario and lives follow. |
| Map frame counter | 1–3 | Matches the ROM on every map visit, apart from skipped load time and console lag (`c202f92`). |
| Level 17 | 3 | 31% of rows match exactly. Most of the rest is the animated cloud strip's phase. Mario's bunny ears now draw correctly, since his overlapping sprite cells take DMG x priority (`b1b2171`). Some rows still show a stray pixel at the left screen edge, not yet traced. |

## Not yet checked

- Wario's Castle keeps its own copy of `sMapRasterPhase`
  (`WarioCastleAnimation::raster_phase`), separate from the hub's. On the
  ROM they're the same byte, so its bands should continue the hub's phase.
  No recording covers it.
- Level 17's stray left-edge pixel.
- Ending and Wario's castle: no recording covers them yet (waits on
  `f-wario-played`).
