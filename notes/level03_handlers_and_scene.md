# Level 3 (index 2) unported handler classes + the scene-latch chain for its two scene cells

Read-only investigation, 2026-10-14. No file in `/home/pmill/sml2-rs` or `/home/pmill/sml2decomp`
was modified. Every ROM address below was produced by disassembling `baserom.gb` byte-by-byte with
the scratch SM83 disassembler `/home/pmill/.hermes/cache/scratch/gbdis.py` and cross-checked against
`home.asm`/`main.asm`. Address conventions (identical to the level-20 report):

* **`0x5xxxx` / "file offset"** — the form `objects.json`, the port's `run_handler` and `sml2.sym`
  use: `bank*0x4000 + (bankaddr - 0x4000)`, e.g. file `0x58DD1`.
* **`bank16 $4DD1`** — the same byte as a bank-16 **bank-local** address (`0x4000 + (file - 0x58000)`).
  Handler bodies live in bank 16 (`$58000-$0x5BFFF`); a `call`/`jp` operand in `$4000-$7FFF` from
  bank-16 code is a bank-16 target (file = operand + `0x54000`), an operand `< $4000` is ROM0
  (file = operand).
* Bank-0 sites (`home.asm`) print as `$0DD1` etc. in code text; bank-2 trampolines as `home.asm:$3C1C → $84EA`.

Marking: **[QUOTED]** = read from `baserom.gb` bytes / from a port file at the citation. **[inference]**
= a reading not directly checked (no emulator run was performed in this investigation).

---

## PART 1 — the level-3 handler classes the port does not implement

### 1.1 The reach set (join: spawn stream × anim table)

`extracted/levels/level03.manifest.json`: `level_id 2`, `sprite_map_index 0`, **31** spawn records.
`level index 2 <= $0A` selects **table A**: [QUOTED] bank-16 `$58036-$5803A` is
`FA 06 AF / A7 / C2 04 5C` (`ld a,[$AF06] / and a / jp nz,$5C04` — mode B only when
`sSpriteMapIndex != 0`) and [QUOTED] `$5803D` is `F0 D5 EF` (`ldh a,[$FFD5] / rst $28`), the inline
`dw` table starting at file `0x58040`. Entry *n* therefore lives at file `0x58040 + 2n`; table-A
entries are bank-16 bank-local addresses. [QUOTED] the ten entries level 3 reaches (read from
`baserom.gb`):

| anim | table-A entry bytes | bank16 target | file offset | records | sprite | handler ported? |
|---|---|---|---|---|---|---|
| 15 | `e1 46` | $46E1 | `0x586E1` | 2 | 29 | yes (`handler_marker85`) |
| 17 | `12 47` | $4712 | `0x58712` | 4 | 25 | yes (`handler_hopper`) |
| 19 | `58 47` | $4758 | `0x58758` | 1 | 27 | yes (`itempose_body`) |
| 21 | `12 47` | $4712 | `0x58712` | 1 | 29 | yes (`handler_hopper`) |
| 22 | `bd 47` | $47BD | `0x587BD` | 1 | 31 | yes (`handler_pose86`) |
| 23 | `d7 47` | $47D7 | `0x587D7` | 1 | 82 | yes (`handler_pose87`) |
| 25 | `58 47` | $4758 | `0x58758` | 2 | 33 | yes (`itempose_body`) |
| **46** | **`d1 4d`** | **$4DD1** | **`0x58DD1`** | **1** | **35** | **NO** |
| **68** | **`eb 54`** | **$54EB** | **`0x594EB`** | **9** | **125** | **NO** |
| **88** | **`c5 5b`** | **$5BC5** | **`0x59BC5`** | **9** | **73** | **NO** |

(Sprite ids from `anim_to_sprite_a`; record counts from `level03.manifest.json`'s `spawns.records`.)

The port's `ObjectSim::run_handler` (`crates/sml2-core/src/objects.rs:712-739`) matches
`0x58201, 0x582A9, 0x582D1, 0x5834F, 0x584E0, 0x5862E, 0x586E1, 0x586EE, 0x586F5, 0x58712, 0x58755,
0x58758, 0x587BD, 0x587D7, 0x5B1C8, 0x5B241, 0x5B28C`, plus the `0x58100` bare-RET fixture
[QUOTED: objects.rs:712-739]. **The three bold addresses are not in that list; they fall into
`h => self.note_unported(h)` and their bodies do not run.** That is 19 of level 3's 31 records
(9 + 9 + 1) with no handler today.

Reach-set caveat, stated honestly: this is a join on each record's **initial** anim. Runtime anim
transitions (`$FFD5` writes), block-spawned items and object-spawned objects can reach handler
addresses the initial stream does not name, so the set is a *lower bound*. (Level 3's records carry
`mode_flag` 0/1 — that is the spawn flip byte, not an anim-table selector.)

**Decimal-check of the join (the level-20 correction applied here).** `objects.json` stores handler
addresses and `type_sets` as **decimal**, while the ROM bytes are hex. Worked example of the mapping
used throughout this report: `anim_profiles_a[68].handler = 365803` → `0x594EB` (365803 − 360448 =
0x14EB → bank16 `$54EB`), and `type_sets [82]` → `$52`, which is exactly the byte the body writes.
The level-20 correction was the other direction (`$586EE` does `add $66` on the **decimal** anim index:
anim 27 → 27 + $66 = `$81`, not 27-hex + $66 = `$8D`). **None of level 3's three classes uses the
`anim + N` pattern** — every `$FFE2` write in all three bodies is an immediate, and I read each body
in full (see below): anim 88 writes `$4E`, anim 68 writes `$52`, anim 46 writes `$20`. So there is no
anim-index arithmetic to get wrong here.

**Extractor accuracy for these three classes.** The level-20 report warned that `states`/`type_sets`
under-report for some anims. For level 3's three classes the extraction is **accurate**, verified
against the inline `rst $28` tables byte-by-byte:
* anim 88 → `states [367569, 367877, 367937]` = `0x59BD1/0x59D05/0x59D41` = exactly the 3 dw entries
  at `0x59BCB`; `type_sets [78]` = `$4E` = the one type written. ✔
* anim 46 → `states [364005, 364036]` = `0x58DE5/0x58E04` = exactly the 2 dw entries at `0x58DE1`;
  `type_sets [32]` = `$20`. ✔
* anim 68 → no `states` (correct: its state dispatch is the *shared* table at `0x582F7`, which the
  extractor attributed to the `$582D1` class); `type_sets [82]` = `$52`. ✔

### 1.2 Class A — `0x59BC5` (bank16 `$5BC5`), anim 88 only — 9 level-3 records

*Selected by:* table-A entry 88 (verified from the ROM table: `c5 5b`). Sprite 73.
Records at x/y: (584,240) (728,240) (888,480) (1048,496) (1160,480) (1912,240) (1928,320)
(1944,408) (1944,256) — `level03.manifest.json`.

*Body, head + inline state table* [QUOTED, file `0x59BC5`]:

```
$5BC5 (0x59BC5)  CD 85 75   call $7585        ; bank16 $7585 = the per-2-frame sprite bob
                 F0 DF      ldh a,[$FFDF]     ; B_FSTATE
                 EF         rst $28
                 D1 5B      -> $5BD1 (file 0x59BD1)   ; state 0
                 05 5D      -> $5D05 (file 0x59D05)   ; state 1
                 41 5D      -> $5D41 (file 0x59D41)   ; state 2
```

`$7585` [QUOTED, file 0x5B585] = `ldh a,[$FFF0] / and $01 / ret nz` (every 2nd frame);
`ld hl,$FFEA / inc [hl] / ld a,[hl] / cp $04 / jr nz / xor a / ld [hl],a` (B_ANIMFRAME cycles 0..3);
`ld hl,$75A2 / add hl,de / ldh a,[$FFDB] / add [hl] / ldh [$FFDB],a` with table bytes at
`0x5B5A2` = `FE 01 FF 02` → `B_SPRITE += [-2, +1, -1, +2]` cyclically. **This is the port's
`sprite_bob` verbatim** [QUOTED: objects.rs:1528-1539 with `BOB: [0xFE,0x01,0xFF,0x02]`].

*State 0* [QUOTED, file `0x59BD1`-$59BE0]:

```
$5BD1: 3E 10 E0 DE   ld a,$10 / ldh [$FFDE],a   ; B_FLASH = $10
       3E 4E E0 E2   ld a,$4E / ldh [$FFE2],a   ; B_TYPE  = $4E
       CD C6 73      call $73C6                 ; bank16 $73C6 = face Mario
       3E 01 E0 EC   ld a,$01 / ldh [$FFEC],a   ; B_T28 = 1
       C9            ret
```

`$73C6` [QUOTED, file 0x5B3C6] = `call $73E0` (compute `$AF2C` = sign(sMarioRefX − objectX) by
comparing `$AF31`/`$AF30` with the staged `$FFD0/$FFD1`), then `ld a,[$AF2C] / dec a / jr z,$73D7`:
`$AF2C == +1` → `B_FSTATE = 2`, `B_FLIP = $20`; otherwise `B_FSTATE = 1`, `B_FLIP = 0`. **This is the
port's `face_mario`** [QUOTED: objects.rs:869-886, doc comment `$5B3C6: face Mario — $FFDF = 1 (left)
/ 2 (right), $FFDD = 0 / $20`]. Note the state-0 body sets the type and flash and *returns without
writing the state byte* — the state comes from `$73C6`.

*State 1 (walk left)* [QUOTED, file `0x59D05`-$59D2C]:

```
$5D05: CD 1C 3C   call $3C1C        ; bank0 -> bank2 $44EA = left-wall probe (X-7, Y-11/Y-5)
$5D08: FA 2F AF   ld a,[$AF2F]      ; sMoveFlags
$5D0B: CB 4F      bit 1,a
$5D0D: 20 1E      jr nz,$5D2D      ; bit1 SET = wall present -> flip
$5D0F: F0 EC      ldh a,[$FFEC]     ; B_T28
$5D11: A7         and a
$5D12: 28 09      jr z,$5D1D        ; 0 -> move
$5D14: 3D         dec a
$5D15: E0 EC      ldh [$FFEC],a
$5D17: C0         ret nz            ; still counting down
$5D18: 3E 30      ld a,$30
$5D1A: E0 E9      ldh [$FFE9],a     ; B_T25 = $30 (48) : arm the step budget
$5D1C: C9         ret               ; ...and do NOT move on this frame
$5D1D: 06 01      ld b,$01
$5D1F: CD 22 3D   call $3D22        ; object X -= b (the port's mv_x_sub)
$5D22: F0 E9      ldh a,[$FFE9]
$5D24: 3D         dec a
$5D25: E0 E9      ldh [$FFE9],a
$5D27: C0         ret nz            ; still stepping
$5D28: 3E 10      ld a,$10
$5D2A: E0 EC      ldh [$FFEC],a     ; B_T28 = $10 (16) : re-arm the pause
$5D2C: C9         ret
```

*State 2 (walk right)* [QUOTED, file `0x59D41`-$59D59 + `0x59D3A`-$59D3F]: identical to state 1
except `call $3BB8` (bank2 `$443C` = right-wall probe) + `bit 0,a` on `$AF2F` for the flip, and the
move body at `$5D3A` is `06 01 / CD 32 3D / 18 E1` (`ld b,1 / call $3D32 (X += 1) / jr $5D22`) —
i.e. **the same `$FFE9` countdown tail at `$5D22`**.

*Shared flip helper* [QUOTED, file `0x59D2D`-$59D39]:

```
$5D2D: F0 DD / EE 20 / E0 DD   ldh a,[$FFDD] / xor $20 / ldh [$FFDD],a   ; B_FLIP ^= $20
$5D33: F0 DF / EE 03 / E0 DF   ldh a,[$FFDF] / xor $03 / ldh [$FFDF],a   ; B_FSTATE ^= 3 (1<->2)
       C9                      ret
```

(The `$5D33` tail is the *same bytes* the level-20 report identified as the shared `$FFDF ^= $03`
helper; here it is reached by fall-through from `$5D2D`.)

*Branch summary.* State 0 is the one-shot init: flash `$10`, type `$4E`, `face_mario` (which picks
state 1 or 2), `B_T28 = 1`. States 1/2: probe the wall on the moving side — bit set (`$22` bit1 for
left, `$11` bit0 for right) means the probe found a **solid** tile, i.e. a wall → `B_FLIP ^= $20`,
`B_FSTATE ^= 3`, return. Otherwise a two-phase counter machine on `B_T28`/`B_T25`: the counter counts
down; the frame it reaches 0 arms `B_T25 = $30` and does not move; then each frame moves the object
1 px and decrements `B_T25`; when `B_T25` hits 0, `B_T28 = $10`. Net cycle: 16 idle frames, then 48
one-pixel steps. Walls are probed on **every** frame, including the idle ones. Nothing re-reads the
dispatch byte (state changes go through `$FFDF`, read by the dispatcher on the next call).

*Calls / falls into:* `$7585` → port `sprite_bob`; `$73C6` → port `face_mario`; `$3C1C`/`$3BB8` →
port `m_left_wall`/`m_right_wall` [QUOTED: objects.rs:1883-1886 (bank2 `$843C`), 1908-1911 (bank2
`$84EA`)]; `$3D22`/`$3D32` → port `mv_x_sub`/`mv_x_add`. **Every callee already exists in the port.**

*Type behaviour:* type `$4E` → `ty & $23 == $02` with bit 6 set: the on-object arm does not apply, so
the `$2E23` bounce path runs (stompable; harmful on the side-contact path that tests bit 6).

*Registers the port must model:* `B_FLASH` (`$FFDE`), `B_TYPE` (`$FFE2`), `B_FSTATE` (`$FFDF`),
`B_FLIP` (`$FFDD`), `B_T25` (`$FFE9`), `B_T28` (`$FFEC`), `B_ANIMFRAME` (`$FFEA`), `B_SPRITE`
(`$FFDB`), `sMoveFlags`. **No new sim state** — all of these are existing fields/constants
[QUOTED: objects.rs:104-128].

*LOC estimate:* **~55-65 lines** in the objects.rs idiom — a head + a 3-state match, plus a new
~12-line "pause/step counter" helper both move states share (there is no ported equivalent: the
closest single-block countdown is `handler_stun`'s `$58364` arm). Plus 2-3 unit tests (the counter
cadence, the wall flip in both directions, the state-0 one-shot).
*Closest ported template:* **`handler_fallwalker` (`0x582D1`)** for the overall shape
(`sprite_bob` + `B_FSTATE` match + `m_*_wall` + `B_FLIP ^= $20`) — but note anim 88's walk states
are *not* the same as the fallwalker's: they move every frame (no `$FFF0 & 1` gate), they use the
counter machine instead of the ledge check, and they do not touch type after state 0.

### 1.3 Class B — `0x594EB` (bank16 `$54EB`), anim 68 only — 9 level-3 records

*Selected by:* table-A entry 68 (`eb 54`). Sprite 125. Records: (1288,240) (2136,192) (2328,752)
(2488,752) (2648,752) (2808,752) (3352,736) (3368,128) (3432,736).

**This class is a clone of the already-ported `0x582D1` handler (`handler_fallwalker`) with a
different type byte.** [QUOTED, file `0x594EB`]:

```
$54EB: CD 85 75   call $7585       ; the same sprite_bob
$54EE: F0 DF      ldh a,[$FFDF]
$54F0: A7         and a
$54F1: C2 F5 42   jp nz,$42F5      ; state != 0 -> the shared state dispatch (file 0x582F5)
$54F4: 3E 52      ld a,$52
$54F6: E0 E2      ldh [$FFE2],a    ; B_TYPE = $52
$54F8: C3 DD 42   jp $42DD         ; -> the shared ground-fall body (file 0x582DD)
```

versus the ported original [QUOTED, file `0x582D1`]:

```
$582D1: CD 85 75 / F0 DF / A7 / 20 1C   ; state != 0 -> $82F5 = the SAME $42F5
$582D9: 3E 56       ld a,$56            ; B_TYPE = $56
$582DB: E0 E2       ldh [$FFE2],a
$582DD: CD 40 3B     call $3B40           ; <- falls through into the shared body
```

The shared bodies, quoted once (they are the port's `handler_fallwalker` body) [QUOTED]:

```
$42DD (0x582DD): CD 40 3B   call $3B40   ; m_ground_own (ground at own position)
                 FA 2F AF   ld a,[$AF2F]
                 CB 5F      bit 3,a      ; ground present?
                 20 07      jr nz,$42EE
                 CD 51 74   call $7451   ; the c=$15 / $7488 step picker (= port fall_step)
                 CD 3D 3D   call $3D3D   ; object Y += b
                 C9         ret
$42EE:           AF / E0 E0             ; B_PHASE = 0 (restart the fall curve)
                 CD C6 73   call $73C6   ; face_mario
                 C9         ret
$42F5 (0x582F5): 3D / EF               ; dec a / rst $28
                 table at $42F7: FB 42 1A 43 -> $42FB / $431A
$42FB (0x582FB): F0 F0 / E6 01 / 20 38 ; odd $FFF0 -> $4339 (the ledge/ground check)
                 06 01 / CD 22 3D       ; b=1 / X -= 1
                 CD 1C 3C / FA 2F AF / CB 4F / C8   ; left-wall probe; no wall -> ret
                 3E 02 / E0 DF          ; B_FSTATE = 2
                 F0 DD / EE 20 / E0 DD / C9          ; B_FLIP ^= $20
$431A (0x5831A): same with CD 32 3D (X += 1), CD B8 3B (right wall), bit 0, state = 1
$4339 (0x58339): the ground check; walking off a ledge -> B_FSTATE ^= 3 + flip (port ledge_reverse)
```

*Branch summary.* State 0: set type `$52`, then the shared fall arm — ground probe; ground under the
object → `B_PHASE = 0` + `face_mario`; otherwise step the fall curve (`$7451`, table `$7488`,
sentinel `$15` = 21) and move down. States 1/2 (reached through the shared `$42F5` `dec a` dispatch):
on even `$FFF0` move 1 px and probe the wall on the moving side, flipping on a wall; on odd `$FFF0`
run the ground check and reverse on a ledge. No re-read of the dispatch byte.

*Calls:* everything is ported — `sprite_bob`, `m_ground_own`, `fall_step` (= `$7451`, [QUOTED:
objects.rs:891-898, "step `+16` through `FALL_SPEED` (cap 21)"]), `mv_y_add`, `face_mario`,
`m_left_wall`/`m_right_wall`, `ledge_reverse` (`$58339`). **Nothing new is needed except the type
byte.**

*Type behaviour:* `$52` → `ty & $23 == $02` (bit 6 set) — same class as `$56`: stompable.

*LOC estimate:* **~4-8 lines** — add `0x594EB => self.handler_fallwalker_52(...)`, or better,
parametrize `handler_fallwalker` with the type byte (`$56`/`$52`) and add the second match arm; plus
1-2 unit tests that pin the `$594EB` type (a test that would fail if the arm reused `$56`).
*Closest ported template:* **`handler_fallwalker` (`0x582D1`) — literally the same body, one
immediate different.**

### 1.4 Class C — `0x58DD1` (bank16 `$4DD1`), anim 46 only — 1 level-3 record

*Selected by:* table-A entry 46 (`d1 4d`). Sprite 35. Record: (1712,176).

*Body* [QUOTED, file `0x58DD1`-$58E11]:

```
$4DD1 (0x58DD1): F0 DF      ldh a,[$FFDF]
                 A7         and a
                 20 09      jr nz,$8DDF      ; -> $4DDF (file 0x58DDF)
                 3E 20      ld a,$20
                 E0 E2      ldh [$FFE2],a    ; B_TYPE = $20
                 3E 02      ld a,$02
                 E0 DF      ldh [$FFDF],a    ; B_FSTATE = 2
                 C9         ret
$4DDF (0x58DDF): 3D         dec a
                 EF         rst $28
                 table at $4DE1: E5 4D 04 4E -> $4DE5 (state 1, file 0x58DE5)
                                              -> $4E04 (state 2, file 0x58E04)
$4E04 (0x58E04): F0 E7      ldh a,[$FFE7]    ; staging +23 = the port's B_FLAGS
                 CB 5F      bit 3,a
                 C8         ret z            ; wait until Mario stands on it
                 3E 01      ld a,$01
                 E0 DF      ldh [$FFDF],a    ; B_FSTATE = 1
                 3E 1C      ld a,$1C
                 E0 EC      ldh [$FFEC],a    ; B_T28 = 28
                 C9         ret
$4DE5 (0x58DE5): F0 EC      ldh a,[$FFEC]
                 A7         and a
                 28 04      jr z,$8DEE
                 3D         dec a
                 E0 EC      ldh [$FFEC],a
                 C0         ret nz           ; still counting down
$4DEE (0x58DEE): CD 51 74   call $7451       ; the c=$15 / $7488 picker (= port fall_step)
                 CD 3D 3D   call $3D3D       ; object Y += b
                 CD DF 3C   call $3CDF       ; <<< the carry helper (bank 0, home.asm:7911)
                 CD 40 3B   call $3B40       ; m_ground_own
                 FA 2F AF   ld a,[$AF2F]
                 CB 5F      bit 3,a
                 C8         ret z            ; no ground -> keep falling
                 CD D7 72   call $72D7       ; ground -> the $5B2D7 removal (port remove_object)
                 C9         ret
```

*Branch summary.* State 0 (one shot): type `$20`, state 2. State 2: waits for `B_FLAGS` bit 3 ("Mario
is standing on this object"), then state 1 with `B_T28 = 28`. State 1: count `B_T28` down (and note
the count-down frame that reaches 0 falls THROUGH to the move), then every frame: step the `$7488`
curve (`$7451`, sentinel `$15` = 21 → after 21 calls the speed sticks at `$03`), move the object down,
carry Mario, ground-probe, and on ground call `$72D7` (the removal/despawn). No re-read of the
dispatch byte.

*The carry helper `$3CDF`* [QUOTED, `home.asm:7911-7919`, bytes `FA E7 FF / CB 5F / C8 / FA 38 A2 /
80 / EA 38 A2 / 21 C0 FF / 18 E6`]:

```
$3CDF: ldh a,[$FFE7] / bit 3,a / ret z          ; same gate as state 2
       ld a,[sScrollDeltaDown] / add b / ld [sScrollDeltaDown],a   ; $A238
       ld hl,$FFC0
       jr $3CD7                                  ; $3CD7: add b to the 16-bit $FFC0/$FFC1 (Mario Y)
```

Two facts about it, both [QUOTED]:
1. The gate byte `$FFE7` is the **staging copy of slot+23** (the port's `B_FLAGS = 23`), and its bit 3
   is set by the on-object arm: `home.asm:6119-6126` (`ld a,255 / ld [sMarioOnObject],a` … `ld a,8 /
   ld [hl],a` on slot+23), cleared by the `$818D` tail when `sClearStageFlags` (set every slot update
   at `$5802B` → file `0x5802B`, `ld a,$01 / ld [$AF09],a`). The port already reproduces both ends
   (`respond` sets `self.slots[i].b[B_FLAGS] = 8`; `active_update` clears it when `clear_stage_flags`)
   [QUOTED: objects.rs:2240-2247, 656-659, 697]. And the port's frame order is right: `step` =
   `collide` ($2D7D) **then** `engine` ($8000) [QUOTED: objects.rs:463-469], so the touch's +23 = 8 is
   visible to the same frame's handler, as in the ROM.
2. The `sScrollDeltaDown` add is **dead**: the camera's down arm writes that byte before reading it
   (`home.asm:1712` writes, `home.asm:1745` reads) and no other path reads it, so only the
   **Mario-Y add** is observable [inference on "dead", from reading every reader of `$A238`: it is
   read at `$1745`/`$0CEC` only]. The port models direct Mario-Y writes with
   `mario_y_adjust: Option<u16>`, applied after the object pass [QUOTED: objects.rs:355-356].
   Flag for the implementation: `Option` means the LAST carrier of a frame wins where the ROM adds
   each carrier's `b` — with 2+ carriers stacking the same frame the port would lose one add
   [inference].

*Calls:* `$7451` → port `fall_step`; `$3D3D` → `mv_y_add`; `$3B40` → `m_ground_own`; `$72D7` → port
`remove_object` (file `0x5B2D7`; [QUOTED: objects.rs:803 area, "$5B2D7: removal — clear the record
marker when not in remove-mode"]). `$3CDF` is **new** (~8 lines: gate on `B_FLAGS & 8`, set
`mario_y_adjust`).

*Type behaviour:* type `$20` → `ty & $23 == $20` → the **on-object arm**: stand on it (`on_object =
true`, `B_FLAGS = 8`), i.e. it is a platform — which is exactly what state 2 waits for. (`$20` bit 6
clear → not harmful.)

*LOC estimate:* **~35 lines** (head + 2 states + the picker/ground/removal wiring) + the ~8-line
carry helper + 2 tests (the state-2 gate and the carry write). Template: `handler_fallwalker`'s
state 0 for the fall+ground shape, `handler_stun`'s `$58364` countdown for the timer shape; the
carry has no ported template.

### 1.5 Per-class table (summary)

| handler | anims | 1-line behaviour | LOC est. | template | verified? |
|---|---|---|---|---|---|
| `0x59BC5` (bank16 $5BC5) | A:88 | sprite-bob head; state 0 sets flash `$10`/type `$4E`/`face_mario`; states 1/2 walk left/right 1 px with a 16-idle + 48-step counter machine (`B_T28`/`B_T25`) and flip on the side wall probe | ~55-65 | `handler_fallwalker` (`0x582D1`) + `handler_stun`'s countdown | ROM QUOTED; no capture |
| `0x594EB` (bank16 $54EB) | A:68 | a type-`$52` clone of the ported `$582D1`: same `$7585` head, same `$42DD` fall body, same `$42F5`/`$42FB`/`$431A`/`$4339` walk states | ~4-8 | `handler_fallwalker` (`0x582D1`) — literally the same body | ROM QUOTED; no capture |
| `0x58DD1` (bank16 $4DD1) | A:46 | waits for Mario to stand on it (`B_FLAGS` bit 3), then falls down the `$7488` curve, carrying Mario (`$3CDF`: Mario Y += b) and removing itself on ground (`$72D7`) | ~35 (+~8 carry) | `handler_fallwalker` state 0 + `handler_stun` countdown | ROM QUOTED; no capture |

Nothing in any of the three needs new sim state; all state lives in the staged slot bytes the port
already names (`B_TYPE` `$FFE2`, `B_FSTATE` `$FFDF`, `B_FLASH` `$FFDE`, `B_FLIP` `$FFDD`, `B_T25`
`$FFE9`, `B_T28` `$FFEC`, `B_ANIMFRAME` `$FFEA`, `B_SPRITE` `$FFDB`, `B_PHASE` `$FFE0`, `B_FLAGS`
`$FFE7`, plus the engine-level `move_flags` and `mario_y_adjust`).

---

## PART 2 — what the scene path actually needs for level index 2

### 2.0 The two cells, as extracted (independently re-checked here)

[QUOTED] `extracted/levels/level03.edge.bin[4] = $0E`, `[21] = $0D`;
`extracted/levels/level03.scene.bin[4] = 7`, `[21] = 6`; every other byte of both 48-byte files is 0.
[QUOTED] the bank-5 script table: id 6 → `$4177` = `40 03 FF`; id 7 → `$417A` = `40 16 FF`
(re-read from `extracted/scenes/script_table.bin`). So the two scenes are `$4x` teleports to cell
(Y page, X page) = (0,3) and (1,6) respectively, and each ends at `$FF` — **level 3's scene path
contains no other opcode class**.

The index is `(Yhi & $0F) * 16 + (Xhi & $0F)` — but the two lookups index **different entities**
(quoted in 2.1): the *edge* byte comes from the **camera's** cell (`$FFC9`/`$FFCB`), the *scene* id
from **Mario's** cell (`$FFC1`/`$FFC3`). So the two facts "cell i4 has edge `$0E`" and "cell i4
latches scene 7" are about different positions; a capture must stand Mario on the scene cell while
the camera's cell authorizes the edge. Cell i4's `$0E` has bits 1/2/3 (left/up/down, **no right**),
cell i21's `$0D` has bits 0/2/3 (right/up/down, **no left**) — [QUOTED from edge.bin], so the
reachable transition directions are constrained per cell. Caveat [inference]: the trigger frame
forces Mario to the extreme left/right of the screen (`sMarioScreenX` per main.asm:440-449 =
`($FFC2 − $FFCA) + 96`, and `sMarioScreenY` = `($FFC0 − $FFC8) + 98`), so whether Mario's cell is
exactly 4 (or 5) at the trigger frame depends on his lo byte; the capture's own pages settle it.

### 2.1 (a) The trigger chain that latches `sSceneId`, and what the port does

**Step 1 — `$0B2A` prologue** [QUOTED, `home.asm:1508-1516`]:

```
$0B2A: ld a,[sSceneState] / and $F0 / ret nz     ; inside a scene -> the camera parks
       ld a,[sBossDefeated] / and a / jr z,$0B39 ; zero -> continue
       and $F0 / ret z                           ; nonzero, high nibble clear -> park
```

**Step 2 — the edge byte** [QUOTED, `home.asm:1518-1532`]: a `sScreenTransition != 0` frame jumps to
`$0DED` (the scroll); otherwise

```
ldh a,[$FFC9] / swap a / ld b,a        ; camera Y hi -> the high nibble of the index
ldh a,[$FFCB] / or b / ld e,a          ; | camera X hi (the engine keeps it & $0F on every write)
ld d,0 / ld hl,sScreenEdgeMap / add hl,de / ld a,[hl] / ldh [$FF98],a
```

**Step 3 — the four hard-edge triggers** (each sets `sScreenTransition` and calls the latch):

| dir | quote (home.asm) | conditions | warp before the call? |
|---|---|---|---|
| right | `1539-1552`: `$0B5E` `bit 0,$FF98`; `$FFCA == $B0`; `sMarioScreenX >= $A1` → `ld a,1 / ld [sScreenTransition],a / call $0F0B` | camera X lo exactly `$B0`, Mario at screen X ≥ `$A1` | no |
| left | `1612-1632`: `$0BE5` `bit 1,$FF98`; `$FFCA == $50`; `sMarioScreenX < $0F` → `sScreenTransition = 2`; `$FFC2 = 0`, `$FFC3 = ($FFC3+1) & $0F`; `call $0F0B` | camera X lo exactly `$50`, Mario at screen X < `$0F` | **yes — X page +1 before the latch** |
| down | `1716-1728`: `$0CEC` `bit 3,$FF98`; `$FFC8 == $C0`; `sMarioScreenY >= $90` → `sScreenTransition = 8`; `call $0F0B` | camera Y lo exactly `$C0` | no |
| up | `1776-1794`: `$0D05` `bit 2,$FF98`; `$FFC8 == $48`; `sMarioScreenY < $1B` → `sScreenTransition = 4`; `$FFC0 = 0`, `$FFC1 = $FFC9`; then `ld a,[sSceneState] / and $F0 / call z,$0F0B` | camera Y lo exactly `$48` | **yes — Mario Y page := camera Y page, and the latch is gated on `sSceneState & $F0 == 0`** |
| (autoscroll) | `1869-1882` `$0DAF`: `bit 0,$FF98`; `$FFCA >= $B0`; `sMarioScreenX >= $A1` → transition 1; `call $0F0B` | not reachable on level 3 (`auto_scroll == 0`) | no |

**Step 4 — `$0F0B`, the latch itself** [QUOTED, `home.asm:2052-2069`]:

```
$0F0B: ld a,255 / ld [sMapEventFlag],a
       ldh a,[$FFC1] / swap a / ld e,a          ; MARIO Y hi -> index high nibble
       ldh a,[$FFC3] / and $0F / add e / ld e,a ; | (MARIO X hi & $0F)
       ld d,0 / ld hl,sSceneMap / add hl,de / ld a,[hl] / ld [sSceneId],a
       xor a / ld [sPaletteFadeStep],a / ret
```

**Port status: reproduced, step for step** [QUOTED, `crates/sml2-core/src/camera.rs`]:
* prologue `if self.scene_state & 0xF0 != 0 { return; }` and the boss gate — `step`, lines 305-311
  (the boss gate is `boss_defeated != 0 && boss_defeated & 0xF0 == 0`, which is the ROM's
  `and a / jr z` + `and $F0 / ret z` pair, byte for byte).
* the edge lookup on the camera's cell — `edge_bits()` (lines 129-135: `yi * 16 + xi` from
  `self.y >> 8` / `self.x >> 8`).
* the right trigger — step lines 341-346 (`edges & 1`, `(self.x as u8) == 0xB0`, `sx >= 0xA1` →
  `transition = 1; scene_lookup`); left — 366-373 (`edges & 2`, `== 0x50`, `sx < 0x0F` →
  `transition = 2`; `sim.x = (page + 1) & 0x0F << 8`; `scene_lookup`); down — 404-416; up — 429-440
  (`sim.y = (camera page) << 8` then `if self.scene_state & 0xF0 == 0 { scene_lookup }`); autoscroll —
  `run_autoscroll` lines 278-288.
* the latch arithmetic on **Mario's** pages — `scene_lookup`, lines 532-538:
  `idx = (((sim.y >> 8) & 0x0F) << 4) | ((sim.x >> 8) & 0x0F)`, `map_event_flag = 0xFF`,
  `palette_fade_step = 0`. ✔ Also the two camera tests pin level 3's real cells
  [QUOTED: camera.rs:1165-1214, `scene_lookup_latches_the_id_from_the_cell_under_mario`,
  `scene_lookup_masks_the_cell_and_tolerates_a_short_map`].
What camera.rs does **not** do: nothing — the trigger + latch chain is complete; the *consumer*
(the scene VM) is where the gap is (2.3).

### 2.2 (b) What `$0EB0` does after the scroll completes; the low-nibble test

[QUOTED, `home.asm:2009-2031`]:

```
$0EB0: xor a / ld [sScreenTransition],a          ; the scroll is over
       ld a,[sSceneState] / and $0F / jr z,$0EE4 ; *** the low-nibble test
       ld a,240 / ld [sSceneState],a             ; $F0: park the camera + select $8DA9
       call DisableVBlank / call $3C94           ; bank-0 trampoline -> bank 2 $8D9C
       ld a,195 / ldh [rLCDC],a                  ; $C3
       ld a,32 / ld [sMusicRequest],a / ld [sLevelSong],a
       ld a,[sCurLevel] / cp $18 / jr nz,$0EE4   ; the Wario castle gets music $21+phase
$0EE4: ld a,[sBossDefeated] / and $F0 / jr z,$0EF3 / ... low nibble kept
       ld a,5 / ld [sSfxRequest0],a ; the three level palettes restored ; ret
```

**Every writer of `sSceneState` in the ROM** (scanned from the listings + `EA 8B A2` byte search):
`home.asm:609-611` (level init: `ld a,[sLevelLayoutIndex] / and $F0 / ld [sSceneState],a`),
`home.asm:2016-2017` ($0EB0 → 240), `home.asm:4314-4315` ($2x → 15), and `main.asm:6735/7149/7477`
(the bank-2 boss-phase advances → 0). The only writer that can set a **nonzero low nibble** is `$2x`.
Level 3's scripts are `40 16 FF` / `40 03 FF`: no `$2x`, and its header's `sSceneState` seed is
`layout_index & $F0 = 1 & $F0 = 0` [QUOTED: `extracted/levels/level03.manifest.json`
`header.layout_index = 1`, `header.scene_state = 0`; raw header byte 18 = `$01` = the layout index].
**Confirmed: for a `$4x`-only scene the `$0EB0` low-nibble branch is MOOT** — `sSceneState` stays 0,
so `$3C94`/`$8D9C` are never called and `sSceneState` is never armed to `$F0`.

**Port status:** `CameraSim::finish_transition` [QUOTED: camera.rs:524-530] does `transition = 0`,
the `boss_defeated & $F0` mask and `sfx0 = 5` — i.e. exactly the `$0EE4` tail, and it deliberately
omits the scene branch with a doc comment saying the omission belongs to the boss system and that
arming `$F0` without the `$8DA9` driver would hang [QUOTED: camera.rs:511-523]. **For level 3 that
omission is correct and behaviourally invisible** (the branch is unreachable). Two caveats to keep
honest: (i) the port's omission is only *safe* for a `$4x`-only scene — a `$2x` scene in the port
would set `scene_state = 15` via `SceneVm::op_2x` [QUOTED: scene.rs:303-307] and then `finish_transition`
would not do the `$3C94` load; the port records `boss_arena` instead and is documented as not
spawning the boss (queue `scene_boss_scripts`); (ii) the ROM's `$0EB0` also restores the three level
palettes and requests sfx 5 — the port does the sfx and leaves the palettes to the renderer.

### 2.3 (c) The `$4x` handler's exact write set, and the port's `op_4x`

[QUOTED, `home.asm:4649-4676`] `$2323` (called by the `$40` dispatch arm at `home.asm:4419-4425`):

```
$2323: inc hl
       ld a,[hl] / swap a / and $0F / ldh [$FFC9],a / ldh [$FFC1],a
          ; OPERAND HIGH NIBBLE -> camera Y hi ($FFC9) AND Mario Y hi ($FFC1)
       ld a,[hli] / and $0F / ldh [$FFCB],a / ldh [$FFC3],a
          ; OPERAND LOW NIBBLE  -> camera X hi ($FFCB) AND Mario X hi ($FFC3)
       push hl
       call $2728                       ; wait one VBlank
       ld a,[sScreenTransition] / cp $01 / jr z,$2359 ; the direction streamers, per bit
       ... cp $02 -> $23E4 ; cp $04 -> $2524 ; cp $08 -> $246F ; else pop hl / ret
```

and the dispatcher's tail [QUOTED, `home.asm:4422-4424`]: `call $2323` then
`ld a,1 / ld [sSceneTransitionFlag],a`.

So: **only the four high bytes are written** (`$FFC9`, `$FFC1`, `$FFCB`, `$FFC3`); the low bytes
(`$FFC8`, `$FFC0`, `$FFCA`, `$FFC2`) are genuinely untouched — verified by reading every store in
`$2323`-`$2360`; the operand byte is read twice (once per nibble). The `sSceneTransitionFlag = 1`
write is *outside* the handler, in the dispatch arm.

**Port status:** `SceneVm::op_4x` [QUOTED: scene.rs:339-347] writes
`cam.y = (cam.y & 0x00FF) | y_page << 8`, `sim.y` likewise, `cam.x`/`sim.x` with `x_page`, and then
`cam.scene_transition_flag = 1`. **The write set and the flag match the ROM exactly.**

Differences to flag (none is in the current write set):
1. **`SceneVm::start()` has no caller** — [QUOTED: `grep scene_vm\.|\.start()` over `crates/` finds
   only `SceneVm::new`, `scene_vm.step` (main.rs:1345 replay, main.rs:2120 play), and
   `scene_vm.unported()` (main.rs:1929); `start()` (scene.rs:226-229) is called only from the
   unit-test helper `latched()` at scene.rs:480]. Consequence [inference from the code, high
   confidence]: when `$0F0B` latches a scene, `SceneVm::step` finds `cam.scene_id != 0`,
   `self.entering == false` and `self.pc == 0`, and `scripts.byte_at(0)` is `None` → it returns
   `SceneStep::BadCursor` **without running a script byte**. The latched path is therefore dead in
   both frame loops today, and the return value is discarded, so nothing prints. **The level-3
   capture is exactly what will expose this: the console teleports and the port will not.**
2. The ROM's `$4x` spends **five frames** (its own `call $2728` + three streamer frames + the
   dispatcher's `$21E8` wait) where the port advances the cursor one opcode per frame [inference from
   the quoted code]. Invisible to `x`/`y`/`camx`/`camy`; it shifts only *when* `$FF` runs, i.e. when
   `sSceneId`/`sSceneTransitionFlag` clear — a capture that records those bytes will show it.
3. Even with `start()` wired, the entry wait model places the first byte one frame later than the
   ROM [inference]: the port returns `Entering` on the latch frame and runs the byte on the *next*
   frame's end; the ROM's `call $2728` (home.asm:5163-5175: `halt` then wait for `$FF82`, then
   `inc $FF97`) completes during the VBlank *before* the next frame's state pass, so the teleport is
   visible to that next frame's passes in the ROM. The capture's frame numbers around the teleport
   settle whether the port needs the byte to run in the same pass that ends the wait.
4. The streamers are not modelled — see 2.5; invisible by construction.

### 2.4 (d) The `$20A4` call position

[QUOTED, `home.asm:263-280`] the main loop is: `call $02AD` (the `$FF9B` state dispatch — this is
where the in-level body `$05E4` runs: player `$0F71`, camera `$0B2A`, objects `$2D7D`/`$8000`) →
`UpdateSound` (unless the state is `$04`/`$18`/`$13`) → **`call $20A4`** (line 274) → read the pad →
loop. So `$20A4` runs **after** the in-level pass, once per frame, in every state.

**Port status:** `scene_vm.step(&mut sim, &mut cam, &scripts)` is called after the object pass and its
write-back: main.rs:1341-1345 (`run_replay`) and main.rs:2115-2120 (`run_play`), both with a comment
quoting home.asm:274 [QUOTED]. That is the same position (and both loops call it, so the two-loop
parity hazard does not apply). The port also reproduces the `$20A4` head's no-scene guard
(`cam.scene_id == 0` → `end_scene()` = the `$21EE` epilogue) [QUOTED: scene.rs:242-248], which the
ROM also runs on every scene-less frame.

### 2.5 (e) The direction streamers and the BG-window fillers — observable?

[QUOTED, `home.asm:4679-4741` ($2359 right), `4743-4805` ($23E4 left), `4807-4886` ($246F down),
`4889-…` ($2524 up); fillers `home.asm:1232-1349` ($0969, vertical) and `1351-…` ($0A35,
horizontal)]. Their write set is: `$FFAF/$FFB0` (the VRAM transfer source pointer → `$9800`-page
tiles), `$FFAA/$FFAB`, `$FFAE` (a 16-row counter), `sMapBlockPtrHigh/Low`, `sMetatileTile0-3`,
`$FFCC/$FFCD` + `$FFCE/$FFCF` (the destination Y/X pages), `sWaterCurrentState = $FF`, and
`sMapScrollFlags` (bits 4/5 cleared with `$3F`/`$CF`). Everything else is VRAM writes to `$9800`
(one metatile row per call, 16 rows per page), plus a `call $2728` frame wait per strip.

**Verdict: no, nothing here is observable through a replay check on this level, for four independent
reasons** [QUOTED for the write set; the non-observability is inference from the harness's check
list, main.rs:1464-1760: x, y, hvel, vvel, ground, timer, dir, frame, camx, camy, edge, en_r, en_l,
sptr, spx, sdir, standby, active, odd, …, objslots, slots_raw, plus exit/lowtime/fade/tally]:
1. They never touch Mario, the camera value, the object slots, or `$FF97`; their latches
   (`$FFCC-$FFCF`, `$FFAA/$FFAB`, `$FFAF/$FFB0`) are not recorded or checked.
2. Their only engine-visible byte, `sMapScrollFlags`, is not a checked field (main.rs has no
   `map_scroll_flags` check; `grep map_scroll_flags crates/sml2-runtime/src/main.rs` = 0 hits).
3. They are reached **only** through `$2323`'s `sScreenTransition` arm, and the port's renderer draws
   from the fully resident decompressed layout, so the `$9800` window they fill has no consumer here
   — this is what scene.rs's module doc already records [QUOTED: scene.rs:56-64].
4. The captures record `x/y/camx/camy` as bytes and the port derives them from the sim; a VRAM-only
   writer cannot change them.

Same verdict for `$0969`/`$0A35` being "BG-window fillers": both build a `$9800` destination and
copy metatile rows from the level bank; neither reads or writes any state the harness compares
[QUOTED: home.asm:1232-1349, 1351-1400]. (Their *frame cost* — the `call $2728` per strip — is the
one thing a scene-state–recording capture would see; see 2.3's difference 2.)

### 2.6 What a capture would have to observe to settle the unverified parts

A sibling agent is recording this capture now (level index 2, `--level 2`; node 34 per
`captures/probe_level_select.py`). To close the open items it should sample, per logic frame, the
following (all already in the `trajectory_capture20.py` style, i.e. `rd()` on the address):

* **The latch and its lifecycle** — `sSceneId $A28E` (0 → 7 or 6 on the latch frame → 0 at the END),
  `sSceneIdHi $A28F` (expected 0 always), `sSceneTransitionFlag $A24A` (set by `$4x`, cleared by
  `$21EE`), `sSceneTransitionLatch $A249` (the flag's value at the END), `sSceneState $A28B`
  (expected 0 throughout — this is the MOOT check), `sScreenTransition $A20E` (1/2/4/8 during the
  scroll, 0 after `$0EB0`), and `sSceneDoneFlag $A2A8` if a flag byte is wanted twice over.
* **The teleport itself** — the four pages: `$FFC9`/`$FFC1` (camera/Mario Y page) and
  `$FFCB`/`$FFC3` (camera/Mario X page), plus the low bytes `$FFC8`/`$FFC0`/`$FFCA`/`$FFC2` to prove
  they are unchanged. For id 7 (`40 16`) expect pages (1,6); for id 6 (`40 03`) expect (0,3).
* **Which frame** the pages jump relative to the latch frame and the trigger frame — this is what
  settles the port's `start()`/`Entering` timing (2.3 differences 1-3). Record a per-frame `tick` and
  the `$FF98` edge byte so the trigger frame can be identified, and `$FF97` if the two-frame cadences
  around it need to be distinguished.
* **For the new handler classes** — the object slots are already captured (`slot_bytes`); if the
  route comes near them, also worth sampling: `$AF2F` (sMoveFlags) for anim 88/68 wall flips and
  anim 46's ground probe, the staged `$FFDF`/`$FFDE`/`$FFE2`/`$FFEC`/`$FFE9` (state/flash/type/
  counters), and `$FFE7` (slot+23 bit 3 — the anim-46 carry gate) with Mario's `$FFC0/$FFC1` to
  catch the carry.
* **Reachability note for the route**: the scene cells are (Yhi 0, Xhi 4) for id 7 and (Yhi 1,
  Xhi 5) for id 6; cell i4's edge byte `$0E` has no right bit and cell i21's `$0D` has no left bit
  [QUOTED from edge.bin], so the transition that latches each id must be chosen per cell (and the
  left/up arms warp Mario's page *before* `$0F0B` reads it — see 2.1's table).

---

## 3. Flags — claims I could not verify to the standard of the rest

1. **No emulator was run.** Everything here is ROM bytes + source reading; the handler classes and
   the scene path are not console-verified by this report.
2. `SceneVm::start()` having no caller is verified by source grep, not by running the port — but the
   code path (`entering` false, `pc == 0` → `BadCursor`) follows from the quoted code and I found no
   other arm that could set `entering` or `pc`.
3. The one-frame entry-timing difference (2.3 no. 3) is an inference from `$2728`'s semantics
   (`halt` + VBlank-flag wait + `inc $FF97`) and the port's loop order; the capture must settle it.
4. I did not trace the exact setter of `$FFE7` bit 3 beyond the `$2DF7` arm and the `$818D` clear —
   the `$2D7D` head's `and $F7` skip test is quoted, and the port's model matches, but a second
   (unfound) setter would change anim 46's gate; the ROM scan for `E0 E7`/`21 E7 FF` found only the
   two whole-byte clears.
5. `$72D7`'s internals (the removal/item-drop routine) were read only far enough to identify it as
   the port's `remove_object`; the port's own tests pin its outward behaviour.
6. The anim-88/46 counter cadences ("16 idle + 48 steps", "28-frame wait then fall") are readings of
   the branch structure, not measurements.
