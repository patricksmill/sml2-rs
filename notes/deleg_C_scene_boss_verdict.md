# Deleg C — scene/boss verdict: `scene_boss_spawn` ($0EB0 → $3C94 → bank 2 $8D9C, + the $8DA9 driver)

Read-only re-derivation, 2026-10-14. Ground truth: byte-exact reads of `baserom.gb`
(byte-identical to `sml2.gb`, 524288 B), cross-checked against `home.asm`/`main.asm` and the
port's current tree (`crates/`, `extracted/`, `PORT_QUEUE.json`). Addresses are ROM **file
offsets** unless prefixed `bankN $adr` (same byte as a bank-local address). Every load-bearing
claim below was re-read from raw bytes at the quoted offsets; disassembly comments were not
trusted where they disagreed with bytes (two of them do).

**Target case** (as queued): the arena of **level index 19 = human L20 = `sCurLevel $13`**
(the probe's node 61 → `sCurLevel 19`; manifest `level_id 19`), scene id **13**
(`extracted/levels/level20.scene.bin`; script `63 20 30 58 58 40 01 FF` at bank 5 `$4194`).

---

## 0. Headline corrections (these change the plan)

1. **`$8E6C`'s sub-image stride is +$10, not +$20.** Raw bytes at file `$8E8F`:
   `7B C6 10 E6 F0 C6 05 5F 0D 20 F0` = `ld a,e / add a,$10 / and a,$F0 / add a,$05 / ld e,a /
   dec c / jr nz`. Applied to `e=$25` that sequence is `$25→$35→$45→$55` — **net +$10 per
   iteration** — so 18-byte images are written at **$AD25, $AD35, $AD45, …** and OVERLAP by
   2 bytes. They do NOT land at `$AD25 + $20·k`. This exact pattern occurs nowhere else in the
   ROM, and no `C6 20 E6 F0 C6 05` variant exists (both greps over all 512 KiB). The "+$20"
   one-liners (`scene_boss_spawn`'s and `scene_boss_scripts`' notes) and `scene_system`'s
   elaborated "correction (c)" ("moves exactly +$20 … no overlap") are wrong;
   `scene_system`'s *earlier* statement ("literal $10 stride … consecutive
   images OVERLAP by 2 bytes … reproduce it literally") was right. The driver, meanwhile,
   genuinely walks slots at **+$20** (raw `$8DEF`: `11 20 00` = `ld de,$0020`).

2. **The target reaches record 4, not record 6.** `$8DFD`'s table is keyed on `sCurLevel`:
   `$05→0, $09→1, $0D→2, $10→3, $13→4, $17→5, else→6`. `sCurLevel $13` (19) → **b=4**, so at
   phase 0 the spawn is **`sObjectDefIndex = 4`**: count **2**, hits 6, sprite **$36 (54)**.
   Record 6 (count 1, sprite $52) is the else-record for non-listed levels.

3. **Those two facts collide on exactly this target.** Record 4 has count 2, and count ≥ 2 is
   precisely where the +$10 arithmetic diverges from the claimed +$20: the literal staging puts
   sub 2 at `$AD35` while the driver reads slot 2's def byte at `$AD45` (= `I2[16]` = 0), so on
   the console only **two** objects activate (objid 16 and 17); objid 18 is staged but never
   read as a slot def. A "+$20" port would activate a third part the console does not.
   For count = 1 records (idx 0, 5, 6, 7, 8) the stride never iterates and both readings agree.

4. **`$8DA9` is NOT needed for the spawn.** `$8D9C`'s four calls are pure RAM staging; the
   driver is reached only via `$0652` → `$3C9F` → bank 2 `$4DA9`, on the per-frame object pass.
   The "park" is caused by the `$F0` write + the already-ported `$0B2A` camera gate + the
   absence of any unfreeze, not by the spawn itself.

5. Residuals: the item correction's arena-reachability list is wrong (§1.9); "$4EAC table … no
   pointer is read for it" is imprecise (the read executes; nothing is copied — §1.4);
   `$8DA9`'s `ld [$5290],a` is a write into the banked-ROM window, a no-op — §3.

---

## 1. Claim-by-claim confirmation

### 1.1 `$3C94 → bank 2 $4D9C = four calls` — CONFIRMED
`home.asm` `$3C94` (line 7858): `ld a,2 / ld [sCurROMBank] / ld [$2100] / jp UnknownJump_0x8D9C`.
File `$8D9C` = bank2 `$4D9C` (main.asm:3552-3557): `call $3A28 / call $3A00 / call $8DFD /
call $8E6C / ret`. **The only caller of `$3C94` is the `$0EB0` site** (home.asm:2019); grep of
both .asm files finds one `call` + one definition. The sibling trampoline `$3C9F` (→ `$8DA9`)
has exactly one caller: home.asm:841 inside `$0652`.

### 1.2 The two `$8D9C` clears — CONFIRMED
`$3A28` → bank 3 `$EC40` (main.asm:9035): `ld hl,sObjectSpawnXHi / ld b,0 / xor a / loop {ld [hli],a;
dec b; jr nz}` — b starts 0, so 256 writes: clears **$AF00–$AFFF**. `$3A00` → bank 3 `$EC31`
(main.asm:9022): `ld hl,sObjectTable / ld bc,$0200 / …` — clears **$AD00–$AEFF** (512 B = 16
slots × $20). Order inside `$8D9C`: `$AF` page first, then `$AD` page, then `$8DFD`, then `$8E6C`.
`ram.asm`: `sObjectTable $AD00`, `sObjectSpawnXHi $AF00` — the cleared ranges are exactly as claimed.

### 1.3 `$8DFD` — CONFIRMED, with the target's real numbers
Table (main.asm:3603-3639): `b = 0` at `cp $05` (level 5), `+1` per `cp $09/$0D/$10/$13/$17`,
else 6 after the last inc. `sObjectDefIndex = [sBossFightPhase] + b`; `sTimerReload = 80`;
`$904E` is `hl = base + [sObjectDefIndex]*2; hl = [hl]` (main.asm:3718-3728, raw-verified
mechanism); then reads `[count, hits, sprite]`. Table1 at `$4E3F` carries **nine** pointers
(`$4E51,$4E54,…, $4E69`; raw `51 4E 54 4E … 69 4E`), records are nine 3-byte entries at `$4E51`:
`01 06 01 · 03 06 12 · 02 06 2A · 02 06 30 · 02 06 36 · 01 06 48 · 01 06 52 · 01 06 52 · 00 06 63`.
Hits is 6 for every record; idx 8 is count 0 / sprite 99 ($63). **For the target**:
`b=4`, phase 0 → index 4 → `[02 06 36]` = count 2, hits 6, sprite $36.

### 1.4 `$8E6C` — CORRECTED: stride +$10; table3 has 8 entries; count-0 read order
- First copy: `ld hl,$4E9A / call $904E / ld de,$AD05 / ld b,18 / call $3CAA` — raw `11 05 AD`
  at `$8E72` → **sObject0Def = $AD05** ✓. `$3CAA` copies B bytes `[HL+]→[DE+]` (home.asm:7871).
- Second table read: `ld hl,$4EAC / call $504E` happens **before** the count check:
  `ld a,[$AFC1] / and a / ret z` (raw `FA C1 AF A7 C8 4F 11 25 AD 06 12` at `$8E80-$8E8A`).
  So for the count-0 record (idx 8) **the pointer read DOES execute** — index 8 reads
  `$4EAC+16 = $4EBC`, the first record's bytes `01 00`, as a `$0001` pointer — and is then
  discarded; **nothing is copied from it**. "No pointer is read" should read "no pointer is
  dereferenced"; the extractor's skip and the port's `_` arm are behaviour-equivalent for the
  only observable (no sub staging), but a literal `$904E` model must not read out of bounds —
  the port's clamp is correct.
- Table3 raw at file `$8EAC`: `5E 4F 70 4F A6 4F CA 4F EE 4F 12 50 24 50 36 50` —
  **EIGHT** pointers (`$4F5E,$4F70,$4FA6,$4FCA,$4FEE,$5012,$5024,$5036`). The
  `main.asm` comment "7 entries" is wrong (it misparsed `$5036` as record data). Verified
  targets: `$4F5E→$4F70` = 18 apart; `$4F70→$4FA6` = 54; idx4's chain is `$4FEE,$5000`.
- Sub loop stride: **+$10** — see §0.1. For count = 1 the loop body runs once and the stride
  expression is computed but never used: count-1 staging is identical under either reading.
- The parenthetical confirmation in `scene_system`'s "correction (c)" ("confirmed by sObject0Def $AD05 vs
  sObject1Def $AD25 and the spawner's `ld de,$0020`") actually only confirms the *slot* stride,
  not the copy stride — it is circular and should be dropped.

### 1.5 `$0EB0`'s scene branch is a documented NON-branch — CONFIRMED
`camera.rs:511-523` (`finish_transition` doc) explicitly documents the un-ported branch and
why, and keeps the `$0EE4` arm a separate `if`. No other caller of `$3C94` exists to reach it.

### 1.6 "arming `$F0` without `$8DA9` would PARK the port" — CONFIRMED, with two precisions
- The ROM selector is exactly as stated: `$0652` (home.asm:830): `ld a,[sSceneState] / and $F0 /
  jr nz,$0665` → `call $3C9F` (bank 2 `$4DA9`); else bank 2 `$8000`. `$0B2A`'s prologue
  (home.asm:1508): same `and $F0 / ret nz` parks the **camera**.
- Precision A: the port does **not** model `$0652` at all — the runtime calls `obj.step(…)`
  unconditionally (main.rs:1359 and 2159). Its park is the already-ported `$0B2A` gate
  (`camera.rs:306: if scene_state & 0xF0 != 0 { return }`) plus the dead end: with `$F0` set and
  no driver, nothing in the port clears it except death → level reload (the `$0472` loader
  re-seeds `sSceneState` from the header, always 0 in retail — note `home $311A` clears
  `sBossFightPhase`, NOT `sSceneState`; the only in-game writer of `sSceneState = 0` is the
  boss-phase code).
- Precision B (hazard the note doesn't name): the staged boss images carry `B_STATE`
  (slot+8) = 0, which the port's own allocators treat as **free** (`objects.rs:836` finds free
  slots by `B_STATE == 0`; the `$8000` spawn scan likewise). Landing the spawn while the normal
  engine keeps running means the boss slots can be overwritten — the sharpest reason for the
  item's "model both or neither" rule.

### 1.7 The missing-deps list — CONFIRMED, one nuance
- `$92DF` (main.asm:3856): `ld a,[sMarioDead] / cp $FF / ret z / ldh a,[$FFD5] / rst $28`.
  `rst $28` at file `$0028` is the inline dispatcher (`add a,a / pop hl / add hl,de /
  ld e,[hl] / inc hl / ld d,[hl] / push de / pop hl / jp hl`) — it jumps through the `dw`
  table that immediately follows (file `$92E8`), indexed by the **staged def byte $FFD5**.
  29 entries, bank-2 targets `$5322 … $70C2` (file `$9322…$B0C2`): entry 0 = `$5322` = a bare
  `ret`; entry 3 = `$5323` = the hit-reaction body the disassembly shows at 3875+. The 29th
  entry is `$70C2`; `sMarioDead = $A224` (ram.asm:71).
- `$3A14` → bank switch to 25 (`ld a,25`), `call $66000` = bank 25 `$6000` = the sprite-list
  loader keyed by `[$FFDB]` (table base `$6092`; main.asm:38568/38685).
- `$9048` → `ld hl,$505B / jp $81DB`; `$81DB` (`de=$FFE3; a=[$FFDB]; add a; …; add hl,bc;
  copy 4 bytes`) = the 4-byte anim record fetch. Raw base bytes verified at `$81CF`/`$81DB`.
- Nuance: the port **already has** `$81A7`/`$81BD`-shaped code — the slot↔`$FFD0` staging copies
  inside `objects.rs` (its own comments cite `$81A7`/`$81BD`), including `$81A7`'s raw-verified
  `and $F0` on the slot pointer (raw `$81A7`: `21 28 AF 2A 47 7E E6 F0 6F 60 11 D0 FF 06 20 CD`).
  They are not exposed as driver-callable helpers, but the logic exists. Genuinely absent: the
  29 handlers, `$3A14`'s bank-25 loader, the `$505B` table contents.

### 1.8 The "LANDED" data claims — CONFIRMED
- `boss_spawns.json` / `lookup_boss_spawn_def`: re-ran the independent re-derivation script
  (`~/.hermes/cache/scratch/boss_verify.py`, read-only, no emulator): **all 9 entries OK** against
  `baserom.gb` tables `$4E3F/$4E9A/$4EAC`. Count of table3 entries is 8, and idx-8's sub list is
  empty — both as the extractor treats them.
- "(a) 9th record `$4E69` (count 0, hits 6, sprite 99), obj0 image `$4E9A[8] = $4F4C`" — CONFIRMED
  (image file `$8F4C`: `1A 00 00 00 88 90 53 …` = objid 26, sprite byte $53).
- "the port's `_` arm returns that image instead of a zeroed one" — CONFIRMED (`scene.rs:479-483`).
- The port's clamp note ("rather than reading past the pointer table as the ROM would") is
  accurate; index > 8 would read record bytes as pointers ($4E51/52 = `$0601` for index 9).
  Index 9 is only reachable for an `else`-level with `sBossFightPhase = 3`; phase 3 is written
  only by the final-boss transition block (main.asm:7473+, which then zeroes sSceneState), so it
  does not appear to be a live spawn path — the clamp is a safety net, keep it.

### 1.9 Arena-script list and reachability — CORRECTED
Scripts 11-16 and 19 are `6x 20 30 yy zz 40 nn FF`, script 27 is `20 FF` — CONFIRMED.
But the item's list of node-reachable arenas is wrong/incomplete (level **index**, then node):
scene map contents × node table (`extracted/map/nodes.bin`, first 130 bytes = `$4218[node]`):

| arena scene | level file (index = level_id) | sCurLevel | b | record | count | node |
|---|---|---|---|---|---|---|
| 11 | level06.scene.bin (5) | $05 | 0 | 0 | 1 | 32 |
| 12 | **level10.scene.bin (9)** | $09 | 1 | 1 | 3 | 43 |
| 13 | level20.scene.bin (19) | $13 | 4 | 4 | 2 | **61** |
| 14 | level17.scene.bin (16) | $10 | 3 | 3 | 2 | 73 |
| 15 | level24.scene.bin (23) | $17 | 5 | 5 | 1 | 53 |
| 16 | level14.scene.bin (13) | $0D | 2 | 2 | 2 | 83 |
| 19, 27 | level25.scene.bin (24) | $18 | 6 | 6 | 1 | none |

So: **every arena scene except the castle's (19/27) is node-reachable** — the note's "the ones
a map node can reach are 12 (L11), 13 (L20, node 61) and 27 (L25…)" should read "12, 13, 14,
15, 16 and 11 are all reachable (node 43/61/73/53/83/32); 19 and 27 are not". And id 12's level
is **index 9 (human L10, node 43)**, not "L11" (level11's map carries ids 17/18). The one
load-bearing part is right: **scene 13 is level index 19** and node 61 selects it.

### 1.10 "`$2x` leaves `sSceneState = 15`, the only state that makes the `& $0F` test pass" — CONFIRMED
All `sSceneState` writers (grep, both files): `home $0472` (level load: `[sLevelLayoutIndex] &
$F0` — always 0 in retail, the port already measured every header's high nibble is 0),
`home:2017` (`$F0`), `home:4315` (`15`, the `$2x` VM handler), `main 6735/7149/7477` (`0`, the
boss-phase transition blocks). So yes: 15 is the only nonzero low nibble ever written, and `$2x`
is its only writer. (Watch out for two lookalikes that write `sBossFightPhase`, not
`sSceneState`: `home $311A` and the bank-$1A ending `$68000`.)

### 1.11 Misc — CONFIRMED
- `$8D9C` "does not load VRAM": correct; the four calls touch only WRAM ($AD00-$AEFF, $AF00-$AFFF).
- The port's `finish_transition` tail (`transition=0`, `boss_defeated &= $0F`, `sfx0=5`) matches
  `$0EB0`'s `$0EE4`/`$0EF3` region (`sfx 5`, palette copies) — the branch slot is exactly there.
- "$0652 selects the driver INSTEAD OF the $8000 engine": correct ROM-side; the port has no
  equivalent and must not grow one without `$8DA9`.

---

## 2. The minimum routine set for the target case (argued from the call graph)

### The trigger, frame by frame (ROM; the port's frame order already matches)
1. Mario crosses the arena screen edge → `$0B2A`'s edge arm sets `sScreenTransition` and calls
   `$0F0B`, which latches `sSceneId = 13` from `sSceneMap` (port: `camera.rs` edge arms +
   `scene_lookup`). Same frame the scroll `$0DED` begins.
2. While the scroll runs (~24 frames at 4 px/frame), the main loop's `$20A4` runs the scene VM
   one byte per frame — the port does this at the end of the frame in both loops (main.rs
   1368/2168). Byte 1 `$63` = tileset record; byte 2 `$20` = the arm: `sSceneState = 15`
   (already ported). Later bytes: `$30 58 58` (camera + Mario Y low), `$40 01` (page teleport +
   flag), `$FF`.
3. Scroll completion → `$0DED`'s lock arm (three `jp`s + one fall-through, home.asm:1929/1954/
   1979/2007-2009) → `$0EB0`: `sScreenTransition = 0`; `ld a,[sSceneState] / and $0F / jr z,+`;
   `sSceneState = $F0`; `call DisableVBlank`; **`call $3C94`**; LCDC $C3; music 32; level `$18`
   special-case does not apply here.
4. Later in the SAME frame (home.asm 803 → 809 → 816): `$2D7D` (Mario-vs-slots interaction)
   then `$0652` — which now sees `$F0` and runs `$8DA9` on the freshly staged slots. So the
   spawn and the first driver frame are same-frame, in that order; the port's
   `cam.step → obj.step → scene_vm.step` order preserves it.

### What the target reaches, and what can be deferred
| routine | reached by the target case? | verdict |
|---|---|---|
| `$0EB0` branch (test + `$F0` + `$3C94`) | yes (trigger) | NEEDED |
| `$3C94` / `$8D9C` | yes | NEEDED |
| `$3A28 → bank3 $EC40` ($AF page clear) | yes | NEEDED (defines the baseline) |
| `$3A00 → bank3 $EC31` ($AD page clear) | yes | NEEDED (baseline; kills stale slots) |
| `$8DFD` (index/timer/record) | yes | NEEDED |
| `$8E6C` (slot images) | yes | NEEDED |
| `$8DA9` driver | **no** — only via `$0652` from the next path, same frame | deferred for the SPAWN; needed for a living boss |
| `$0652` selector | not for the spawn (per-frame, onward) | defer until the driver lands |
| `$92DF` + `$92E8` + 29 handlers, `$3A14`, `$9048/$505B`, `$81A7/$81BD` | only inside `$8DA9` | driver deps, not spawn deps |
| `$3A14` (bank 25) | inside `$8DA9` | deferrable even after: VRAM-only, invisible at state level |
| streamers/`$2323` panels | not needed; port already documents them as render-invisible | drop |

**Answers to the brief's explicit questions.** *Is `$8DA9` needed for the spawn?* No — the spawn
is `$8D9C`'s four calls, all RAM writes, and `$8DA9` is never called from the spawn path. It is
needed to keep the boss alive (and, per §3, to be the machine that consumes the slots).
*Does the spawn depend on the driver being installed?* Only on the **state byte**: `$0EB0`'s
gate is `sSceneState & $0F`, armed by the already-ported `$2x`. The driver install is what makes
the state survivable. *Smallest change that produces a spawned boss instead of a parked port?*
The spawn itself: branch + two clears + record + images (units U1/U2 below). But if the branch
lands, the port is parked (camera frozen, boss inert, slots clobberable) — so the landing order
must be **U1 → U3(-skeleton) → U2**, or U2 gated behind the driver-skeleton flag. There is no
smaller "unparked" configuration: the only exits from `$F0` are the boss-defeat code inside the
missing handlers (`$B15B`-family, which set `sBossFightPhase` and `sSceneState = 0`) and the
death/exit path.

---

## 3. The per-frame driver contract (`$8DA9`)

**Selector.** `sSceneState & $F0 != 0` (`$0652`; the port's camera gate uses the same mask).
`$0EB0` writes `$F0`; nothing else in retail gameplay writes a high nibble.

**Per call (= per frame), raw-verified** (`$8DA9`, file offsets $8DA9-$8DF6):
1. `inc [$FFF0]` — **unconditional, once per call**. Compare the normal engine: `$8000`'s
   `$812E` increments `$FFF0` only when the `sObjectUpdatePhase` toggle lands on 1
   (main.asm:2305-2312), i.e. every OTHER frame. The port models the normal path that way
   (`objects.rs:558-562`); a driver substitution must reproduce the driver's cadence, never both.
2. `sObjectSlotCount = [sObjectDefCount] + 1`; slot ptr := `$AD00`.
3. Per slot (repeat while `sObjectSlotCount` decrements to 0):
   - zero `sObjectDone ($AF3E)`, `sBossHitLatch ($AFC3)`, `[$5290]` (banked-ROM window write —
     no-op on hardware; model as nothing);
   - `$81A7`: copy 32 B slot → `$FFD0` (source = slot ptr with Lo `& $F0`);
   - if `$FFD5` (staged def byte) == 0 → skip the rest for this slot;
   - `$92DF`: if `sMarioDead != $FF`, RST-dispatch on `$FFD5` through the 29-entry table
     `$92E8` → bank-2 handler (the boss/part behaviour; runs on the staging copy);
   - `$3A14`: bank 25 `$6000` sprite-gfx load keyed by `[$FFDB]`;
   - clear `$FFE7` (the staging copy of `B_FLAGS` = slot+23 — so the handler sees this frame's
     Mario-contact flags and the driver erases them before write-back);
   - `$9048`: 4-byte anim record from `$505B[[$FFDB]]` → `$FFE3-$FFE6` (staging bytes 19-22);
   - `$81BD`: copy `$FFD0` → slot (write-back);
   - advance slot ptr `+= $20`.
4. Reads: slot bytes, `sObjectDefCount`, `sMarioDead`, `$FFD5`, `$FFDB`. Writes: the slot bytes,
   `sObjectSlotCount`, `sObjectSlotPtrHi/Lo`, `sObjectDone`, `sBossHitLatch`, `$FFF0` (+ the
   no-op `$5290`). It never touches sSceneState — the unfreeze lives in the handlers.

**Can the port run `$8DA9`'s records on its EXISTING object engine? — No for dispatch, yes for
the substrate.** The two machines share the RAM machine: 32-byte slots at `$AD00` ($20 stride),
the `$FFD0` staging window, `$81A7`/`$81BD`-shaped copies, the `$FFF0` parity counter, and the
`B_FLAGS` byte at slot+23 (whose writer, the `$2D7D` pass, runs unconditionally over
`$AD00-$AEFF` in the frame — home.asm:6040-6070 walks `ld hl,sObjectTable / add hl,de($20) /
cp $AF` — so Mario-contact flags land on boss slots for free, and the driver consumes them).
But the dispatch differs end to end:
- existing engine: handler key = the staged anim index `[$FFDB]` (staging byte 11 = image[6])
  through the mode A/B tables (`$58040`/`$59C07`) → bank-$16 ($5xxxx) handlers;
- boss driver: handler key = the staged def byte `[$FFD5]` (image[0]) through the 29-entry
  `$92E8` table → bank-2 handlers `$5322-$70C2`; anim bytes instead come from `$505B`.
So the boss records can **live** in `ObjectSim::slots` and be interacted with by the existing
`$2D7D` pass, but running them THROUGH the existing engine would index the wrong dispatch table
with the wrong byte. The alternate driver is a genuinely different per-frame machine on a shared
RAM substrate — the "spawn is the missing LINK, not missing machinery" framing is true only for
the slot/staging half.

---

## 4. State bytes to add, and the staging order

### 4.1 Addresses, target values, writers
| byte | addr | after the spawn (target, phase 0) | set by |
|---|---|---|---|
| sSceneState | $A28B | **$F0** (was 15) | `$0EB0` branch |
| sObjectDefIndex | $AFC0 | **4** | `$8DFD` |
| sObjectDefCount | $AFC1 | **2** | `$8DFD` (record[0]) |
| sBossHits | $AFC5 | **6** | `$8DFD` (record[1]) |
| sObjectDefSprite | $AFCA | **$36** | `$8DFD` (record[2]) |
| sTimerReload | $AFCD | **80** | `$8DFD` |
| sObjectSlotCount | $AFC2 | 0 (driver later sets 3 per call) | cleared by `$3A28`, set by `$8DA9` |
| sBossHitLatch | $AFC3 | 0 | cleared; driver re-zeroes per slot |
| sObjectDone | $AF3E | 0 | driver per slot |
| sObjectSlotPtrHi/Lo | $AF28/$AF29 | 0 (driver sets $AD/$00) | cleared; driver |
| $FFF0 | HRAM | unchanged by the spawn | driver (`inc`, once/call) |

Cleared wholesale first: **$AD00-$AEFF** (16 slots) and **$AF00-$AFFF** (object scratch page,
which holds all $AFC* bytes above). `sBossFightPhase ($A2DC)` is not written by the spawn — it
is read; assume 0 for a fresh fight (its writers are the boss-phase transitions and the
level-exit/ending resets).

### 4.2 Staging order (and what lands where) — target, byte-exact
Order: clear `$AF` page → clear `$AD` page → `$8DFD` (index/timer/record) → `$8E6C`:
- `$AD05` ← I0 (18 B), then `sObjectDefCount` × 18-B copies from table3[4], at `+$10` each
  (literal ROM arithmetic):
  - I0 = `10 00 00 00 30 90 34 00 00 00 00 00 00 00 00 00 00 00` ($AD05; objid 16)
  - I1 = `11 00 00 00 30 90 0E 00 00 00 00 00 00 00 00 00 00 00` ($AD25; objid 17)
  - I2 = `12 00 00 00 40 98 0E 00 00 00 00 00 00 00 00 00 00 00` ($AD35; objid 18)
- Resulting slot bytes:
  - slot0 `$AD00`: `00 00 00 00 00 | 10 00 00 00 30 90 34 …` (5 zero + I0 + 9 zero)
  - slot1 `$AD20`: `00 00 00 00 00 | 11 00 00 00 30 90 0E 00 00 00 00 00 00 00 00 00 |
    12 00 00 00 40 98 0E 00 00 00 00` — note the last 11 bytes are I2's bytes 0-10 spilling
    over the I1 copy (the 2-byte overlap at `$AD35/$AD36`, plus 9 more bytes into `$AD37-$AD3F`)
  - slot2 `$AD40`: bytes 0-6 ← I2's bytes 11-17 (= all 0), rest 0 → **def byte ($AD45) = 0**
  - slots 3-15: zero.
  Under a "+$20" implementation, I2 would land at `$AD45` and slot2's def would be 18 — the
  console difference. For count = 1 records (idx 5/6, the L24/castle targets) the two readings
  are identical and slot1 = 5 zero + I1 + 9 zero, no spill.

### 4.3 The 18-byte image, field by field (img[i] = slot+5+i; port `B_*` names)
| img | slot byte | port name | evidence |
|---|---|---|---|
| 0 | +5 ($FFD5) | B_DEF | `$92DF` RST index; `$8240`'s anim-table key for normal spawns |
| 1,2 | +6,+7 ($FFD6/7) | B_RECHI/B_RECLO | record pointer; 0 for boss parts |
| 3 | +8 ($FFD8) | B_STATE | 0 in most boss images; 6 in idx2's subs; driver ignores it |
| 4,5 | +9,+10 ($FFD9/$FFDA) | B_SCRY/B_SCRX | initial screen position (target I0: 48,144) — axis order per port names, not re-derived here |
| 6 | +11 ($FFDB) | B_SPRITE | **anim index** — `$81A7` stages it, `$9048`/`$81DB` read it (measured) |
| 7 | +12 ($FFDC) | B_DEFEAT | 0; idx3 main = $80 |
| 8 | +13 ($FFDD) | B_FLIP | handler `xor $40` (seen at `$5323`); idx5 main = $80 |
| 9 | +14 ($FFDE) | B_FLASH | handler-written |
| 10 | +15 ($FFDF) | B_FSTATE | handler state byte (`$5323` family) |
| 11 | +16 ($FFE0) | B_PHASE | handler counter |
| 12 | +17 ($FFE1) | B_PARKED | — |
| 13 | +18 ($FFE2) | B_TYPE | $20 in idx1-4 subs |
| 14-17 | +19..22 ($FFE3-$FFE6) | B_HB_TOP… cache | per-frame overwritten by the `$81DB` 4-byte fetch; the overlap spill lands here (harmless) |

---

## 5. Recommended split into landable units

Per the project's standard (self-contained `#[test]` constructing state/slots in memory, running
the handler, asserting resulting bytes), and the item's gate (cargo test + the existing replay
ONCE, quiet):

- **U1 — staging unit (no runtime wiring).** `spawn_boss(def_index)` on/in `ObjectSim` (slots +
  a small boss-state block holding the `$AFC*` bytes), implementing the two clears + `$8DFD` +
  `$8E6C` exactly, including the literal `+$10` arithmetic and the spill.
  Tests: `boss_spawn_def_index_4_stages_the_literal_slot_bytes` (the §4.2 arrays);
  `boss_spawn_count_1_stages_one_sub_without_spill` (idx 5 = L24; idx 6 = castle);
  `boss_spawn_index_8_stages_no_subs_but_keeps_obj0` (count 0; obj0 = `$4F4C` image =
  `1A 00 00 00 88 90 53 …`); `boss_spawn_clears_both_pages_first` (pre-dirty slots/state come out
  zero plus the recorded values).
- **U2 — the branch (LAND ONLY WITH U3; the item's own "both or neither").**
  `finish_transition` gains `if scene_state & 0x0F != 0 { scene_state = 0xF0; /* request */ }`.
  Because `cam.step` cannot reach `obj`, carry it as a request flag consumed by the runtime
  before `obj.step` (the ROM order is camera-pass → spawn → `$2D7D` → `$0652`).
  Tests: `finish_transition_arms_the_boss_branch` (in 0x0F → out 0xF0 + request);
  `finish_transition_without_a_low_nibble_does_not_arm` (0 and 0xF0 inputs);
  `boss_defeated_clear_is_independent_of_the_branch` (regression for the previously reverted
  half-landing).
- **U3 — driver skeleton.** `$8DA9`'s walk with the port's staging helpers, def-0 skip,
  `$FFE7` clear + write-back, `$FFF0` cadence, and handlers as `note_unported` (the objects.rs
  pattern) until U4.
  Tests: `team_driver_walks_count_plus_one_slots_and_skips_def_zero` (target slots: processes
  slot0, slot1, skips slot2; ptr ends 2×$20 past base; slotCount ends 0);
  `driver_clears_the_staged_flag_byte_on_write_back`; `driver_reports_unported_objid_16`;
  `driver_increments_fff0_once_per_call`.
- **U4 — handlers.** The `$92E8` table's bank-2 bodies, starting with objid 16/17/18
  (target entries `$60B9`/`$627E`/`$62BE`). Each handler is its own unit with slot-state tests.
- What this unlocks for captures: none of U1-U3 change any existing capture (level 1 never arms
  the low nibble), so the replay gate stays green throughout. The first runnable boss-arena
  probe (node 61) will show the camera parked and the boss inert until U4 — that is the console
  behaviour minus the fight, and it is the honest intermediate state.
- Target choice note for the parent: scene 13 / L20 is the **count-2** case (the stride-sensitive
  one). If a stride-agnostic first landing is preferred, **L24 scene 15 (index 23, node 53,
  def_index 5, count 1)** or **L6 scene 11 (index 5, node 32, def_index 0, count 1)** avoids the
  issue entirely; the L20 run's extra value is that `level_targets.md` already routes it for the
  autoscroll/space items.

---

## 6. Not verified, and what would settle it

1. **The console's live behaviour for count ≥ 2** (does the third part really never activate?).
   The bytes say the staging is `$AD25, $AD35` and the reads are `$AD05, $AD25, $AD45`; the
   consequence (slot2 def = 0 for every count-2 record; objid 18 staged but unread) follows from
   the byte-exact arithmetic, but has not been watched on hardware. **Settlement:** read
   `$AD00-$AEFF` (or OAM/part count) on the console/emulator after a count-2 arena spawn — i.e.
   the capture this item lacks. Until then: reproduce the arithmetic literally; the alternative
   ("+$20") is refuted by the bytes but is what the queue note currently states.
2. **Whether `+$10` is a shipped bug.** Unknowable from ROM bytes alone; note that
   `PORT_QUEUE.json` currently contains both statements (earlier "literal $10 … overlap" =
   matches bytes; later `scene_system`'s "correction (c)" = does not). Recommend a supervised edit of the note
   and an explicit pinning test (U1) so the choice cannot drift.
3. The 29 bank-2 handler bodies: only entries 0/3 were read (`ret`; hit-reaction pattern).
   The target's entries (16/17/18 → `$60B9`/`$627E`/`$62BE`) are not decoded — needed only for U4.
4. `$505B` table contents and `$66000`'s loader body: mechanism verified `$81DB`; contents not
   extracted. Needed for slot-byte fidelity once U3 runs handlers; not for U1/U2.
5. `sBossFightPhase == 0` at the target moment: assumed (writers are the boss-phase transitions
   and resets; not re-traced through the node-61 entry route). Settle by reading `$A2DC` when
   the arena fires. If it were 1, the spawn would be record 5 (count 1).
6. The item's "no capture has ever reached it": accepted from the queue, not re-checked.
7. `$5290`: appears only in the driver (main.asm:3574) in this disassembly; it is inside the
   banked-ROM window at that point, so the write is a no-op (MBC register window at most, value
   0). Model as nothing; flag if a mapper-level claim is ever made about it.

---

## Appendix — byte-level evidence index (all re-read from `baserom.gb`)

- `$8E3F` tbl1: `51 4E 54 4E 57 4E 5A 4E 5D 4E 60 4E 63 4E 66 4E 69 4E`
- `$8E51` records: `01 06 01 03 06 12 02 06 2A 02 06 30 02 06 36 01 06 48 01 06 52 01 06 52 00 06 63`
- `$8E6C` prologue: `21 9A 4E CD 4E 50 11 05 AD 06 12 CD AA 3C 21 AC 4E CD 4E 50 FA C1 AF A7 C8 4F 11 25 AD`
- `$8E89` loop: `06 12 CD AA 3C 7B C6 10 E6 F0 C6 05 5F 0D 20 F0 C9` ← the stride bytes
- `$8E9A` tbl2: 9 dw `BC4E CE4E E04E F24E 044F 164F 284F 3A4F 4C4F`
- `$8EAC` tbl3: 8 dw `5E4F 704F A64F CA4F EE4F 1250 2450 3650` (idx4 → `$4FEE`)
- `$8DA9`: `FA C1 AF 3C EA C2 AF 3E AD EA 28 AF 3E 00 EA 29 AF 21 F0 FF 34 AF EA 3E AF EA C3 AF EA 90 52 CD A7 41 F0 D5 A7 28 0F CD DF 52 CD 14 3A AF E0 E7 CD 48 50 CD BD 41 …`
  tail `$8DEF`: `11 20 00 19 7C EA 28 AF 7D EA 29 AF 18 C1` ← walk stride `$0020`
- `$92DF`: `FA 24 A2 FE FF C8 F0 D5 EF` + 29 dw from `$92E8` (`$5322 … $70C2`)
- `$81A7`: `21 28 AF 2A 47 7E E6 F0 6F 60 11 D0 FF 06 20 CD`
- `$81CF`: `21 35 49 FA 06 AF A7 20 03 21 37 47 11 E3 FF 06 00` (falls into `$81DB`)
- tables re-derived end to end by `boss_verify.py` (9/9 OK vs `extracted/scenes/boss_spawns.json`)
