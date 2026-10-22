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
| On map phase changes, the ROM moves sprites a frame before background tiles | The zone's tile updates (`$3EC7D`/`$3ECF5`/`$3ED35`) are written by an interrupt in the frame after they're computed. The port draws both from the same tick. Every animation's frames and order still match. | Mario Zone: hands vs face at counter values 31, 63 and 95 |
| A coin-room press can land on a different prize | The wheel spins from the same start value as on the ROM. Because the room arrives sooner (8 frames in recording 3), the wheel is further along when the player sees it, so a press replayed on the same recording row hits a different slot. Pressing at the same point in the room's own timeline gives the same prize. | `bde3880` |

## Checked

| Screen | Recordings | Result |
|---|---|---|
| Levels | 1–3 | Position, enemy counter, power-up and walk timer match on every row. The exception is two isolated rows in recording 3, where the same change lands one row apart. Most rows match pixel for pixel, apart from tile-animation phase. |
| Hub return fade | 1–3 | Matches: a 12-frame fade-in. |
| Coin room | 3 | Matches frame for frame, offset by the 8-frame load. |
| Title | 1–3 | Matches on every row except the first two, where the screen appears. |
| File select | 1–3 | Matches, including the drop into the pipe (`54e238f`). The opening drop runs two frames earlier than on the ROM, a load-timing difference. |
| Map frame counter | 1–3 | Matches the ROM on every map visit, apart from skipped load time and console lag (`c202f92`). |
| Level 17 | 3 | 31% of rows match exactly. Most of the rest is the animated cloud strip's phase. Mario's bunny ears now draw correctly, since his overlapping sprite cells take DMG x priority (`b1b2171`). Some rows still show a stray pixel at the left screen edge, not yet traced. |

## Not yet checked

- Zone maps other than Mario Zone, paired by counter value. Mario Zone
  (recording 1) matched in the scenery area on 99 of 105 frames paired by
  `sMapFrameCounter`. The other six are two fade-in frames and four
  phase-boundary frames, the sprite/tile difference listed above. The same
  check still needs running on the Pumpkin, Tree, Turtle and Macro maps and
  on the hub.
- Ending and Wario's castle: no recording covers them yet (waits on
  `f-wario-played`).
