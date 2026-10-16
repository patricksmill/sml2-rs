# Camera-pan timer: verified shared path and remaining dependencies

Ticket `camera_pan_timer` stays pending. Shared countdown, scroll/draw phases,
jump/acceleration guards, replay derivation, and renderer phase consumption are
covered by local tests. Completion requires the positive producers and remaining
flutter consumer below to run through normal gameplay.

## Source contract

- Home `$060F` calls `$0B2A`, `$2062`, then `$2D0A` before objects (`$0652`).
- `$2062` latches scroll using the current `$A2B3 & 3` even when a boss parks
  `$0B2A`. Level object draw `$C090` consumes that latched scroll.
- Player draw `$54E9` decrements a nonzero `$A2B3`, then subtracts the new low two
  bits from `$FFC4`. It does not rewrite `sMarioScreenY` or the latched scroll.
- The dead draw `$5550` skips the timer when raw screen Y is `$E0..=$FF`;
  otherwise it reaches `$54E9`. Pause/exit/death-fade bodies skip the draw.
- `$160A` skips jump state changes while the timer is nonzero. `$1701` handles
  goal-bell motion first, then gates input acceleration on this timer. Friction,
  movement integration, and the rest of the player frame still execute.
- `$1090` gates flutter on the same timer. Its fuller flutter behavior remains
  part of `moon_physics_remaining_arms`; preserve this gate when porting it.

`CameraSim` owns the timer; `PlayerSim.pan_timer` is an entry-frame input snapshot.
Both runtime loops use `CameraSim::update_player_pan` before objects, with separate
`scroll_pan` and `player_pan` values. Replay may seed the initial timer snapshot;
subsequent recorded `pan` values are assertions, never injected inputs.

## Positive producers: do not synthesize them

| Handler prerequisite | Exact write | Preconditions and associated effects |
|---|---|---|
| Wario anim 20, ROM `$A8AB` (bank `$02`, CPU `$68AB`) | `$A9EA`: `$34` | Its `$69BD` state returns from the fall at screen Y >= `$8A`; clears phase and direction counter, enters state 9, arms boss transition and sfx 24, wait `$30` |
| Same handler | `$AA60`: `$38` | Its `$6A39` state completes a picker leg and flips direction back to zero; clears phase, enters state 9, arms boss transition/sfx 24 and wait `$38` |
| Wario anim 23, ROM `$AB8B` (bank `$02`, CPU `$6B8B`) | `$AD4E`: `$38` | Its `$6D31` state finishes the wait then falls to screen Y >= `$8A`; clears phase, enters state 9, arms boss transition/sfx 24 and wait `$30` |

Use one handler ticket each (`boss_wario_anim20`, then `boss_wario_anim23`).
Generate briefs with `tools/handler_task.py --addr 0xA8AB` and `--addr 0xAB8B`;
resolve reported helper prerequisites before dispatch. Do not register a partial
handler or infer impacts solely from screen position. Connect actual handler
writes to the camera using a consumed object-result event, following the existing
fade/sound handoff pattern, and test writer -> camera -> next-frame player input.

## Acceptance

- Keep all three original preconditions, byte writes, and associated state changes.
- Preserve pre-decrement scroll versus post-decrement Mario OAM phases; a newly
  armed shake is not decremented by a frame-tail call on its creation frame.
- Test both `$34` and `$38` seeds, zero/expiry, frozen frames, visible/hidden dead
  draw, and the full input guards including flutter.
- Keep the parent ticket pending until the writers and normal gameplay paths are
  integrated; the frozen corpus records only zero for `pan` and cannot prove
  the missing Wario producers.
