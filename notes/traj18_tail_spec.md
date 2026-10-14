# traj18 — the level-clear tail: a port-ready spec for capturing past the `$14` hand-off

Target: extend the level-clear sequence (`traj17.json`, 6977 records, ends on the frame that sets
`$FF9B = $14`) through the banked `$14` body (`$3EF2B`) into the state the console settles on, so the
port's `PlayerSim::map_handoff` + `PlayerSim::save_record` stop being unit-only and get a console check.

Queue items this closes / unblocks: `level_tail_capture` (this capture), `map_handoff_14_body`
(the banked body it exercises), `overworld_map_state_0C` (bounded by the cut below).

Base artifacts (read-only for this spec):
- `captures/trajectory_capture17.py` — the route + the two-hook splice; traj18 is a copy of it with a
  longer tail.
- `crates/sml2-runtime/src/main.rs` — the harness (`run_replay`).
- `crates/sml2-core/src/player_sim.rs` — `exit_fade_step`, `map_handoff`, `save_record`.

Evidence tiers used below — every load-bearing claim is tagged:
- **QUOTED** — read from the disassembly at the cited `file:line` (address in parens).
- **MEASURED** — live PyBoy runs on the same route with the arm's end state poked in
  (`/home/pmill/.hermes/cache/scratch/traj18_probe.py`, `traj18_probe2.py`, `traj18_probe3.py`;
  two independent runs agreed tick-for-tick, and one that walked Mario first answered the
  "frozen vs reset" question). The poke writes exactly the arm's end state
  (`sExitTileFlag = 0`, `sGoalBell = 0`, `sLevelTally = 0`, `sFadeTimer = 192`, `$FF9B = 24`), so the
  measured sequence is the console's own.
- **INFERENCE** — reasoning, marked as such.

---

## 1. The state sequence from the arm frame to the settle, and what is input-dependent

Console frame-by-frame, `$FF9B` at the frame boundary (`$FF9B` is read/dispatched once per frame by
`$02AD`'s `rst $28` table — **QUOTED** home.asm:290-330; state `$04` → `$05D5`, `$0C` → `$2C2E`
(home.asm:307), `$14` → `$2C23` (home.asm:315), `$18` → `$3547` (home.asm:319)):

| # | frame start state | what runs | ticks (measured) | input? |
|---|---|---|---|---|
| 1 | `$04` | the arm frame: `$1BFF`'s `(x+16, y+28)` sample reads the `$4A` cell → `$1CBA` sets `$FF9B = 24`, `sFadeTimerLow = 192`/`High = 0`, `sExitTileFlag = 0`, `sMusicRequest = 3`, clears `sLowTimeFlag`/`sSecretRouteFlag`/`sTileAnimConfig`/the three SFX requests (**QUOTED** home.asm:3671-3701) | 1 | none; the arm record's `exit_arm` block is the capture's own evidence |
| 2 | `$18` | `$3547`: fade timer non-zero → raster-wait, `ret`. VBlank decrements `sFadeTimer` (`sub 1`/`sbc 0`, saturating at 0) **QUOTED** home.asm:115-124 | 192 frames (192 `$028D` fires, all `st=$18`) | none — `$3547` never reads the pad; the pause entry `$273E` only runs from the state-`$04` tail (home.asm:818/827), so START during the fade is inert |
| 3 | `$18` (last fade frame) | `$3547` tail: timer 0 → unless `sLevelProgressFlags` bit 7 or `sExitTileFlag`, BCD `sLevelTally += 1`; then `$FF9B = 20` (**QUOTED** home.asm:7201-7216) | this frame IS recorded (its end-sample reads `$14` — traj17's last record) | none |
| 4 | `$14` | `$2C23` maps bank 15 in and `jp $6F2B` (= file offset `$3EF2B`) (**QUOTED** home.asm:5851-5855; main.asm:24126): palettes `$E4`/`$D0`/`$39` (main.asm:24128-24133); `sGoalBell == 0` → `$3EF56`; `sExitTileFlag == 0` → `$3EFB2`; `$3EEF0` marks `sLevelProgress[$650E[node]]` + saves (main.asm:24089-24124); overworld gfx copy; `$3EEA4` cursor/window from `$5EE6[node]` (main.asm:24052-24087); `sMapCursorMove = 8`, `sMapScreenId = 0` (main.asm:24214-24217); node tail `$3F003`→`$3F024`: node 0, live Mario → `sLevelProgress` bit 7 set, `sMapNode = 20` (main.asm:24261-24272); `$3F03D`: `$3F156` hub setup → `$2934` save → `$FF9B = 12` → `$3EA9A` (music 6) → `ei; ret` (main.asm:24274-24282) | **21-22 ticks**, 0 `$028D` fires (the body abandons the main loop; measured gap = 33-35 ticks of no fire from the tail frame) | none — nothing in `$3EF2B`'s path reads the joypad |
| 5 | `$0C` | the overworld hub as `sMapScreenId = 0`, `sMapNode = $14`; first `$028D` fire lands here (11-13 ticks after `$FF9B` became 12 — the tail of the body's work) | the **gap record** | NONE NEEDED — first input consumer is the map's own cursor code; the capture cuts here |

Measured timeline (probe2, tick = PyBoy frame; probe3 identical shape):

```
poke at 860
fade fires:      861..1052   st=$18   (192 fires, one per tick)
tail frame fire: 1053        st=$14   (tally 0->1, the LAST recorded frame)
   --- no $028D fire for the next 33 ticks ---
gap fire:        1086        st=$0C   (node=$14, prog0=$80, scr=0, curm=$08)
```

Key structural facts:

- **The `$14` body is one dispatch that spans ~21-22 wall-clock frames**, with the main loop abandoned
  for its whole duration: `0` `$028D` fires during it, and `$FF9B` reads 20 at every tick end until
  `$3F03D` writes 12. Thereafter it still takes ~11-13 more ticks before `$028D` is reached again
  (the body's remaining work + the `$3F156` VRAM copies, which run with interrupts off). Total
  fire-silence from the tail frame to the gap record: **33-35 ticks** (measured 34/35 in two runs;
  traj17's own 200-tick poll ended while still inside the body — it observed the first 9 of them).
- **Fires resume one per tick in state `$0C`** (measured: max 1 fire/tick over 374 subsequent fires,
  the game re-syncs to VBlank), so the *only* unrecordable stretch is the body itself.
- **Nothing in the chain is input-dependent.** The fade, the tally charge and the whole `$14` body are
  timed; the capture presses nothing from the arm to the cut. This is the difference from the death
  cycle (whose map needed A taps): the level-clear path has no key wait before the settle.
- **Bell-clear caveat (why the capture must assert `sGoalBell == 0` at the arm).** If the goal-bell
  *object* (item id 6) had been struck, `$3EF2B` takes the other branch: `sGoalBell != 0` →
  `$3F4B1` (even node) / `$3F5A4` (odd) — the castle-walk cutscene setup (`sMapScreenId` 13/14),
  which clears `sGoalBell` and hands to `$0C` in the same frame (**QUOTED** main.asm:24134-24151);
  the walk's end re-enters `$3EF2B` for pass 2 via `$3C101` when `sCastleState == $0A`
  (**QUOTED** main.asm:18451-18456) — and *that* state machine IS input-gated (its key test is
  `ldh a,[$FF80] / and $20`, main.asm:18442-18447). traj17's route reaches the bell on the FLOOR
  (the bell object sits 108 px up, DESIGN.md:787-799) so `sGoalBell` is 0 and the single-pass path
  above is what runs; the capture records `$A24F` and asserts 0 so a route regression aborts loudly
  instead of silently capturing the cutscene path. Two independent live runs already show this:
  the previous session's walk-to-the-bell probe (scratch `end_level_probe.json`, tag `walk102`:
  `mx=4057`, `st=24`, `goal=0`) and this spec's probes. **Correction for the record:** an earlier
  scratch analysis (`handoff.json`, `min_set_for_level1_clear`) assumed "level 1 rings the bell, so
  pass 1"; the measured route arrives with `sGoalBell = 0`, so traj18 exercises the map-return half
  (single pass) — the pass-1 branch stays capture-less and is the follow-up for a deliberate
  bell-ring capture.
- Pass 2's `$3EF56` secret-exit rewrite is skipped here (`sExitTileFlag == 0` from the `$4A` arm);
  the dead-Mario variant (`sMarioDead != 0`) also skips the `$3F024` mark — measured in probe3, where
  Mario died on the walk first (`node` stayed 0, `prog0` stayed 0) — another reason the capture
  asserts `dead == 0` at the arm (it already does, trajectory_capture17.py:611-612).

## 2. What a capture can CHECK to prove the body ran

All values below are for a plain level-1 bell clear, alive Mario, `$4A` (the measured probe state):

| address | what | expected | evidence |
|---|---|---|---|
| `$A840` (sMapNode) | node moved by the `$3F024` tail | `$14` (20) at the gap record; `0` through the level | QUOTED main.asm:24270-24271 (`ld a,20`/`ld [sMapNode],a`); MEASURED 20 |
| `$A848` byte 0 (sLevelProgress, node 0's `$650E` slot = 0) | `set 7` + `res 3` | `$80` | QUOTED main.asm:24108-24109 and 24267-24269; MEASURED `$80` |
| `$A2C5` (sLevelTally BCD) | charged by `$3547`'s tail | `$01` (0→1 on the tail frame) | QUOTED home.asm:7208-7211; MEASURED 01 (already in traj17) |
| `$A27D` (sLevelProgressFlags) | **not written** by the hand-off | `$00` | QUOTED (no write in `$3EF2B`/`$3EEF0`/`$3F003`); MEASURED 00 |
| `$A2B4` (sExitTileFlag) | `$4A` arm cleared it | `$00` | QUOTED `$1CBA`; MEASURED 00 |
| `$FF9B` | the settled state | `$0C` at the gap record (`$18` during the fade, `$14` on the tail frame, `$04` before) | QUOTED main.asm:24278-24279; MEASURED |
| `$A000+0`, `$A000+8` (save slot 0 = `sSaveFileIndex` 0) | the `$2934` image: node, then `sLevelProgress` at +8 | `$14`, `$80` | QUOTED home.asm:5460-5503; MEASURED `14 00 4B 4C 08 00 00 00 80 ...` |
| `$A000+$40..$44` | coins lo/hi, tally, lives, money bag | `00 00 01 05 00` | MEASURED (lives 05 = the fresh-file poke; tally 01) |
| `$A000+$45` | the checksum = sum of the 69 bytes before it | `$39` | QUOTED home.asm:5502 + `$28EC` home.asm:5400-5429; MEASURED. **The port cannot predict this byte** — +2/+3/+4 are the cursor X/Y and `sMapCursorMove` (console `4B 4C 08`), which the port does not model — so it is checked as the record's own internal consistency, not against `save_record`. |
| `$A000+$46..$49` | the 4 signature bytes `$28EC` requires | `12 34 56 78` | QUOTED home.asm:5416-5427; MEASURED (note: `DESIGN.md:977` says "five signature bytes … $12 $34 $56 $78 $78" — the ROM checks **four**, and the port's `map::SAVE_SIGNATURE` is the correct 4 bytes) |
| `$A68B`, `$A844`, `$A842/$A843` | `sMapScreenId = 0`, `sMapCursorMove = 8`, cursor (75,76) | `00`, `08`, `4B/4C` | QUOTED main.asm:24214-24217; MEASURED — capture-side facts only (unmodelled) |
| palette shadows `$A27E/$A27F/$A280` (and rBGP/rOBP0/rOBP1) | the hand-off writes `$E4`/`$D0`/`$39` | settled: `$E4`/`$D1`/`$39` (a later map-side write bumps OBJ0 to `$D1`) | QUOTED main.asm:24128-24133; MEASURED — **capture-side only**: the port renders the level from the manifest and has no palette model, so nothing in the harness can compare it |

The strongest port-comparable pair is **`$A840` + `$A848[0]`**, because the port has both
(`PlayerSim::map_node`, `PlayerSim::level_progress[0]`) and `map_handoff` writes them. The save record
adds `$2934` itself (both saves — measured: two `$2934` calls, the second's image is what lands in
SRAM).

## 3. The harness change (smallest edit that compares the post-hand-off record)

**Measured premise that keeps the edit small: the gap record is comparable by the ORDINARY per-frame
table.** probe3 walked Mario to x=442 (walker spawned, 2 non-zero slots) and poked the fade: the gap
record still read `x=442`, the same camera, the same `sptr`/`standby`/`active`, byte-identical 32-byte
slot dumps, the same `frame`/`edge`. The hand-off freezes the level registers; it does not reset them.
Only `lvl9b`, `tally` and `sMapScreenId` changed. So the per-frame checks do **not** need gating; what
is needed is the fields for the hand-off's own outputs plus a gap-branch dispatch.

All line numbers are `crates/sml2-runtime/src/main.rs` as of this spec.

1. **`struct Frame` (main.rs:847-984)** — add, next to the `exit_arm` field (line 972):
   ```rust
   /// `sMapNode` ($A840) — the level-clear hand-off's node move (`traj18`).
   #[serde(default)]
   node: Option<u8>,
   /// `sLevelProgress[0]` ($A848) — the node-0 progress byte the hand-off sets.
   #[serde(default)]
   prog: Option<u8>,
   /// `$FF9B` — dispatch-only (a record whose end sampled `$0C` is a map frame).
   #[serde(default)]
   lvl9b: Option<u8>,
   /// The `$2934` save record ($A000 + sSaveFileIndex*$50, 0x50 bytes hex),
   /// carried on the hand-off's gap record only.
   #[serde(default)]
   save: Option<String>,
   ```
   All optional, so every existing capture keeps replaying (the `#[serde(default)] Option` rule).

2. **The per-frame pushes** — in the check block, next to the `exit`/`tally` pushes (main.rs:1551-1581),
   mirror that pattern exactly:
   ```rust
   if let Some(v) = f.node { push("node", sim.map_node == v, format!("node sim=${:02X} exp=${:02X}", sim.map_node, v)); }
   if let Some(v) = f.prog { push("prog", sim.level_progress[0] == v, format!("prog sim=${:02X} exp=${:02X}", sim.level_progress[0], v)); }
   ```
   No extra work is needed to make these fire on the gap record: **the port already calls
   `sim.map_handoff(&nodes)` on that exact record** — main.rs:1249 reads `exit_done_at_entry =
   sim.exit_completed` *before* line 1252's `exit_fade_step`, which completed on the previous (tail)
   record, so line 1254-1255 runs the collapse on the first post-hand-off record. The pushes then
   compare the body's outputs against the console's.

3. **The gap dispatch** — the existing death handler at main.rs:1342-1384 misfires on traj18's record
   (`fade_ok` wants `Some([9, 192])`; traj18's seq is `[[20, n1], [12, n2]]`), so wrap it and mirror
   its own shape for the level-clear branch:
   ```rust
   if let Some(g) = &f.gap {
       if g.seq.first().map(|s| s[0]) == Some(0x14) {
           // The level-clear hand-off (`captures/traj18.json`): the fade is
           // recorded and checked per-frame; this gap covers only the `$14`
           // body's unrecordable run. The port's fade step + map_handoff have
           // already run in the frozen branch above (main.rs:1250-1256) — so
           // only the captured accounting is validated here.
           let acct: u64 = g.seq.iter().map(|s| s[1]).sum();
           let ok = g.seq.first().map(|s| s[0]) == Some(0x14)
               && g.seq.last().map(|s| s[0]) == Some(0x0C)
               && acct == g.total;
           println!("  frame {i}: level-clear hand-off: {} unrecorded ticks (seq {:?}, total {} vs {} accounted){}",
                    g.total, g.seq, g.total, acct, if ok { "" } else { "  <- MISMATCH" });
           handoff_gap_ok &= ok;
       } else {
           ... the existing death-cycle body, unchanged (main.rs:1343-1383) ...
       }
   }
   ```
   **Do NOT reuse the death relation.** The death handler validates `1 + sum(seq) == total`
   (main.rs:1345-1346); traj18's measured relation is `sum(seq) == total` (34==34 and 35==35 in the two
   probe runs) because its extension loop starts counting after the tail frame, not at it. The capture
   asserts the same relation, so a bookkeeping drift fails in the capture first (`abort`), and a
   harness/capture disagreement fails here.
   Add `let mut handoff_gap_ok = true;` beside `gap_ok` (main.rs:1183) and a note beside the existing
   one (main.rs:1900-1902): `note: the level-clear hand-off gap check FAILED`.

4. **The one-shot save check** — mirror the `exit_arm` block (main.rs:1806-1892; that block needs **no
   change**: the fade is fully stepped by the per-frame loop, so `while sim.fade_timer > 0` performs 0
   steps and `sim.exit_completed` is already true — its trailing comment at 1852-1855 is stale for the
   tail frame, see §4). After the loop, find the frame carrying the level-clear gap + `save`:
   - parse `rec` = the 0x50 hex bytes;
   - `map::save_record_valid(&rec)` on the CAPTURED record (checksum + 4 signature bytes) — this is
     independent of the sim and proves `$2934`'s layout end to end;
   - compare the modelled offsets against `sim.save_record(obj.money_bag)`:
     `sim.map_node == rec[0]`, `sim.level_progress[0] == rec[8]`, `rec[0x40..0x45] == coins_low,
     coins_high, level_tally, lives, money_bag`;
   - print one `  hand-off: ...` line and raise a `note:` on failure (the one-shot pattern, since these
     checks exist on a single record and must not go through the per-frame table).

No pairing change: the `$028D` records sit on the same frame boundary as the `$05E4` ones (traj17's
splice already replays all-zero through 193 of them), `apply_pairing.py`'s `KF` shift covers the gap
record identically, and `tick`/`gap` are not shifted (apply_pairing.py:21-29).

## 4. What is structurally unrecordable, and where the capture MUST be cut

- **The `$14` body itself is unrecordable.** Measured: `$028D` fires 0 times across the body's whole
  run (21-22 ticks in `$14` + ~11-13 more before the fire; 33-35 ticks of silence from the tail
  frame), because the body never returns to the main loop. There is no hook point that fires during
  it, so the tick count can only be POLLED, never sampled — this is the case the skill warns about,
  and the gap technique is the only cover.
- **The cut point: right after the FIRST post-hand-off (`$FF9B = $0C`) record.** One record is
  recorded past the gap — the first `$028D` fire in state `$0C` — and then the capture stops
  (no further ticks into the map).
  - Reason 1 (measured): the map's own screen logic rewrites state the port models — `sMapNode` goes
    `$14` → `$13` ~42 ticks after the gap fire, and `sMapCursorMove` walks `$08` → `$14` → `$44` → …
    (probe1/probe2). Every later record would fail the `node` check for a reason that is the map's,
    not the hand-off's.
  - Reason 2: the first fire lands well inside the window where the hand-off's outputs are still the
    console's (node `$14`, prog `$80`), so exactly one comparable record exists and is enough.
  - The capture asserts the record's own evidence before splicing (`lvl9b == 0x0C`, `node == 0x14`,
    `prog == 0x80`, `tally == 1`, `scr == 0`) and aborts loudly otherwise, so a route/timing change
    can never write a silently-wrong capture.
- **Stale comment to fix while editing** (not a behaviour change): main.rs:1852-1855 says the frame
  that charges the tally "never reaches `$028D`". The measured truth (and traj17's own last record,
  `lvl9b = $14`) is that the tally frame DOES fire `$028D`; the frame that never reaches it is the
  `$14` body's. Same wording in `captures/README.md`'s traj17 row ("the `$14` hand-off frame … never
  reaches `$028D`") is right; the harness comment is the one that misattributes it.

## 5. The PyBoy route change — `trajectory_capture18.py` (copy of `trajectory_capture17.py`)

Keep, verbatim:
- the whole prefix (trajectory_capture17.py:36-74), the slot helpers and `state()` (76-122) —
  **plus two new fields in `state()`**: `node=rd(0xA840)`, `prog=rd(0xA848)`, `goal=rd(0xA24F)`;
- the two hooks and the `held()` plumbing (124-146) — the `$028D` splice is the mechanism that makes
  the tail recordable at all (one fire per frame in every state);
- the route and the arm-detection walk (196-617);
- `arm = dict(...)` and its two sync asserts (629-640);
- the splice discipline `REC.extend(POST[arm['frame']:])` (663).

Replace the fixed `TAIL = 200` poll (trajectory_capture17.py:619-673) with the two-phase tail:

```python
# phase A — tick until the tail frame's fire: the frame that ran $3547's tail.
# Its end-sample is the FIRST fire whose lvl9b == 0x14 (measured: exactly one
# such fire exists; the fade's 192 fires all read 0x18).
n = 0
while not (POST and POST[-1]['lvl9b'] == 0x14):
    tick(); n += 1
    if n > 400: abort('the $14 tail frame never fired')
tail_at = len(POST) - 1

# phase B — the extension: poll $FF9B every tick (death_capture2.py's
# gap technique) until the FIRST post-hand-off $028D fire appears.
# The gap's seq is built by THIS loop only (its own counters), which makes
# sum(seq) == total exact (measured 34==34, 35==35).
nt = len(POST); cur = rd(0xFF9B); k = 0; seq = []
for it in range(500):
    tick()
    s = rd(0xFF9B)
    if s != cur:
        seq.append([cur, k]); cur = s; k = 0
    k += 1
    if len(POST) > nt: break
seq.append([cur, k])
gap_at = len(POST) - 1
tot = POST[gap_at]['tick'] - POST[tail_at]['tick']
assert sum(d for _, d in seq) == tot, f'gap accounting skewed: {seq} total {tot}'
assert seq[0][0] == 0x14 and seq[-1][0] == 0x0C, seq
assert POST[gap_at]['lvl9b'] == 0x0C, POST[gap_at]['lvl9b']
POST[gap_at]['gap'] = dict(total=tot, seq=seq)
POST[gap_at]['save'] = ''.join(f'{rd(0xA000 + j):02X}' for j in range(0x50))
assert POST[gap_at]['node'] == 0x14 and POST[gap_at]['prog'] == 0x80
assert POST[gap_at]['tally'] == 0x01 and POST[gap_at]['scr'] == 0
assert arm['exit'] == 0 and rd(0xA2B4) == 0 and rd(0xA24F) == 0   # $4A arm, no bell object
REC.extend(POST[arm['frame']:])          # now ends on the gap record
json.dump(REC, open(f'{OUT}/traj18_raw.json','w'))
print(f'records: {len(REC)} = {arm["frame"]+1} in-level + '
      f'{tail_at - arm["frame"]} fade ($028D) + 1 hand-off; gap total {tot}, seq {seq}')
```

Then `python3 captures/apply_pairing.py traj18_raw.json traj18.json` exactly as today.

Splice notes that must not be lost:
- the `$028D` fires up to the arm are dropped (`POST[arm['frame']:]`), keeping the two hooks 1:1 —
  the existing `assert len(POST) == arm['frame'] + 1` (line 637) still guards that;
- the first post-arm fire is the arm frame's own end-sample, so the splice still includes one record
  per tick through tail + gap; nothing between the two hooks changed;
- the schema additions (`node`, `prog`, `goal` everywhere; `save`, `gap` on the last record) are all
  optional fields, so traj5-17 keep replaying untouched.

## 6. Acceptance

1. `cargo run -q -p sml2-runtime -- --level 0 --replay captures/traj18.json` → all checks
   `=0`, the new `node`/`prog` checks included, plus the one-shot `hand-off:` line, with no `note:`.
   Expected: the tail grows by ~34-35 ticks of gap and exactly one comparable record.
2. The capture's own asserts (gap accounting, node/prog/tally/scr on the gap record) pass — the
   capture must print them (the skill's "record the console's own evidence" rule).
3. The full sweep (`captures/sweep.sh`) stays green; `traj4_ff0.json` remains the only exemption.
4. Update `captures/README.md` (traj18 row + the sampling section) and the queue items with the
   measured numbers above.

Open follow-ups this capture defines but does not close: the map's node rewrite
(`$14`→`$13`, ~42 ticks after the first hub frame) and `sMapCursorMove` walk — the first
`overworld_map_state_0C` work items.
