# deleg_A — record-field timing audit (`record_field_timing_audit`)

READ-ONLY audit of every record field `run_replay` reads, per the PORT_QUEUE item `record_field_timing_audit` (filed 2026-10-14). No source, test, capture, DESIGN.md or PORT_QUEUE.json was modified; the only write is this note. The gate suite was NOT re-run by me (a parallel writer was mid-sweep at the pinned commit — see §4); every verdict below is read-derived, and the one suspect field is corpus-inert by measurement (pan ≡ 0 on every record of all 19 gate captures, verified below).

## 0. Snapshot and the pairing convention

Pinned: repo `/home/pmill/sml2-rs`, HEAD `7ecf2afd819dc60d10a72588d90a60512c7d0577` (Gate: let a parallel writer run the sweep against its own build). The repo was LIVE during the audit — `crates/sml2-runtime/src/main.rs` grew 2324 → 2386 lines while I was reading it. Every file:line below is pinned to these md5s; re-check line numbers if anything lands after this commit:

| file | md5 | lines |
| :--- | :--- | ---: |
| `crates/sml2-runtime/src/main.rs` | `aa4a62e52d202c0a8191af13e9cda223` | 2386 |
| `crates/sml2-core/src/player_sim.rs` | `4794ed4ba7b64233672298689d717f11` | 4234 |
| `crates/sml2-core/src/camera.rs` | `45855b9561174b40f88402532b9b2b8d` | 1244 |
| `crates/sml2-core/src/objects.rs` | `945f77f4d271d83693e0a0725c02cf07` | 3324 |

**The convention** (quoted from the pairing tool, `captures/apply_pairing.py:4-10`): “The hook fires at the START of each logic frame, so raw record k holds the state at the END of frame k-1 together with frame k’s held keys. The sim’s frame i replays the console’s logic frame i-1, so it needs THAT frame’s keys paired with the record-i state.” And `apply_pairing.py:15-21`: the per-frame INPUTS “read by the console *during* logic frame k” — the held keys and `phase` ($FF97) — are shifted from record i-1; `edge` ($FF98) is NOT shifted, because it is “the camera’s own lookup at the frame’s start … its position is the record-i state”.

So for harness iteration i (replaying console frame F = i-1), record i carries each console register’s value **at the END of frame F**, except keys/`phase`, which the pairing shifts to the value frame F read. A register the console writes mid-frame therefore arrives **one write ahead** of what F’s early readers read — the class the settled camx fix describes (PORT_QUEUE `autoscroll_clamp_x_divergence`): “A record is sampled at the `$05E4` hook = the frame’s START, so record k’s camx is the camera AFTER the frame that invocation replays”. The harness loop body says the same (main.rs:1240-1251).

**Frame order (console).** The in-level body `$05E4` (home.asm:779-812): `$2C61` → latch Y/X → (`sScreenTransition != 0` skips the player update) → `$0F71` → `$060F` tail: `$0879`, `$0B2A`, `$2062`, `$2D0A`, `$2C82`, `$2D7D` (objects) → `$3753` (moon FX) → `$2A58`, `$0652`, `$2CFF`, `$273E`. `$0F71`’s call list (notes/level20_handlers_and_moon.md §2.2, from home.asm:2101-2124): `$307D` crouch pan, `$13FE`, `$0FF2`, `$1BFF` probe/hazard, `$129F` landing, `$1413` push-out, `$14A6`, `$126F` spin, `$160A` jump, `$1090`, `$14FA` gravity/integrator, `$1701` accel, `$16CC` friction, `$181C` horizontal integration, `$0FBF` screen-edge wrap, `$14BF` walk timer, `$1189` pipe, `$319F` dispatch, `$3451` item consumer, `$32C1/$331C/$324F`. Frame TOP (main loop, home.asm:250-280 and 345-356): halt → `$0300` waits `$FF82` then `inc $FF97` (home.asm:349-351) → `ReadJoypad` (`$1FD2`, home.asm:4130; called home.asm:259) → state dispatch `$02AD` (→ `$05E4`) → `$20A4` scene VM → loop.

**Port loop order** (main.rs:1205-1385): keys (1206-1217) → `a_pressed` (1218) → phase (1229-1234) → frame_counter (1239) → `sim.camx/camy = cam.x/cam.y` (1252-1253) → dead/fade branches (1282-1295) → `crouch_pan` (1301) → `sim.step` (1302) → stomp-chain reset (1308) → below-bonk scroll with `f.pan` (1322) → `cam.edge_now` (1327) → `cam.step` (1328) → `update_enables` (1329) → `build_contact` with `f.pan` (1336) → frame selection (1339-1354) → `obj.step` (1359) → `apply_object_result` (1362) → scene VM (1368) → gap (1381) → the check block (1490-1830).

## 1. The table

Columns: field | injected-at (file:line) | reader(s) (file:line + ROM addr) | reader’s frame position vs the writer | run_play source | verdict. Verdict tokens: `derived` / `injected-and-safe` / `injected-and-suspect`.

### 1a. Injected per frame (every iteration)

| field | injected-at (file:line) | reader(s) (file:line + ROM addr) | reader’s frame position vs the writer | run_play source | verdict |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `right`/`left`/`up`/`down`/`a`/`b` (held keys) | main.rs:1207-1212 (`PlayerKeys`) | `$307D` byte test (camera.rs:180; home.asm:6531-6543, `cp $82`/`cp $42`), `$1701` (player_sim.rs:1480-1507), `$160A` (player_sim.rs:1414-1427) — all inside `$0F71` (home.asm:799) | input sampled once per frame at the frame TOP: `ReadJoypad` `$1FD2` (home.asm:4130, called home.asm:259) **before** the `$05E4` body; nothing in the frame writes it, so there is no writer to be ahead of; `apply_pairing.py:21` (`KF`) pairs the keys to the replayed frame’s own read | yes — live window keys (main.rs:2071-2083, consumed 2096-2122) | injected-and-safe (input, exact by the pairing) |
| `a` as press edge (`a_pressed`, `prev_a`) | main.rs:1204 (`prev_a = f0.a`), 1218-1219 | `$160A` jump gate `a_pressed` (player_sim.rs:1416-1427) | same as above; the edge is derived from two consecutive records, both shifted by the pairing | yes (live prev_a, main.rs:2084-2085) | injected-and-safe |
| `phase` ($FF97 low 3 bits) | main.rs:1229-1234 | `$181C` dither index `MarioSpeedTableH[hvel+(phase&7)]` (player_sim.rs:1536, ROM `$181C`/`$19FF` table); `$129F`→`$12CE` side-nudge `phase & 1` (player_sim.rs:1181/1186, ROM `$12CE`); spin pose `($FF97&$0C)>>2` (player_sim.rs:2230, ROM `$5348`/`$532D`) | writer = `$0300`/`$2729` `inc $FF97` at the frame TOP (home.asm:349-351; 5170-5172), **before** every reader; the raw sample races that inc, so `apply_pairing.py:21` shifts `phase` like the keys and main.rs:1229-1234 repairs the slipped-increment case (traj10 f2986/2988/2989, comment 1220-1228) | yes — cadence only: `sim.phase = (sim.phase+1) & 7` (main.rs:2105) | injected-and-safe (exact for `$05E4`-paired captures; repair rule for tick-sampled ones) |
| `pan` ($A2B3 `sCameraPanTimer`) | main.rs:1322 (`$2F72` scroll for the below-bonk), 1336 (`build_contact` pan → object engine) | (a) `$2F72` (home.asm:6342-6345) reads `sScrollY`; (b) object draw `$C090` (main.asm:8597-8598) reads `sScrollY`; (c) capture reg `rd(0xA2B3)` (captures/trajectory_capture14.py:107); ROM writers: the dec at main.asm:479-485 (`$54E9`), set 52/56 by the boss arms main.asm:6790/6859/7275 | (a) the consumed value is **two** pan writes behind: `$2F72` reads frame F-1’s `$2062` latched `sScrollY` (home.asm:4227-4243), whose pan term was read before frame F-1’s own `$54E9` dec (home.asm:804 vs 805); (b) **one** behind: frame F’s `$2062` reads pan before frame F’s `$54E9` dec; the port feeds record-i pan (= end of frame F) at both sites | no — passes 0 at both mirrored sites (main.rs:2147, 2157; comment 2116-2117) | **injected-and-suspect** (§2; inert on the current corpus) |
| `edge` ($FF98) | 1277 (check read), 1327 (`cam.edge_now`), 1335 (`edge_derived`) — only when the level edge table is missing | `$0B39` write at the head of `$0B2A` (home.asm:1518-1532, `ldh [$FF98],a` at 1532) then the same routine’s branch reads (home.asm:1540 `$0B5E` bit0, 1614, 1716, 1776, 1870); port: camera.rs:327 (`step`), 231/246 (crouch-pan re-lookup `$3098`/`$30D9`, home.asm:6551-6554/6568-6575) | the record’s byte = the value frame F’s own `$0B39` writes (not rewritten later in the frame), so it is correct for the step’s reads; the check reads the port’s **pre-step** cell because the frame-END register value IS the pre-move cell (settled instance #2; main.rs:1271-1276, camera.rs:138-144) | yes — derives from the level table in both loops (camera.rs:129-136); the fallback exists only in replay | injected-and-safe (settled; one fallback-only nuance, §4) |
| `gap` | main.rs:1381 (`if let Some(g) = &f.gap`) | none — no sim/cam/obj state (drives the modelled fade + reload accounting only) | n/a | no (no such stretch interactively) | injected-and-safe (accounting, no timing surface) |

### 1b. Injected once, from `frames[0]` (the seed; record 0 = the state at the end of frame -1, `apply_pairing.py:9`)

These are the pre-roll: the port starts the replay already in this state, so every reader of the first replayed frame sees exactly what the console saw until the port’s own writer for that register runs at the console’s own position. All `injected-and-safe`; the notes say where the port re-derives each.

| field | injected-at (file:line) | reader(s) (file:line + ROM addr) | reader’s frame position vs the writer | run_play source | verdict |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `x` | 1137 | `$0FBF` wrap (player_sim.rs:1562, ROM home.asm:2119-2141), `$181C` (player_sim.rs:1532-1552), `$129F`/`$1413`/`$1BFF` | seed only; re-derived every frame inside `$0F71` | yes — header spawn (main.rs:210) | injected-and-safe (seed) |
| `y` | 1138 | `$14FA` integrator (player_sim.rs:1424-1450), `$129F` landing, camera `$0C7D` `sy98` via `prev_mario_y_lo` (camera.rs:398-399; ROM home.asm:1800-1839) | seed only; re-derived in `$0F71` | yes — header spawn (main.rs:211) | injected-and-safe (seed) |
| `hvel` | 1139 | `$1701`/`$16CC`/`$181C` (player_sim.rs:1480-1536) | seed; re-derived | yes — `PlayerSim::new()` `$80` (player_sim.rs:516) | injected-and-safe |
| `vvel` | 1140 | `$14FA` gravity (player_sim.rs:1424+) | seed; re-derived | yes — default 24 (player_sim.rs:507) | injected-and-safe |
| `ground` | 1141-1142 (with `in_air`) | `$129F` landing / gravity gates (player_sim.rs:1435-1438) | seed; re-derived | yes — default true (player_sim.rs:518) | injected-and-safe |
| `dir_right` | 1143 | direction gates `$1701`/`$181C`, frame selection | seed; re-derived | yes — default true (player_sim.rs:520) | injected-and-safe |
| `timer` → `walk_timer` | 1144 | `$14BF` anim timer (player_sim.rs:1580-1581, ROM home.asm:2124), frame selection `$2D0A`/`$530D` (player_sim.rs:2230) | seed; re-derived | yes — default 0 (`PlayerSim::new()`) | injected-and-safe |
| `lives` | 1145 (`unwrap_or(5)`) | `$311A` BCD decrement / game-over (player_sim.rs:1661-1662, 1748-1755) | seed; re-derived by the modelled fade/gap | yes — default 5 (player_sim.rs:552) | injected-and-safe |
| `camx`+`camxh` | 1152-1154 (`CameraSim::new`) | camera `$0B2A` self-reads; via `sim.camx` (derived at 1252) the `$0FBF` wrap (player_sim.rs:1562) | seed; the camera is re-derived every frame | yes — header camera_start (main.rs:212-215) | injected-and-safe |
| `camy`+`camyh` | 1153-1154 | camera `$0B2A` self-reads; via `sim.camy` (derived at 1253) the `$1BFF` hazard `sMarioScreenY` (player_sim.rs:732-733); crouch-pan screen-Y gates (`$3098`/`$30D9`, camera.rs:224-255) | seed; re-derived | yes — header camera_start | injected-and-safe |
| `y` → `prev_mario_y_lo` | 1156 | `$0C7D` vertical follow (camera.rs:398-399, 217; ROM home.asm:1808/1833 threshold reads); rewritten per frame (camera.rs:465) | seed; re-derived | yes — `cam.prev_mario_y_lo = sim.y` (main.rs:216) | injected-and-safe |
| `limit` → `map_scroll_limit` | 1157 | camera thresholds (camera.rs:444/450; ROM home.asm:1808/1833, `sMapScrollLimit` `$A2D9` = `LevelPropertiesUNK`, loaded home.asm:741-747, written nowhere else) | run-constant level property — no in-frame writer | yes — `manifest.properties.unknown != 0` (main.rs:223) | injected-and-safe (different source; no per-frame check, §4) |
| `autoscroll` | 1158 | `$0DAF` branch select (camera.rs:335; ROM home.asm:1533, `sAutoScroll` `$A2C8`, loaded home.asm:633-639) | run-constant in replay (cleared only by the pause-abandon path, home.asm:5217, which no record reaches) | yes — `manifest.properties.auto_scroll` (main.rs:224) | injected-and-safe (no per-frame check, §4) |
| `easy` | 1173 | spawn-scan mode gate (objects.rs:2382; ROM main.asm:3356/3476/7174, `sEasyMode` `$A2E4`, loaded main.asm:16033) | run-constant save/session flag — no in-frame writer | no (default false; play has no menu source) | injected-and-safe |
| `ff0` ($FFF0) | 1175 (fallback from `f0.odd`) | `$812E` toggle re-derives it (objects.rs:551-555); readers `fff0 & 1`/`& 3` (objects.rs:864, 912, 937, …) | seed; re-derived by the engine | no (default 0) | injected-and-safe |
| `uphase` (+ `phase` bool) | 1175-1180 | `$812E` toggle (objects.rs:552-553); parity gate `(i&1) != (update_phase&1)` (objects.rs:694, ROM `$85AA` tail) | seed; re-derived | no — `ObjectSim::new()` (`phase=true` 399, `update_phase=0` 402) | injected-and-safe |
| `pcx` | 1182-1183 | scan direction `$813C` (objects.rs:537), writer same scan (objects.rs:614) | seed; re-derived by the scan | no — `prev_cam_x = cam.x` at construction (objects.rs:394) | injected-and-safe |
| `odd` | 1175 (only as the `ff0` fallback) | `obj.odd` derived at 1185 (`fff0 & 1`) | derived | n/a (derived) | derived |

### 1c. Check-only (never injected)

Read **only** in the post-frame comparison block (main.rs:1490-1830); the sim derives them, and a wrong record value fails the gate loudly — it cannot steer a branch. The record holds the frame-END register value by the pairing (`apply_pairing.py:4-6`), which is exactly what a post-frame comparator wants. Verdict for all rows: `derived`. The two check arrangements that needed care are called out where they live (`edge`’s comparator is `cam_edge_before`, main.rs:1277/1547; `slots`/`slot_bytes`/`bslots` are raw-byte comparisons).

| field | injected-at (file:line) | reader(s) | reader’s position vs the writer | run_play source | verdict |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `x` | — (check 1494) | none (comparator) | post-frame | n/a (no checks in play) | derived |
| `y` | — (check 1495) | none | post-frame | n/a | derived |
| `hvel` | — (check 1498) | none | post-frame | n/a | derived |
| `vvel` | — (check 1503) | none | post-frame | n/a | derived |
| `ground` | — (check 1508) | none | post-frame | n/a | derived |
| `timer` | — (check 1513) | none | post-frame | n/a | derived |
| `dir_right` | — (check 1518) | none | post-frame | n/a | derived |
| `frame` | — (check 1523) | none | post-frame | n/a | derived |
| `camx`+`camxh` | — (check 1528) | none | post-frame (derivation checked; the settled camx fix) | n/a | derived |
| `camy`+`camyh` | — (check 1538) | none | post-frame (same) | n/a | derived |
| `edge` | — (check 1548, reads `cam_edge_before` 1277) | none | **pre-step cell** by construction — the frame-END register value is the pre-move cell (settled #2) | n/a | derived |
| `en_r` | — (check 1553) | none | post-frame | n/a | derived |
| `en_l` | — (check 1558) | none | post-frame | n/a | derived |
| `sptr` | — (check 1563, `+$AB06`) | none | post-frame | n/a | derived |
| `spx` | — (check 1572) | none | post-frame | n/a | derived |
| `sdir` | — (check 1577) | none | post-frame | n/a | derived |
| `standby` | — (check 1582) | none | post-frame | n/a | derived |
| `active` | — (check 1587) | none | post-frame | n/a | derived |
| `slots` | — (check 1460, `objslots`) | none | post-frame | n/a | derived |
| `exit` | — (check 1592) | none | post-frame | n/a | derived |
| `lowtime` | — (check 1599) | none | post-frame | n/a | derived |
| `fadelo`+`fadehi` | — (check 1606) | none | post-frame; the sim’s fade model is stepped by the frozen branches (1282-1285) and the gap (1381+) — the record never seeds it | n/a | derived (VBlank-side writer, but never injected) |
| `tally` | — (check 1614) | none | post-frame | n/a | derived |
| `odd` | — (check 1624) | none | post-frame | n/a | derived |
| `moon` | — (check 1631) | none | post-frame | n/a | derived |
| `slot_bytes` | — (check 1640, raw 32-byte dumps) | none | post-frame | n/a | derived |
| `mf` | — (check 1668) | none | post-frame | n/a | derived |
| `fac` | — (check 1675) | none | post-frame | n/a | derived |
| `dead` | — (check 1682) | none | post-frame | n/a | derived |
| `lives` | — (check 1689) | none | post-frame | n/a | derived |
| `pow` | — (check 1696) | none | post-frame | n/a | derived |
| `inv` | — (check 1703) | none | post-frame | n/a | derived |
| `goal` | — (check 1710) | none | post-frame | n/a | derived |
| `secret` | — (check 1719) | none | post-frame | n/a | derived |
| `sbt` | — (check 1726) | none | post-frame | n/a | derived |
| `scc` | — (check 1733) | none | post-frame | n/a | derived |
| `mb` | — (check 1740) | none | post-frame | n/a | derived |
| `uphase` | — (check 1747) | none | post-frame | n/a | derived |
| `ff0` | — (check 1754) | none | post-frame | n/a | derived |
| `pipe` | — (check 1761) | none | post-frame | n/a | derived |
| `tr` | — (check 1768) | none | post-frame | n/a | derived |
| `pcx` | — (check 1775) | none | post-frame | n/a | derived |
| `spin` | — (check 1782) | none | post-frame | n/a | derived |
| `coins` | — (check 1789) | none | post-frame | n/a | derived |
| `coinshi` | — (check 1796) | none | post-frame | n/a | derived |
| `bslots` | — (check 1805, raw 16-byte dumps) | none | post-frame | n/a | derived |
| `exit_arm` | — (one-shot check 1866) | none | outside the loop; `sim.exit_phase`, the modelled fade tail and `sim.level_tally` compared to the arm’s facts | n/a | derived |

## 2. `injected-and-suspect` fields

**One field: `pan` (`sCameraPanTimer`, `$A2B3`).** Verdict reached by reading (the ROM order is unambiguous); it cannot be *measured* on the current corpus because pan is 0 on every record of all 19 gate captures (verified by scanning every `captures/*.json` for the `pan` value — `traj4`, `traj5` do not carry the field at all; every other capture, `crouch_pan1` included, has `pan` = 0 on every record). It is inert, so landing a verdict now costs nothing to the suite; landing a FIX without a pan-exercising capture would.

Evidence, site by site:

1. Writer schedule. `sCameraPanTimer` (`$A2B3`, ram.asm:178) is decremented by `$54E9` (main.asm:479-485: `ld a,[sCameraPanTimer] / and a / jr z,+ / dec a / ld [sCameraPanTimer],a / and $03 / ld b,a / ldh a,[$FFC4] / sub b`). `$54E9` sits in the `$2D0A` stage of the frame (home.asm:805 `call $2D0A`, which is `jp $530D` in bank 1 — main.asm:439-491 is that pipeline: the pose/dispatch arms converge on `$54A7` → `$54C8` → `$54E9` → `$54FB`). Nonzero can only be produced by the boss-transition arms (main.asm:6790 = 52, 6859 = 56, 7275 = 56); the crouch pan `$307D` does **not** touch it (it moves `$FFC8/$FFC9` directly, home.asm:6554-6565).
2. Console reader (a) — the below-bonk scroll (`$2F72`, home.asm:6342-6356): reads `sScrollY` (6345) and `sScrollX` (6352). `sScrollY` is written by `$2062` (home.asm:4227-4243) as `camY - 72 - (pan & 3)`, and `$2062` runs at home.asm:804 — after `$0B2A` (803) and **before** `$2D0A` (805, the pan dec). Frame F-1’s `$2062` therefore embedded the pan value as of BEFORE frame F-1’s own dec. For the harness frame F (iteration i) that value is the pan at the end of console frame F-2 = record **i-2**’s pan. `$2F72` runs inside `$0F71` (early in frame F). Port: main.rs:1317-1326 feeds `f.pan.unwrap_or(0)` (record i) while the camera term (`cam.y`) is the post-frame-(F-1) camera (correct) — the pan term is therefore **two writes ahead** of the console’s.
3. Console reader (b) — the object draw `$C090` (main.asm:8597-8598 reads `sScrollY`) at the `$2D7D` stage: its `sScrollY` was written by frame F’s own `$2062`, which read pan before frame F’s dec → pan at the end of frame F-1 = record **i-1**’s pan. Port: objects.rs:650 (`st.b[B_SCRY] = y + 16 - (cam_y - 72 - (pan & 3))`, with the pan from `build_contact`, main.rs:1336) — the pan term is **one write ahead** of the console’s.
4. run_play parity (item rule 3): play passes 0 at both mirrored sites (main.rs:2147, 2157) — a documented source gap, but it means the two loops disagree on this input by construction; replay’s own construction is the one with the timing defect.

What would settle it: the verdict itself is already settled by reading. To settle the *impact*: a capture that fires one of the boss-pan arms (none exists; the gate is blind to pan), or — cheaper and capture-free — a Rust unit test that drives a nonzero pan through the two call sites and compares the produced `sScrollY`-equivalent against the ROM chain (record i-2 at `$2F72`; record i-1 at `$C090`). Either way the fix shape is “use the record’s pan from the matching frame, not the current one”, not “re-time back to record i”.

## 3. Two-loop differences (item method rule 3)

Every difference found, with its status:

1. `pan`: replay = `f.pan` (1322/1336) vs play = 0 (2147/2157, comment 2116-2117). Combination of a documented source gap (play has no sCameraPanTimer source) and the replay-side timing defect above (§2).
2. `phase`: replay adds the record-repair branch (1229-1234); play is pure cadence (2105). Not a defect — the replay-only branch exists for the tick-sampled captures’ boundary race; both loops land on the same cadence value in the normal case.
3. `edge`: the missing-table fallback (1277/1327) exists only in replay; play derives only (or runs all-clear bits with a note). Gate-unreachable path (the note fails the sweep).
4. Seeds: replay takes record 0 for `x`,`y`,`hvel`,`vvel`,`ground`,`dir_right`,`timer`,`lives`,`camx/camy` (+`prev_mario_y_lo`,`limit`,`autoscroll`), `easy`, `ff0`, `uphase`/`phase`, `pcx`; play takes header/manifest/defaults (main.rs:208-243). All are the recordless-play side of the class already queued as `infra_play_parity_harness`; the manifest-sourced ones (`limit`, `autoscroll`, `scene_state`) were unified 2026-10-14. Residual record-only seeds: `easy`, `ff0`, `uphase`/`phase`, `pcx` (defaults differ from a live console value — e.g. `traj4_ff0` documents a real `$FFF0` = 3 at frame 0).
5. `sim.camx`/`sim.camy`: both loops now latch the port’s own camera (1252-1253 / 2100-2101) — the fixed divergence; no difference remains.

## 4. What I could NOT determine

- **`pan` impact is unmeasurable on the current corpus.** All-zero in every gate capture (verified directly). The `$54E9`-per-frame reachability through the `$530D → $54A7/$54C8/$54E9` pipeline is inferred from the arm convergence at main.asm:439-491, not exhaustively disassembled branch by branch; and whether any recorded level ever sets pan nonzero away from the boss arms (3 writer sites only) is untested.
- **The raw-sample mechanics** (which PyBoy boundary fires `$05E4` and how `apply_pairing.py`’s shift was calibrated for each capture generation) are taken from `apply_pairing.py:4-10` and the settled camx fix, not independently re-derived. For TICK-sampled captures (traj5-traj13) I relied on main.rs:1220-1239’s comments about the $FF97 cadence race.
- **`phase`’s else-branch** (record wins when it disagrees with the cadence) is documented for the death captures and gate-verified; I did not re-derive it from the death captures’ raw files. A one-behind record would be followed as-is (no lower-bound repair) — unobserved in the corpus.
- **`frame_counter`**: bits ≥ 3 are free-run (1235-1239); its readers (pause flash `$2756` bit 4; spin pose `$5348/$532D`) are unreachable in the gate. Wrap-phase fidelity unverified — documented as best-effort in the code.
- **The `!own_edges` fallback’s crouch-pan nuance**: `crouch_pan` (1301) reads `cam.edge_now` set at 1327 of the PREVIOUS iteration, so on the first frame after a camera cell-crossing it uses the previous record’s byte — while the console’s `$3098`/`$30D9` re-looks-up the table at the current cell (home.asm:6551-6554/6568-6575). Read-derived; unmeasurable because the missing-table path fails the gate by its own note.
- **Unchecked injected fields**: `limit`, `autoscroll`, `easy`, `prev_mario_y_lo` and `pan` have NO per-frame check — a wrong record value for them would be silent (all are run-constant or panel-inert except pan, §2). `odd`-at-frame-0 is only used as the `ff0` fallback.
- **The gate suite was not re-run by me** (read-only role; at the pinned HEAD a parallel writer was mid-sweep — the commit message itself says so). Nothing in this note changes behavior, and the one suspect verdict is inert on the corpus (pan ≡ 0), so the suite state is unaffected by the audit; a fix for `pan` should land with the suite proven byte-identical, which the current corpus guarantees trivially.
