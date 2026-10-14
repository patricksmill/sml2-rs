# Replay captures

Live PyBoy captures used as the regression suite for the port. Run:

```sh
cargo run -p sml2-runtime -- --level 0 --replay captures/trajN.json
```

Every capture replays with `--level 0` — the CLI index is 0-based while the
level binaries are `level{index+1}.bin` (`--level 0` = `levels/level01.bin`).
`traj19` is the one exception: it is the **level-20** capture and takes
`--level 19` (the only per-capture level override in `sweep.sh`). Passing the
wrong index silently loads a different layout and the replay diverges from
frame 1 (a trap hit once).

Every capture must replay with **all-zero** mismatches (`mismatches: ... =0`), and
every file in this table does. `traj19` is the one capture that does not replay at
`--level 0` — it is the level-20 capture, so it takes `--level 19`, the only
per-capture level override in `sweep.sh` (which lists it, so it is in the gate).

| capture | frames | checks | what it pins |
| :--- | ---: | :--- | :--- |
| `traj4.json` | 725 | 19 legacy | Legacy spawner capture (no `ff0`/`uphase`/raw slots). Diverges from frame ~231 on the sprite-toggle phase (missing `$FFF0`) and at frame 291 where Mario dies to the walker. Use `traj4_ff0.json` instead. |
| `traj4_ff0.json` | 725 | 19 legacy | `traj4` + `ff0: 3` on frame 0. Replays clean to **frame 291** (Mario's death to the walker's side contact — the death sequence `$31D1` is not modelled), then the unported anim-1 handler (`$582D1`). |
| `traj5.json` | 381 | 19 | The spawner: scan direction/edge quirks, stream pointer, slot table, phase parity. No object ever spawns (the camera stays left of the walker). |
| `traj6.json` | 468 | 28 | The full object milestone: walker spawns (f207) → activates (f218) → walks 472→443 → Mario stomps it (f337, anim 9→10) → squashed countdown → removed (f401, coin credited). Checks every byte of all 16 slots + `sMoveFlags`/`sObjectFacing`/`sMarioDead`/`sStompBounceTimer`/`sStompChainCount`/`sMoneyBagCount`/`$FFF0`/`sObjectUpdatePhase` + player/camera. |
| `traj7.json` | 838 | 28 | The anim-1/anim-2/anim-17 handlers: walker stomp, then record 1 (anim 1, type `$56`) spawns f394 → activates f406 → falls/lands → paces its 2-tile platform 575↔608 with ledge reverses at f444/f576/f708 (sprite bob 4/5/6); record 2 (anim 17) spawns f459/activates f470 and poses as an item (type `$80`). |
| `traj8.json` | 2126 | 29 | Pipes: stomp walker 1, climb the block staircase (four drift-timed hops), cross the raised floor (walker 2 falls off its edge — the walker's fall states), climb the `$08/$09` pipe at x=1136 (arming on landing, 7-px X-window entry at x=1141 → dir `$10`), 38 travel frames descending 420→542 (spin frame 16). Trimmed at frame 2127, the transition trigger — the pre-transition regression; `traj9` covers the transition itself. Adds the `pipe` check. |
| `death1.json` | 537 | 29 | The death sequence, sampled per LOGIC frame (hooked at `$05E4` — the dead branch lag-frames, so tick-based sampling merges frames; see `death_capture1.py` + `apply_pairing.py`). Walk → harmful contact (dead 15) → dispatch (dead `$FF`, vvel 95) → the arc (`MarioSpeedTableDead`, 8-bit y with the lo wrap 510→257, vvel floored at 1, frame 107) → the fade. Adds the `pcx` check. |
| `death2.json` | 838 | 30 | The FULL death cycle (`death_capture2.py`): `death1`'s sequence, then the fade (192 frames — the model is checked against the captured `$FF9B` duration) → `sLives` 05→04 (BCD) → the map/reload gap → respawn at the header spawn (34/468; camera 80/448; `$0F2A` hvel $80/vvel 24; `$EC40` clears the object scratch, `$FFF0` persists, frame 107 persists) → the walker re-spawns from the reloaded stream (standby f807, active f823) → walk. The first post-gap record carries `gap: {total, seq}` — the state machine between the fade trigger and re-entry, polled from `$FF9B`; the input-dependent map (136 of 329 ticks) is skipped, the fade + reload state are modelled and checked. Adds the `lives` check + the `gap` line. |
| `traj9.json` | 2349 | 29 | The screen transition: `traj8`'s run continued past frame 2127 — the `$C0`-row trigger, 34 `$0DED` scroll frames (camera +4/frame, Mario +1/+2), the finish at camera lo `$48`, the resumed travel ending dir 0, the fall + landing at y=612 in the underground, and the post-transition camera catch-up. No trim. |
| `traj10.json` | 5130 | 30 | The long haul: `traj9`'s run continued past x=1640 — the plateau, the arena crossing (jump1 from x≈2032 over pipe-1/bobber-1 onto column-2, jump2 over bobber-2), the running ledge-launch east of pipe-2 (`$129F`: hvel ≥ `$C0` + no floor + was grounded → vvel 28; f3555), the walker stomp at x≈2801 (the `$2DC6` bounce whose `$1DFE` ceiling probe reads the *shifted* Y — f4603), the hopper/item pair, the anim-23 ledge, the end zone (final mx=3094, my=468). Also the only capture that exercises the `$FF97` dither cadence over 5k frames and the `$129F` launch. |
| `traj11.json` | 6470 | 30 | `traj10`'s run continued past x=3094: the anim-15 marker (x=3192, type `$85` — a static no-op), the walker at 3352 (jumped over, not stomped — a straight-up stomp at dx=−19 is a side contact), and the anim-12 bobber at 3456, crossed by parking **inside** its 24 px `$5B4B4` trigger radius but **outside** its 21 px box until it cycles into state 0 (type `$80` = harmless item); then east to x=3586, just west of the 496/480/464 tower at x=3616. |

| `traj12.json` | 6840 | 30 | `traj11`'s run continued past x=3586: the 496/480/464 tower (3616-3807, three solid tiers plus 432) climbed with running hops (18-frame run-up — the jump only clears the 33 px `$81` spike row at 3696-3727 at run speed; an earlier attempt clipped it at x=3709 and the console killed Mario exactly where the new `$1BFF` hazard probe predicts), the fallwalker at 3744 (flown over), then a **walk** east to x=4026, which is where the anim-22 pose-86 object (x=4008; spawns f6606, goes active f6616) is exercised. The walk is deliberate: hopping through 3872-3887 head-bumps the `$7F` bumpable block at row 26 and the console answers with the `$5B06` block-bump pose (`$FFC6 = 66`, drawn at the block's position) — bounce blocks were the port's next target at the time (ported and verified in `traj15`, 2026-10-13) — and stopping short also keeps Mario's floor probe off the exit tile at 4064. |

| `death3.json` | 5807 | 25 | **The `$1BFF` hazard death — the negative control for the probe.** `death_capture3.py` replays the whole level-0 route (`trajectory_capture12.py`'s) and then, instead of hopping the tower's spike gap, walks east off the end of the 448 tier: row 28 has no cell at 3696-3727, so Mario drops onto the `$81` spike cells and the probe at `(x+16, y+28)` kills him (level 1's only hazard run; `tile & $FE == $50`). Sampled per LOGIC frame (`$05E4` hook) + `apply_pairing.py death3_raw.json death3.json`. This capture is what proved a hazard hit dispatches in the **same** frame (alive f5685 → `dead = $FF`, vvel `$5F` at f5686) while an object contact takes one more frame — the port had it the other way and this capture caught it. |

| `traj13.json` | 4645 | 27 | **Item pickup** (`$2DC6`'s bit-7 branch + the `$3451` consumer). Route: `trajectory_capture12.py`'s prefix to the hopper pair, then hops eastward with growing run-ups until one crosses the anim-23 pose87 at (2792, 416) — the object writes type `$87`, so the touch stages item id 7 and the next frame's `$3451` sets `sSecretRouteFlag` ($A2A0) = `$FF`, which the capture records and the replay checks (`secret`). The same capture carries the other consumer observables (`pow`/`lives`/`inv`/`goal`), all zero through the run. **Trimmed at record 4645**, the frame before the route's tail: `traj14` carries that tail (see below). TICK-SAMPLED, so its records are only 1:1 with the console's logic frames while no frame overruns its VBlank slot (see `probe_frame_cadence.py`). |

| `traj14.json` | 4718 | 31 | **The same route as `traj13`, sampled per LOGIC frame — `traj13`'s replacement for anything that moves.** `trajectory_capture14.py` runs `trajectory_capture13.py`'s route verbatim — the script prints its tick count, **4719**, as the check that it is the same console run as `traj13_full`. Its records come from a `$05E4` hook instead of the tick loop, so a lag frame yields two records and a stall none. The tail `traj13` was trimmed before: the pickup, the coast east, the drop onto the 2880 ledge (row 28), the record-16 stream spawn at x=3016 and the record-14 fallwalker's harmful contact at ~x2920 → the death dispatch and 55 arc frames. Replays all-zero on all 31 checks (`pcx` included) — the checks `traj13_full` could not reach because its tick sampling sampled *mid-frame* there. |

| `traj15.json` | 818 | 35 | **Brick bumps ("bounce blocks") — the `$1993`/`$5960`/`$5997` system, and the level-map rewrite a bump performs.** `trajectory_capture15.py` walks east on the row-31 floor to the `$01` block at col 22 (x 352-367; the ceiling probe `(x+16, y+28)` wants Mario at x ∈ [336,351]), jumps three times, then runs into the first walker (a harmful side contact → the death dispatch and arc). The first bonk is the richest case in the level: `$19D2` sets `sSpinJumpBounce` = `$FF`, allocates a **type 3** slot (the `$5B06` bump) and then `$2C77`/`$591F` credits +1 coin and allocates the **type 4** coin slot, whose `$5AF9` cycle (109/110/111) draws last, so it wins `$FFC6`. The bump's `$5CF1` also writes the **level map**: cell (28,22) goes `$01` → `$24` (timer 2) → `$07` (timer 8) — so the second and third jumps bonk `$07`, a `$04`+ tile: a plain bonk that clears the spin flag and credits nothing (the console's `frame` stays 40 and `spin` stays 0). Adds the `spin` ($A220), `coins`/`coinshi` ($A262/$A263 BCD) and `bslots` (the 8 `sBlockAnimSlots` at $AA80, 16 bytes each — including the stale bytes a freed slot keeps) checks. Sampled per LOGIC frame (`$05E4` hook) + `apply_pairing.py traj15_raw.json traj15.json`. |
| `traj16.json` | 6608 | 33 | **The coin touch (`$1D75`/`$1DA5`), called from the `$1E7C`/`$1E8D` side probes.** `trajectory_capture16.py` runs `trajectory_capture12.py`'s route — its first 6522 per-tick records are field-for-field identical, checked in the probe run — and stops at the tower-climb exit (x ≥ 3800). Climbing the 496/480/464 tower, the side probes read level 1's only `$48` cells — (24,231) and (24,232), in the air at x 3688-3719 / y 384-415 — and the console collects both: `sCoinLow` goes `00` → `01` → `02` at the frames whose state is (3691, 383) / (3700, 365), and the two level-map cells are rewritten `$48` → `$60` (`$1DA5` → `$2C0D` → bank-1 `$5CF2`), which `probe_coin_touch.py` measured tick by tick. Adds the `coins`/`coinshi` checks ($A262/$A263, BCD) — the port's `PlayerSim::coin_touch` fires at exactly those frames and writes exactly those cells. Sampled per LOGIC frame (`$05E4` hook) + `apply_pairing.py traj16_raw.json traj16.json`. |
| `traj17.json` | 6977 | 38 | **The `$1BFF` exit-tile arm (`$1CBA`) — the level-complete sequence, console-verified for the first time.** `trajectory_capture17.py` runs `trajectory_capture16.py`'s route (the whole level: stomps, pipe room, arena, the 496/480/464 tower) and then `trajectory_capture12.py`'s deliberate WALK east continued past traj12's stop at 4026 to level 1's only `$4A` cell — the bell at col 254 (world x 4064). `$1BFF`'s `(x+16, y+28)` sample reads it as Mario reaches x=4057 and the arm fires at that frame: `sCollisionTile = $4A`, `sMusicRequest = 3`, `sLowTimeFlag = 0`, `sExitTileFlag = 0`, `$FF9B = 24` (state `$18`) and `sFadeTimer = 192`. The records then carry the whole 192-frame fade (state `$18` for 191 further ticks) and the `$3547` tail's BCD tally charge `00` → `01`, ending on the `$14` hand-off frame. **Two hooks**: `$05E4` (state `$04`'s body) stops firing the moment the state leaves 4, so the frames after the arm come from a second hook at `$028D` (the main loop's post-state point, one fire per frame in EVERY state) spliced onto the same record list — the two sample the same frame boundary, so one pairing covers both. Adds the per-frame `exit` (`sExitTileFlag` $A2B4), `lowtime` (`sLowTimeFlag` $A53F), `fade` (`sFadeTimerLow/High` $A266/$A267) and `tally` (`sLevelTally` $A2C5) checks plus the one-shot `exit_arm` block (the trigger's geometry, the fade span measured from outside the hook, the tally) which the harness reports on its `exit arm:` line and fails with a `note:` if it disagrees. Sampled per LOGIC frame (`$05E4` + `$028D`) + `apply_pairing.py traj17_raw.json traj17.json`. |
| `traj19.json` | 1309 | 33 | **The first capture on a level other than level 1 — level 20 (index 19, "Star Course"), reached with `probe_level_select.py`'s node-61 poke route — and the gate for the `$0DAF` autoscroll path.** Inputless by design (the autoscroll drags Mario along the left edge, `msx` 16-33, x 63→699, y 212), cut at the anim-95 contact frame *before* the death, no second hook needed (`$FF9B` = 4 and `dead` = 0 on every record). Replays **all-zero at `--level 19`** across 1308 compared frames — `camx`/`camy`/`edge`/`en_r`/`en_l` included, with the camera delta +1 on exactly 654 frames (1 px per 2 logic frames, parity off `$FF97 & 1`) — and it records `autoscroll` = 1 plus `moon` (`sMoonPhysics` `$A287`) = 1 on every frame, independent confirmation of level 20's property bytes. It earned its keep before it was green: it exposed **two** real gaps in one run — `x` diverged by a persistent +1 on exactly the frames the camera steps (root-caused to `run_replay` injecting the record's *post*-frame camera into the clamp instead of latching the sim's own camera as `run_play` did; the fix is measured neutral on all 18 level-1 captures) and the object engine diverged once anims 94/95 activated (`note: unported bank-16 handler path(s) hit: $5B241, $5B28C`, now ported and unit-tested). **No scene latch**: `sSceneId` is 0 on all 1309 records — level 20's `$0FBF` clamp holds `sMarioScreenX` below the `$A1` the right-edge trigger needs, so a scene needs a different level (see the queue's `scene_capture`). |

| `traj13_full.json` (RETIRED) | 4719 | — | **RETIRED** — not shipped in this repo. Tick-sampled mid-frame artifact; `traj14` supersedes it. |

Capture scripts (`trajectory_capture5.py` … `trajectory_capture17.py`,
`trajectory_capture20.py`, `scene_capture1.py`,
`death_capture1.py`…`death_capture3.py`) need a PyBoy venv
(`pip install pyboy pillow`) and `baserom.gb` from the sibling decomp
(`SML2_DECOMP_ROOT`, default `../sml2decomp`); they drive the title → file
select → hub → level-0 path, then record per frame. Record the raw 32 slot
bytes (`slot_bytes`) and `$FFF0`/`$AF08` — see DESIGN.md. Since `traj14` every
new capture records its fields in a `$05E4` hook (one entry per logic frame) and
is passed through `apply_pairing.py`; `trajectory_capture15.py` adds the
brick-bump observables (`spin`, `coins`/`coinshi`, `bslots`) and
`trajectory_capture16.py` the coin-touch ones (`coins`/`coinshi` again — the
`$1D75` BCD credit), which the runtime checks only when present, so the older
captures keep their check count.

**Sampling method, and why it matters.** v5-v13 sample once per PyBoy tick
(`tick(); frames.append(state(keys))`): that is 1:1 with the console's logic
frames only while every frame fits its VBlank slot. A heavy frame (a stream
spawn) overruns, the main loop drifts against the tick boundary, and a tick can
then sample *mid-frame* (before the update's tail) or catch the same frame
twice — the record list stops matching logic frames and no port can replay it.
`death_capture1-3` and `trajectory_capture14.py` instead take the records in a
`$05E4` hook (the in-level frame entry), which is 1:1 by construction; the hook
fires at the frame START, so run `apply_pairing.py <raw> <out>` afterwards (the
sim's frame i replays the console's logic frame i-1 with that frame's keys).
Prefer the hook method for anything new.

**When the feature ENDS the level, `$05E4` stops firing.** It is the state-`$04`
body's entry, so the first frame whose state is no longer 4 produces no record —
which is exactly where a level-clear's fade and tally live. `traj17` handles
that with a SECOND hook at `$028D` (the main loop's post-state point, after the
state handler returns and before `$02FF`'s VBlank wait): one fire per frame in
every state. Both hooks sample the same frame boundary — a `$05E4` sample is the
start of its frame, a `$028D` sample is the end of the frame the same record
list already treats as "one frame earlier" — so the two lists splice into one
stream that the ordinary pairing covers unchanged. (The one asymmetry: VBlank
writes — the fade timer — land between the two instants, so a `$028D` sample
shows the pre-decrement value the sim's own post-step state also holds.)

The venv used since 2026-10-12 is `/home/pmill/.venvs/sml2-pyboy/bin/python`
(durable; an earlier one under the scratch dir was pruned). The scripts write
their JSON to `$OUT` (the scratch dir, pruned after 24 h idle), so copy the
result into `captures/` immediately — the tree copy is the authoritative one.

`probe_c6_selector.py` is the diagnostic for a divergence in the `frame`
check: it hooks `$530D` with `p.hook_register(1, 0x530D, cb, None)` (bank 1 —
`home.asm`'s `$2D0A` calls it) and logs the selector's inputs (on-ground,
on-object, invincibility, powerup, hvel, walk timer, the previous `$FFC6`,
spin/moon/flutter/stomp timers) at every logic frame, which is what told apart
"the selector used a state the capture doesn't record" from "another routine
wrote `$FFC6`" — the answer was the latter (`$5B06`'s block bump). It writes
`traj12_probe.json`, never the real capture.

`probe_block_bump.py` is the diagnostic for the brick-bump system: it walks
Mario to the `$01` block at col 22, jumps, and logs the `$FFC6`/`sBlockAnimTimer`/
`$A220`/`$A262` track plus a hook on `$15CF` (the bonk entry) and `$1993` (the
classification) that records the probe point (`$FFB7`/`$FFB9`), the tile read
and `sCollisionTile`. That is where the two slots and the `$01` → `$24` → `$07`
level-map write come from: the probe lands on `(360, 479)`, cell (28,22). It
never writes `traj15.json`.

It also logs the staged sprite frame (`$FFC4`/`$FFC5`/`$FFC6`) and the camera
every frame, which is what pins the bump's **draw position**: at the coin slot's
timer-2 frame the console stages `$FFC4`/`$FFC5` = 80/71 for the slot's
(472, 368) with camera (448, 377); `PlayerSim::block_draw_slot`'s anchors
(`slotX - 16`, `slotY + offset - 26`) compute the same 80/71, and the filmstrip
(`--replay captures/traj15.json --filmstrip DIR`) shows the block bump drawn on
the block and the coin rising above it, with the HUD coin digits reading 001.

`probe_coin_touch.py` is the diagnostic for the coin touch (`$1D75`/`$1DA5`): it
re-runs `trajectory_capture12.py`'s route tick-sampled and logs `sCoinLow`/
`sCoinHigh` ($A262/$A263), `sCollisionTile` and the level-map bytes at (24,230)
… (24,233) every tick (SVBK noted — the four `$B000`-region cells read back from
the selected WRAM bank). It is where the console side of `traj16` was measured:
the counter changes at the ticks whose end state is (3691, 383) / (3700, 365),
and cells (24,231)/(24,232) go `$48` → `$60` on those same ticks — i.e. `$1DA5`
writes the probe's own cell, before the counter reads in `$1D75`'s tail. It
writes `$OUT/probe_coin_touch.json`, never a capture for the suite.

`probe_frame_cadence.py` is the diagnostic for "the replay is off by a frame
somewhere": it re-runs `trajectory_capture13.py`'s route with cumulative call
counters on `$05E4` (the in-level frame entry), `$0F71`, `$8000` (the object
engine, the frame's last call), `$8C31` (the stream scan) and `$8C8D` (the
spawn), so the deltas between two records say exactly which of them ran in that
tick. That is how `traj13_full`'s tail was pinned to the sampling rather than to
physics: ticks 4651/4652 straddle the spawn frame (4652 has no `$05E4` entry at
all), and the whole tail replays all-zero once those records are dropped.

It needs `baserom.gb.sym` next to the ROM (`rgblink -n sml2.sym -o /dev/null
main.o` in the repo root, then `cp sml2.sym baserom.gb.sym`) because a raw
bank-2 hook (`p.hook_register(2, 0x8000, ...)`) trips PyBoy's VRAM-bank path
("bank of VRAM is only supported for CGB mode"); `p.hook_register(None,
'UnknownCall_0x8000', ...)` resolves the bus address from the symbol file
instead. `trajectory_capture14.py` keeps the hook at `$05E4` (bank 0), which
registers fine raw.

`probe_below_hit.py` is the diagnostic for the `$86A0` below-hit dispatch
(`$2F72`'s recipient). No route in the suite bonks a brick that has an object
on it, so it does not use a route at all: it boots level 0 and pokes a free
slot ($AD00) into an active type-`$86` pose — anim 22, a bogus sprite `$7F`, a
live record pointer (`$AB06`), `+23` = `$20` (exactly what `$2F72` writes) —
then logs the 32 slot bytes per logic frame. Measured: the sprite re-derives
`$7F` → `$1F` (the anim-22 map entry), the *type survives* the `$8240` clear
that consumes the flag and wipes `+14..+30`, `+27` becomes 2, the next frame's
`$58101` faces Mario (`+15` 1, `+13` `$20`) and the knockback then runs
`(100,468)` → `(101,465)` → `(102,462)` on the slot-parity cadence. The port's
`objects.rs` test `stepped_below_hit_on_the_bell_re_derives_and_faces_mario`
replays the same sequence and asserts those bytes. It writes
`$OUT/probe_below_hit.json`, never a capture for the suite.

`probe_bell_checkpoint.py` is the diagnostic for the "the midpoint bell saves
the respawn coordinates" question — and it answers **no**. It boots level 0
(capture6's prefix) and pokes `sPendingItemId` ($A26C) = 6, which is exactly
what `$2DC6`'s bit-7 branch stages when a type-`$86` object (the goal bell) is
touched, then logs `sGoalBell` ($A24F), vvel and the level-start block
($A800-$A807) per logic frame. Measured: the poke is consumed on the next
frame (`sGoalBell` 00 → 0C, the id clears, vvel's `$34EC` lift) while
`$A800-$A803` keep the header's spawn (34, 468) through 119 frames and ~85 px
of walking — `$34EC` never writes them, and `$0F2A` only reads them on the
death reload. `$A800-$A807`'s other writer is the secret-route overlay
(`$55C0`, the per-level records at `UnknownData_0x377B`), not the bell. It
writes `$OUT/probe_bell_checkpoint.json`, never a capture for the suite.

`probe_level_select.py` is the **route every non-level-1 capture needs**, and a
diagnostic in its own right — no other script here can reach a level other than
level 1. On the overworld map (state `$0C`) it pokes `sMapScreenId` (`$A68B`) to
any nonzero value (the `$3E1D7` entry gate needs screen != 0) and `sMapNode`
(`$A840`) to the target node, then taps A. The console then runs its own entry
path — state `$0C` → `$0B` → `$2` → `$3` → `$4` — setting
`sCurLevel = $4218[sMapNode]` at `$3C1A2` (main.asm:18544; not `$3D0D7`, which
is a castle node's animation) and loading the real level header. Measured on two
targets, first state-4 frame: node 0 → level 1 with Mario (34, 468) / camera
(80, 448), and node 61 → level 20 with Mario (63, 212) / camera (96, 192) — the
`level01`/`level20` manifest headers exactly; 60 frames later on level 20 the
camera has moved +30 px, i.e. the `$0DAF` autoscroll rate is live. `node_for_level(idx)`
reads the node index out of `extracted/map/nodes.bin` (`$4218[node]` = level
index, `$FE` = a pure path node); `enter_level(p, tick, tap, rd, node)` does the
pokes and waits for state 4. Level 25 (Wario's Castle) has **no** map node and
needs the castle path. The map state is *poked*, not driven, so record from
state 4 on, exactly as the other capture scripts do — the pokes only choose which
level, they do not touch it. Run it with no arguments to re-verify both targets.
