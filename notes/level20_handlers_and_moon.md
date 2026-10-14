# Level 20 (index 19) unported handler classes + sMoonPhysics ($A287) specification

Read-only investigation, 2026-10-14. No file in `/home/pmill/sml2-rs` or `/home/pmill/sml2decomp`
was modified. Every address below was produced by disassembling `baserom.gb` byte-by-byte with a
scratch SM83 disassembler (`/home/pmill/.hermes/cache/scratch/gbdis.py`), cross-checked against the
text of `main.asm`/`home.asm` and the labels in `sml2.sym`. Address conventions used below:

* **`$5xxxx` / "file offset"** — the form `objects.json` and the port's `run_handler` use: the ROM
  file offset in hex (`0x586EE`). This is `bank*0x4000 + (bankaddr - 0x4000)`.
* **`bank16 $46EE`** — the same byte as a bank-16 **bank-local** address (what the handler tables
  actually store, and what `sml2.sym` prints as `16:46ee`).
* Bank-0 sites (`home.asm`) print as `$0FF7` etc. in code text and `00:0FF7` in listings; bank-1
  sites (`main.asm` bank01 section) as `01:5354`.

---

## PART 1 — the level-20 handler classes the port does not implement

### 1.1 The reach set (join: spawn stream × anim table)

`extracted/levels/level20.manifest.json`: `level_id 19`, `sprite_map_index 1`, 30 spawn records.
Because `sSpriteMapIndex != 0` for `sCurLevel > $0A` ($58036-$5803A: `ld a,[$AF06] / and a /
jp nz,$5C04`), level 20 dispatches through **table B** at file `$59C07` (bank16 `$5C07`; the
dispatcher is `$59C04` = `ldh a,[$FFD5] / rst $28`, bytes `F0 D5 EF`). Table B's entries are dw
bank-16 bank-local addresses, so entry *n* lives at file offset `$59C07 + 2n`.

| anim | records | sprite | handler (mode B) | ported? | notes |
|---|---|---|---|---|---|
| 17 | 4 | 25 | `$58712` (bank16 $4712) | yes | hopper |
| 20 | 1 | 28 | `$58755` (bank16 $4755) | yes | itempose20 |
| 23 | 1 | 82 | `$587D7` (bank16 $47D7) | yes | pose87 |
| **27** | **2** | **25** | **`$586EE` (bank16 $46EE)** | **NO** | type-setter stub |
| **92** | **5** | **122** | **`$5B1C8` (bank16 $71C8)** | **NO** | 4-state bobber/grower |
| **94** | **14** | **126** | **`$5B241` (bank16 $7241)** | **NO** | 3-state vertical bobber |
| **95** | **3** | **126** | **`$5B28C` (bank16 $728C)** | **NO** | 3-state horizontal mover |

The port's `ObjectSim::run_handler` matches `0x58201, 0x582A9, 0x582D1, 0x5834F, 0x584E0, 0x5862E,
0x586E1, 0x586F5, 0x58712, 0x58755, 0x58758, 0x587BD, 0x587D7, 0x58100`; **the four bold handlers
fall into `note_unported(h)` and their bodies do not run.**

Reach-set caveat, stated honestly: this is a join on each record's **initial** anim. Runtime anim
transitions (`$FFD5` writes), block-spawned items and object-spawned objects can reach handler
addresses the initial stream does not name, so the set above is a *lower bound*. Two adjacent
handlers are reachable in the same region but **not** by level 20: `$586EB` (anim 30) and `$586E6`
(anim 31), listed for context in §1.2.

Mode A maps anims 92/94/95 to **different** code (`$59B1F`, `$59CE7`, `$59E06`) and anim 27-29 to
the same `$586EE`; anim 31 differs between tables (A → `$58848`, B → `$586E6`). Only the mode-B
bodies are needed for level 20.

Consequence of the missing bodies today: a slot whose handler is unported never writes `$FFE2`, so
`B_TYPE` stays 0 and the `$2EDD` gate (`ty & $80` set → require `ty & $7F != 0`; else require
`ty & $63 != 0`) skips the slot entirely — the port currently makes **22 of level 20's 30 objects
non-contactable ghosts** (5 + 14 + 3), where the ROM has 2 harmless-but-touchable objects
(anim 27's 2 records) and 18 deadly ones (anim 92/94/95).

### 1.2 Class A — `$586EE` (bank16 `$46EE`), anims 27/28/29 — 2 level-20 records

*Selected by:* mode-B entries 27, 28, 29 (and mode-A entries 27, 28, 29). Sprites 25/26/27
(`anim_to_sprite_b`). Level 20 uses anim 27 → sprite 25.
*Body (bytes `CD 84 76 F0 D5 C6 66 E0 E2 C9`) — the anim-30 stub first, then the shared body:*

```
$586EB (bank16 $46EB)  CD 84 76   call $7684        ; anim 30 entry — NOT reached by level 20
$586EE (bank16 $46EE)  F0 D5      ldh a,[$FFD5]     ; $FFD5 = the staged anim byte
                       C6 66      add $66
                       E0 E2      ldh [$FFE2],a     ; type = anim + $66  ($8D/$8E/$8F)
                       C9         ret
```

`$7684` (bank16, = port's file-offset name `$5B684`) is
`ldh a,[$FFF0] / and $03 / ret nz / jr $7690`, and `$7690` is
`ldh a,[$FFDE] / xor $10 / ldh [$FFDE],a / ret` — i.e. **toggle `B_FLASH` bit 4 every 4th frame**.
The port has only the 2-frame variant (`$768B`, `flash_toggle`), so the anim-30 path would need a
4-frame variant; irrelevant for level 20.

*Type sets / states:* none declared by the extractor and none in the code — the type is computed
(`anim + $66`), so `anim_profiles_*[27..29].type_sets` is empty and there is no `rst $28` sub-state
machine (the extractor's `states` scan is empty too, which is correct here).

*What it does at runtime:* nothing except set the type. Type `$8D`/`$8E`/`$8F`:
* `$2EDD` gate lets it through (bit 7 set, `ty & $7F != 0`), so the `$2EF5` box test runs;
* `$2DC6`'s tail sees `ty & $80` → `and $7F / ld [sPendingItemId],a` → id `$0D`, which **no arm of
  `$3451` handles** ($3451's arms are 1,2,3,4,5,7,8,6,9; everything else falls to `$3539`, clearing
  the pending id) → no effect;
* the `$8611` touched cluster: `cp $86 / ret nc` — `$8D`+ does nothing;
* it cannot be stomped or killed (Mario bounces off nothing; the object is only "touched").

So: an inert, harmless, non-solid type-`$8D` marker. Behaviour is DOMINATED by the type byte — a
port that omits the type write changes nothing visible on contact except that the box test stops
running for that slot.

*Calls / falls into:* nothing (it returns; the anim-30 caller's `call $7684` is the only callee in
the region). It does not re-read the dispatch byte.

*LOC estimate:* **3 lines** (`st.b[B_TYPE] = st.b[B_DEF].wrapping_add(0x66);`) — no new sim state.
*Template:* the ported **`handler_marker85` (`$586E1`)** — literally the same shape with a computed
immediate.

### 1.3 Class B — `$5B1C8` (bank16 `$71C8`), anim 92 only — 5 level-20 records

*Selected by:* mode-B entry 92 (sprite 122). Mode A entry 92 is `$59B1F`, a different body.
*Body:* `$71C8`-`$7240` (bank16). Head + inline table (bytes `F0 DF EF D3 71 E4 71 13 72 29 72`):

```
$71C8 (bank16): F0 DF      ldh a,[$FFDF]      ; sub-state byte
                EF         rst $28            ; inline dw table at $71CB: $71D3, $71E4, $7213, $7229
```

Quoted branch structure (state bodies, verbatim):

```
$71D3:  3E 4C        ld a,$4C
        E0 E2        ldh [$FFE2],a      ; type = $4C (harmful: $4C & $63 = $40, $4C & $23 = 0)
        3E 01        ld a,$01
        E0 DF        ldh [$FFDF],a      ; state = 1
        3E 03        ld a,$03
        E0 E8        ldh [$FFE8],a      ; +24 (step counter)     = 3
        E0 E9        ldh [$FFE9],a      ; +25 (reload value)     = 3
        E0 EA        ldh [$FFEA],a      ; +26 (iteration count)  = 3
        C9           ret
$71E4:  06 01        ld b,$01
        F0 E0        ldh a,[$FFE0]      ; +16 = the up/down phase
        A7           and a
        20 05        jr nz,$71F0
        CD 2D 3D     call $3D2D         ; Y -= b   (up)
        18 03        jr $71F3
$71F0:  CD 3D 3D     call $3D3D         ; Y += b   (down)
$71F3:  F0 E8        ldh a,[$FFE8]
        90           sub b              ; counter -= 1
        E0 E8        ldh [$FFE8],a
        C0           ret nz             ; still stepping
        F0 EA        ldh a,[$FFEA]      ; segment exhausted
        3D           dec a
        E0 EA        ldh [$FFEA],a
        28 0B        jr z,$720B
        F0 E9        ldh a,[$FFE9]
        E0 E8        ldh [$FFE8],a      ; reload the counter
        F0 E0        ldh a,[$FFE0]
        EE 01        xor $01
        E0 E0        ldh [$FFE0],a      ; flip the direction
        C9           ret
$720B:  AF           xor a
        E0 E2        ldh [$FFE2],a      ; type = 0  (harmless from here on)
        3E 02        ld a,$02
        E0 DF        ldh [$FFDF],a      ; state = 2
        C9           ret
$7213:  F0 F0        ldh a,[$FFF0]      ; +$FFF0 frame counter, low byte
        E6 03        and $03
        C0           ret nz             ; every 4th frame only
        F0 DB        ldh a,[$FFDB]      ; +11 = the slot's sprite id (122 initially)
        3C           inc a
        E0 DB        ldh [$FFDB],a
        FE 7C        cp $7C             ; 124
        C0           ret nz
        3E 20        ld a,$20
        E0 EC        ldh [$FFEC],a      ; +28 = 32-frame wait
        3E 03        ld a,$03
        E0 DF        ldh [$FFDF],a      ; state = 3
        C9           ret
$7229:  F0 EC        ldh a,[$FFEC]
        A7           and a
        28 04        jr z,$7232
        3D           dec a
        E0 EC        ldh [$FFEC],a
        C9           ret
$7232:  F0 F0        ldh a,[$FFF0]
        E6 03        and $03
        C0           ret nz
        F0 DB        ldh a,[$FFDB]
        3D           dec a
        E0 DB        ldh [$FFDB],a
        FE 7A        cp $7A             ; 122
        C0           ret nz
        18 92        jr $71D3           ; back to state 0 (re-arms type $4C, state 1, counters 3/3/3)
```

Branch summary: state 0 = type `$4C` + arm 3 up/down segments of 3 px (counter `$FFE8`, reload
`$FFE9`, segment count `$FFEA`, direction `$FFE0`), **no movement on state 0's own frame**; state 1
= one 1-px step per call (up while `$FFE0 == 0`, down otherwise; `b = 1`), on exhaustion reload and
flip direction, and on the 3rd exhaustion drop the type to 0 and go to state 2; state 2 = every 4th
frame sprite id +1 until it reaches `$7C`, then `$FFEC = $20`, state 3; state 3 = wait 32 frames,
then every 4th frame sprite id −1 until `$7A`, then jump back to state 0. Nothing re-reads the
dispatch byte (state changes go through `$FFDF`, which the dispatcher reads at the top of the *next*
call).

*Type sets / sub-states:* none in the extractor's `type_sets` (the `$FFE2` immediates are in the
state bodies, not the handler head) — **the extracted `type_sets` field under-reports for this
class**. Sub-states: the four addresses above.
*Calls:* `$3D2D` / `$3D3D` (bank 0 `ld hl,$FFD4 / Y ∓= b`) → the port's `mv_y_sub` / `mv_y_add`.
Nothing else.
*Registers the port must model:* `$FFF0` low byte (already `ObjectSim::fff0`, used by
`sprite_bob`/`flash_toggle`), `+$FFDB` (already `B_SPRITE` — the handler's write survives to the
`$81CF` hitbox refresh and the draw in the port's pipeline order, same as the ROM), `B_PHASE`
(`$FFE0`), `B_T24` (`$FFE8`), `B_T25` (`$FFE9`), `B_ANIMFRAME` (`$FFEA`), `B_T28` (`$FFEC`). **No
new sim state.**
*LOC estimate:* **~40 lines** in the objects.rs idiom (4-state match + the counter dance + the
sprite-id animation), plus 2-3 unit tests. Danger to flag: `B_ANIMFRAME` (`$FFEA`) is the sprite-bob
cycle index in `spin_step`/`sprite_bob`; in this class it is an iteration counter — do not reuse the
field's other meaning.
*Template:* closest ported body is **`handler_hopper` (`$58712`)** for the "state machine + per-frame
move + counter" shape; the sprite-id mutation has no ported template (the closest is `sprite_bob`,
which adds a wobble to `B_SPRITE` from `$FFF0`).

### 1.4 Class C — `$5B241` (bank16 `$7241`), anim 94 only — 14 level-20 records

*Selected by:* mode-B entry 94 (sprite 126). Mode A entry 94 is `$59CE7`.
*Body:* `$7241`-`$728B`, plus the shared state-0 body at `$724A` (shared with anim 95) and the
shared tail at `$7266`.

```
$7241 (bank16): F0 DF   ldh a,[$FFDF]
                EF      rst $28         ; inline table at $7244: $724A, $7253, $7277
$724A:  3E 48   ld a,$48               ; *** SHARED with anim 95's state 0 ***
        E0 E2   ldh [$FFE2],a           ; type = $48 (harmful: $48 & $63 = $40, $48 & $23 = 0)
        3E 01   ld a,$01
        E0 DF   ldh [$FFDF],a           ; state = 1
        C9      ret
$7253:  F0 E8   ldh a,[$FFE8]           ; +24 selects the step profile
        A7      and a
        20 08   jr nz,$7260
        CD 73 74 call $7473             ; c = $12, profile table $74A1
        CD 2D 3D call $3D2D             ; Y -= b
        18 06   jr $7266
$7260:  CD 4D 74 call $744D             ; c = $12, profile table $7488
        CD 2D 3D call $3D2D             ; Y -= b
$7266:  F0 E0   ldh a,[$FFE0]           ; *** shared tail (also anim 95) ***
        B9      cp c                    ; compare against the sentinel the picker left in c
        C0      ret nz
        AF      xor a
        E0 E0   ldh [$FFE0],a           ; restart the profile counter
        F0 E8   ldh a,[$FFE8]
        EE 01   xor $01                 ; toggle the profile selector
        E0 E8   ldh [$FFE8],a
        C8      ret z                   ; (was 1 → now 0) → done
        C3 33 5D jp $5D33               ; (was 0 → now 1) → flip sub-state
$7277:  F0 E8   ldh a,[$FFE8]           ; state 2 body: same shape, the other axis sign
        A7      and a
        20 08   jr nz,$7284
        CD 73 74 call $7473
        CD 3D 3D call $3D3D             ; Y += b
        18 E2   jr $7266
$7284:  CD 4D 74 call $744D
        CD 3D 3D call $3D3D             ; Y += b
        18 DA   jr $7266
```

Helper `$744D` / `$7473` (bank16) — `ld c,$12` (sentinel 18) then `ld hl,$7488` / `ld hl,$74A1`,
both falling into:

```
$7456 (bank16): F0 E0   ldh a,[$FFE0]      ; profile index
                5F      ld e,a
                16 00   ld d,$00
                19      add hl,de
                46      ld b,[hl]          ; b = profile[$FFE0]  → the px step for this call
                21 E0 FF ld hl,$FFE0
                7E      ld a,[hl]
                B9      cp c               ; counter reached the sentinel?
                C8      ret z              ; yes → no move, b unused
                34      inc [hl]
                C9      ret
```

Profile tables (verbatim ROM bytes):
`$7488` = `00 00 01 01 01 02 01 02 02 01 02 03 02 03 03 03 02 03 03 02 03 03 03 02 03 03`
(indexed 0..18), `$74A1` = `03 02 03 03 03 02 03 02 01 02 02 01 02 01 01 01 00 00 00`.
And `$5D33 (bank16)` = `F0 DF / EE 03 / E0 DF / C9` → `$FFDF ^= $03`, which maps state 1 ↔ 2
(1^3 = 2, 2^3 = 1), i.e. **flip the movement direction**.

Branch summary: state 0 sets type `$48` and state 1 (no movement in its own frame); state 1 walks
19 profile entries of Y− steps, state 2 does the same with Y+ steps; each profile cycle toggles
`$FFE8` and every *other* cycle flips state via `$5D33` (so the object floats up, then down, etc.).
The profile selector (`$FFE8`) picks *which* of the two tables supplies the per-frame step. Neither
body re-reads the dispatch byte; the state write lands for the next call.

*Type sets / sub-states:* extractor `type_sets` empty (type is set in the shared state-0 body) —
again under-reported. Sub-states: `$724A`, `$7253`, `$7277`.
*Calls / falls into:* `$3D2D`/`$3D3D` → port `mv_y_sub`/`mv_y_add`; `$744D`/`$7473` → **new shared
helper** (needed by this class and by anim 95); `$5D33` → **new shared helper** (one-liner). The
other `$74xx` picker entries (`$7451` c=$15, `$7465` c=$08, `$746C` c=$0C, `$747A` c=$15, `$7481`
c=$0F) are used by handlers in other levels — model the general picker but only these two entries
are needed for level 20.
*Registers the port must model:* `c` (the sentinel) **crosses the helper call boundary into the
shared tail** — in Rust the picker must *return* the sentinel so the tail can do `cp c`; do not fold
the tail into one of the two callers, since both callers and both classes share it. `b` is the px
step. Both are locals. No new sim state.
*LOC estimate:* **~25 lines** for this handler + **~12 lines** for the `$7473/$744D` picker + 2
tables + `$5D33` (2 lines). The state-0 body is **shared with class D** — implement it once.
*Template:* closest ported template is **`handler_fallwalker` (`$582D1`)** — a state machine whose
states are "move along an axis, then change state" — combined with `fall_step`'s table-driven step
(`$58293` uses a curve table indexed by a phase byte, which is the closest existing "table-driven
per-frame move" in the port).

### 1.5 Class D — `$5B28C` (bank16 `$728C`), anim 95 only — 3 level-20 records

*Selected by:* mode-B entry 95 (sprite 126 — same as anim 94). Mode A entry 95 is `$59E06`.
*Body:* `$728C`-`$72BD`, sharing `$724A` (state 0) and `$7266` (tail) with class C.

```
$728C (bank16): F0 DF   ldh a,[$FFDF]
                EF      rst $28        ; inline table at $728F: $724A, $7295, $72AA
$7295:  F0 E8   ldh a,[$FFE8]
        A7      and a
        20 08   jr nz,$72A2
        CD 73 74 call $7473
        CD 22 3D call $3D22           ; X -= b   (LEFT)
        18 C4   jr $7266              ; the SAME tail as anim 94
$72A2:  CD 4D 74 call $744D
        CD 22 3D call $3D22
        18 BC   jr $7266
$72AA:  F0 E8   ldh a,[$FFE8]
        A7      and a
        20 08   jr nz,$72B7
        CD 73 74 call $7473
        CD 32 3D call $3D32           ; X += b   (RIGHT)
        18 AF   jr $7266
$72B7:  CD 4D 74 call $744D
        CD 32 3D call $3D32
        18 A7   jr $7266
```

Branch summary: identical machinery to class C with the X axis: state 1 moves left, state 2 moves
right, the same profile tables/sentinels, the same `$FFE8` toggle, the same `$5D33` sub-state flip
(so it shuttles horizontally). Both bodies jump into `$7266`, which does `cp c` against the sentinel
returned by the picker.

*Type sets / sub-states:* `type_sets` empty (type `$48` set in the shared `$724A`); sub-states
`$724A`, `$7295`, `$72AA`.
*Calls / falls into:* `$3D22`/`$3D32` → port `mv_x_sub`/`mv_x_add`; `$7473`/`$744D` and `$5D33` as
in class C; the shared state-0 body at `$724A`.
*Registers:* same note as class C (`c` must cross the call boundary).
*LOC estimate:* **~12 lines** once class C's shared helpers exist (state 1/2 bodies differ from C by
two mnemonics).
*Template:* same as class C — **`handler_fallwalker` (`$582D1`)** for the state shape; the ported
`mv_x_add`/`mv_x_sub` are exact transliterations of `$3D32`/`$3D22`.

### 1.6 Per-class table (summary)

| handler | anims | 1-line behaviour | LOC est. | template |
|---|---|---|---|---|
| `$586EE` | B:27,28,29 (A:27,28,29) | sets `type = anim + $66` (`$8D`/`$8E`/`$8F`) and returns — inert, harmless, non-solid | 3 | `handler_marker85` (`$586E1`) |
| `$5B1C8` | B:92 | 4-state: type `$4C` + 3×(3 px up / 3 px down) via `$3D2D`/`$3D3D`, then type 0, then sprite id 122→124 (every 4th frame), 32-frame hold, 124→122, repeat | ~40 | `handler_hopper` (`$58712`) + `sprite_bob` for the id animation |
| `$5B241` | B:94 | 3-state: type `$48`, then Y− (state 1) / Y+ (state 2) table-driven steps (`$7488`/`$74A1` profiles, 19 entries, sentinel 18), flipping direction every other profile cycle via `$5D33` | ~25 (+12 shared helpers) | `handler_fallwalker` (`$582D1`) + `fall_step`'s table move |
| `$5B28C` | B:95 | same 3-state machine as `$5B241` but on X (`$3D22` / `$3D32`), sharing state 0 and the tail | ~12 (given C's helpers) | `handler_fallwalker` (`$582D1`) |

Nothing in any of the four needs new sim state; all state lives in the staged slot bytes the port
already names (`B_TYPE`, `B_FSTATE`, `B_PHASE`, `B_T24`, `B_T25`, `B_ANIMFRAME`, `B_T28`,
`B_SPRITE`).

---

## PART 2 — `sMoonPhysics` (`$A287`): every reader and writer, and the moon arm

### 2.1 Definition, seeding, and bit semantics

`ram.asm:150 DEF sMoonPhysics EQU $A287`. Writers found by scanning `baserom.gb` for the literal
`FA 87 A2` (read) / `EA 87 A2` (write): **26 read sites, 7 write sites**. One writer is the seed and
six are bit ops — the byte is **never moved as a whole after the seed**.

**Seed — `$04DD`-`$04F0` (`home.asm`, in the level-init state handler; `LevelPropertiesSpace` label
at `home.asm:4099`):**

```
$04DD: 21 91 1F   ld hl,$1F91          ; LevelPropertiesSpace
$04E0: FA 69 A2   ld a,[$A269]         ; sCurLevel
$04E3: 5F         ld e,a
$04E4: 16 00      ld d,$0
$04E6: 19         add hl,de
$04E7: 7E         ld a,[hl]            ; "check level space physics"
$04E8: EA 87 A2   ld [$A287],a         ; *** the only whole-byte write ***
$04EB: F0 9B      ldh a,[$FF9B]
$04ED: 3C         inc a
$04EE: E0 9B      ldh [$FF9B],a        ; advance the game state
$04F0: C9         ret
```

Table bytes at `$1F51` (`LevelPropertiesUNK`), `$1F71` (`LevelPropertiesScroll`), `$1F91`
(`LevelPropertiesSpace`), read out of `baserom.gb`:

```
UNK    $1F51: 00 00 08 08 08 08 08 00 00 00 08 00 00 00 00 08 00 00 08 08 00 00 00 00 08 00 00 00 08 00 00 08
Scroll $1F71: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 01 00 00 00 00 01 00
Space  $1F91: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 08 01 00 00 00 00 00 00 00 00 08 00 00 00
```

So the nonzero `space_physics` entries are **index 18 → `$08` (bit 3), index 19 → `$01` (bit 0),
index 28 → `$08` (bit 3)** — matching `properties.space_physics` in the manifests (1 for level 20).
**The value is the raw byte, and it is a bitfield, not a flag** — the port must seed the raw byte and
test bits, exactly as it already does for `auto_scroll` (and unlike its bool for
`map_scroll_limit`).

Bit assignments, from the arms that consume them:

| bit | set by | means |
|---|---|---|
| 0 (`$01`) | seed (level index 19 only) | level's "hold-A float" (the `$106F` arm: vvel += 4/frame up to `$48`) |
| 1 (`$02`) | runtime only, `$1D14`-`$1D19` | currently inside a moon zone (collision tile `$52`-`$57`) |
| 2 (`$04`) | item id 8 (`$34E2`-`$34E7`) | the moon item is held (one-shot shield / flight sprite) |
| 3 (`$08`) | seed (level indices 18, 28) | the timed float (`$1029` arm: vvel += 2/frame up to `$4A`, `sMoonFloatTimer` budget 50) |

The test forms are **inconsistent on purpose** and must be transcribed literally:
`and a` (any nonzero: `$0FF7`, `$10A1`, `$14D5`, `$1541`, `$156E`, `$1749`, `$179E`), `and $0F`
(low nibble: `$1282`, `$15E2`, `$1647`), and `bit 1` / `bit 2` (`$0626`, `$14FA`, `$16D2`, `$1835`,
`$1D28`, `$1D4B`, `$2E7C`).

### 2.2 Frame order — before or after the movement integration

The in-level frame (`home.asm`, the `$FF9B = 4` body) is:

```
call $2C61                       ; frame selection
if sMarioDead == $FF → $0640     ; dead comms
latch Y/X into sMarioY/sMarioX
if sScreenTransition != 0 → $060F ; transition: SKIP the player update
ld [sInLevelFlag],0
call $0F71                       ; *** the player update *** (falls through)
$060F (shared tail): $0879, $0B2A, $2062, $2D0A, $2C82,
    if sPipeTravelDirection < $10: call $2D7D      ; the object engine
    ld a,[sMoonPhysics] / bit 1,a / call nz,$3753  ; *** $0626: moon FX, every 128th frame ***
    $2A58, sSavedOAMIndex, $0652, $2CFF, $273E
```

`$0F71` itself (disassembled, addresses exact):

```
$0F71: call $307D      ; crouch-look pan
$0F74: call $13FE      ; per-frame resets
$0F77: if sPipeTravelDirection >= $10 → $0FBB (pipe travel)
$0F7E: call $0FF2      ; <<< MOON JUMP EXTENSION (bits 0/1/3 + the no-bit arm) — FIRST player call
$0F81: call $1BFF      ; <<< probe + moon-zone bit 1 SET/CLEAR + hazard + exit bell
$0F84: call $129F      ; landing check
$0F87: call $1413      ; wall push-out
$0F8A: call $14A6      ; ducking
$0F8D: call $126F      ; <<< spin jump (gated by moon & $0F)
$0F90: call $160A      ; jump (sfx gated by moon & $0F)
$0F93: call $1090      ; <<< bunny flutter (disabled by any nonzero moon)
$0F96: call $14FA      ; <<< gravity + vertical integration (bit 1, vvel $1541, V table $156E)
$0F99: call $1701      ; <<< horizontal accel (hvel caps)
$0F9C: call $16CC      ; <<< friction (bit 1)
$0F9F: call $181C      ; <<< horizontal integration (H table, bit 1 + on-ground)
$0FA2: call $0FBF      ; screen-edge wrap
$0FA5: call $14BF      ; <<< walk-anim timer (any nonzero)
$0FA8: call $1189      ; pipe arming
$0FAB: call $319F
$0FAE: call $3451      ; <<< item consumer (id 8 sets bit 2)
$0FB1: call $32C1 / $0FB4: $331C / $0FB7: $324F
$0FBA: ret
```

Answer to the ordering question, per site:
* `$0FF2` reads the byte **before** `$1BFF` has refreshed bit 1 → it sees the *previous* frame's
  zone state.
* `$1BFF`'s probe writes bit 1 **before** `$14FA`/`$1701`/`$16CC`/`$181C` read it → the gravity and
  the horizontal integration consume the **fresh** value in the same frame.
* `$1090` and `$126F` run **after** `$1BFF` (fresh bit 1) and after the jump (`$160A`).
* The `$3451` item consumer (bit 2 set) runs at the **end** of the same player update, so a moon
  pickup affects the *next* frame's `$2E6F`/`$54FB` reads (and `$2E7C`, the harm consume, runs
  inside the object pass, which is after `$0F71`).
* `$0626` (bit 1 → `$3753`, the moon FX) is in the **shared frame tail, after the object engine**.
* The two bank-1 pose sites (`$5354`, `$53CF`) and the draw site (`$54FB`) are in the frame's
  sprite pipeline (after the player update).

### 2.3 The read sites, with what they change

**(1) `$0626`-`$062B` — frame tail, moon FX (ported? NO).**
```
$0626: FA 87 A2  ld a,[$A287]
$0629: CB 4F     bit 1,a
$062B: C4 53 37  call nz,$3753
```
`$3753`: `ldh a,[$FF97] / and $7F / ret nz` → runs only on frames where the global counter's low 7
bits are 0 (**once every 128 frames**); if `sMoonFxState != 0` it returns; else it sets
`sMoonFxState = 2`, `sMoonFxTimer = 0` and latches Mario's position into `sMoonFxY/YU/X/XU`
(`$A2B8`-`$A2BB`). Effect: a periodic moon-footstep/dust effect sprite. The effect is *drawn* by bank
1 at `$6485` (`ld a,[sMoonFxState] / and a / ret z / cp $01 / jr z,$64F9 / ...` rising 2 px per even
frame, etc.). The port models neither the FX state nor the draw. Not reachable in level 20?
`bit 1` needs a moon-zone tile — see §2.5.

**(2) `$0FF2` — the moon jump extension (ported? NO — no `0FF2` exists in `player_sim.rs`).**
```
$0FF2: FA 4F A2  ld a,[sGoalBell] / and a / ret nz        ; bell ringing → out
$0FF7: FA 87 A2  ld a,[$A287]
$0FFA: A7        and a
$0FFB: C8        ret z                                     ; *** plain level: no-op ***
$0FFC: CB 47     bit 0,a
$0FFE: 20 6F     jr nz,$106F                               ; level 20's arm
$1000: CB 4F     bit 1,a
$1002: 20 51     jr nz,$1055                               ; in a moon zone
$1004: CB 5F     bit 3,a
$1006: 20 21     jr nz,$1029                               ; levels 18/28's timed float
$1008: FA 14 A2  ld a,[sMarioOnGround] / and a / ret nz    ; bit-2-only arm
$100D: F0 80     ldh a,[hKeysHeld] / bit 0,a / ret z       ; A must be held
$1012: F0 81     ldh a,[hKeysPressed] / bit 0,a / jr z,$101D
$1018: 3E 12     ld a,18 / ld [sSfxRequest0],a             ; sfx 18 on the A press
$101D: FA 01 A2  ld a,[sVVelocityIndex] / cp $4A / ret nc / add 2 / ld [sVVelocityIndex],a
$106F: [bit 0 arm] sMarioOnGround → ret nz; A held → ret z; A pressed → sfx 18;
       vvel: cp $48 / ret nc / add 4 / store                ; *** +4/frame up to $48, level 20 ***
$1055: [bit 1 arm] hKeysPressed bit 0 → ret z; vvel = 52;
       ld a,[sMoonJumpPoseTimer] / and a / ret nz; ld a,8 / ld [sMoonJumpPoseTimer],a; sfx 3
$1029: [bit 3 arm] sMarioOnGround → $104A (sMoonFloatTimer = 50, ret);
       A not held → xor a / sMoonFloatTimer = 0, ret;
       sMoonFloatTimer == 0 → ret; dec it; vvel: cp $4A / ret nc / add 2 / store
```
So for **level 20 (bit 0)** the live effect is: airborne + A held → vvel index **+4 per frame, capped
`$48`**, i.e. against the same frame's `−1` gravity below it, a net slow rise/float while A is held;
an A *press* mid-air requests sfx 18. This is a pure physics arm with no timer, and it runs *before*
the gravity, so the cap is reached in one frame from a jump apex (75 → 72 immediately).

**(3) `$10A1` — bunny flutter disable (ported, `player_sim.rs:~1278`).**
```
$1090: ld a,[sGoalBell] / and a / ret nz ; sCameraPanTimer → ret nz
$1099: ld a,[sCurPowerup] / cp $02 / jr nz,$10EB     ; powerup 2 = the bunny/flutter
$10A1: FA 87 A2  ld a,[$A287]
$10A4: A7        and a
$10A5: 20 44     jr nz,$10EB                          ; *** ANY nonzero → no flutter ***
$10A7: ld a,[sVVelocityIndex] / cp $48 / jr nc,$10EB ; only while falling slowly
...
```
Effect: a whole routine (flutter charge/armed machinery, `sFlutterCharge`, `sFlutterArmed`, sfx)
is skipped when `sMoonPhysics != 0`.

**(4) `$1282` — spin-jump disable (ported: `keys.down && powerup & 1 && !on_ground && moon & 0x0F == 0`).**
```
$126F: held bit 7 (Down); sCurPowerup bit 0 set; !sMarioOnGround
$1282: FA 87 A2  ld a,[$A287]
$1285: E6 0F     and $0F
$1287: 20 11     jr nz,$129A        ; *** low nibble nonzero → clear sSpinJump, ret ***
$1289: ld a,[sSpinJump] ... 255 / sSpinJump = 255; sfx 14 on the engaging edge
```

**(5) `$14D5` (= the `$14BF` walk-animation timer) — moon forces b = 1.**
```
$14BF: call $14D5 ; then the scroll deltas
$14D5: FA 87 A2  ld a,[$A287]
$14D8: A7        and a
$14D9: 20 0E     jr nz,$14E9        ; *** moon → b = 1 ***
$14DB: ldh a,[hKeysHeld] / and $02 / srl a / inc a / ld b,a   ; normal: b = 1 + (B held)
$14E3: (sGoalBell test)
$14E9: ld b,1
$14EB: ld a,[sMarioWalkAnimTimer] / add b / ld [..],a / cp $0C / ret c / xor a / ld [..],a
```
Effect: in a moon level the walk-anim timer advances 1/frame even with B held (no 2× cadence).

**(6) `$14FA` head — moon bit 1 short-circuits the flutter timer.**
```
$14FA: FA 87 A2  ld a,[$A287]
$14FD: CB 4F     bit 1,a
$14FF: 20 0B     jr nz,$150C        ; *** in a moon zone → skip the sFlutterTimer arm ***
$1501: ld a,[sFlutterTimer] / and a / jr z,$150C / dec a / store / ret
```
Note the polarity: the flutter-timer decrement+early-return happens only when bit 1 is **clear**.

**(7) `$1541` — the vvel-$30 "hold at apex" (half-ported: only the non-moon arm exists).**
```
$1533: ld a,[sVVelocityIndex] / and a / jr z,$156B
$1539: dec a / ld [sVVelocityIndex],a    ; gravity's own decrement
$153D: cp $30                            ; (vvel after the decrement)
$153F: jr nz,$156B
$1541: FA 87 A2  ld a,[$A287]
$1544: A7        and a
$1545: 20 07     jr nz,$154E             ; *** moon → the $154E arm ***
$1547: 3E 18     ld a,24 / ld [sVVelocityIndex],a / jr $156B    ; normal: snap to 24 once
$154E: ld a,[sVVelocityIndex] / dec a / ld [..],a               ; *extra* decrement
$1555: cp $19 / jr nc,$156B
$1559: ld a,24 / ld [sVVelocityIndex],a
$155D: ldh a,[$FF97] / and $01 / jr nz,$156B
$1563: ld a,23 / ld [sVVelocityIndex],a
```
The port models only `if self.vvel == 0x30 && self.moon == 0 { self.vvel = 24 }`; the moon arm
(extra decrement, then the `$19`/24/23 parity dance) is missing.

**(8) `$156E` — the vertical speed table select (ported: `moon != 0 → SPEED_V_WATER`).**
```
$156B: 21 47 19  ld hl,MarioSpeedTableVWater          ; $1947
$156E: FA 87 A2  ld a,[$A287]
$1571: A7        and a
$1572: 20 0C     jr nz,$1580                           ; *** ANY nonzero → the $1947 table ***
$1574: 21 AF 18  ld hl,MarioSpeedTableV                ; $18AF
$1577: ld a,[sSwimming] / and a / jr z,$1580
$157D: 21 FB 18  ld hl,MarioSpeedTableVGoo             ; $18FB
$1580: index by sVVelocityIndex, then `add 16 / add b / adc 0 / sub 16`
```
`home.asm:3466` labels `$1947` `MarioSpeedTableVWater ;$1947 Also used for outer space and hippo
bubble` — the ROM itself documents the reuse. The port's table constant is correct (its tail 6 bytes
`AF EA 20 A2 FA 0D` are past the ROM's table and unused — the array is 82 entries for the 76-byte
table, "extra, not wrong").

**(9) `$15E2` — ceiling bonk: moon low nibble suppresses the sfx (port does not model sfx here).**
```
$15CF: ld a,[sMarioY] / ldh [$FFC0],a ; sMarioYU → $FFC1   ; snap y back
$15D9: xor a / ld [sMoonFloatTimer],a
$15DD: ld a,24 / ld [sVVelocityIndex],a
$15E2: FA 87 A2  ld a,[$A287]
$15E5: E6 0F     and $0F
$15E7: 20 0B     jr nz,$15F4          ; *** low nibble nonzero → no sfx 2 ***
$15E9: ld a,[sSwimming] / and a / jr nz,$15F4
$15EF: ld a,2 / ld [sSfxRequest0],a
$15F4: call $1993 ...
```
Also note this arm **clears `sMoonFloatTimer`** on every ceiling bonk — the float budget resets.

**(10) `$1647` — jump sfx selection.**
```
$1637: ld a,[sGoalBell] / and a / jr nz,$16AE
$163D: ld a,1 / ld [sJumpFlag],a ; ld a,1 / ld [sSfxRequest0],a     ; the jump sfx
$1647: FA 87 A2  ld a,[$A287]
$164A: E6 0F     and $0F
$164C: 28 0B     jr z,$1659                                          ; no space bits
$164E: CB 4F     bit 1,a
$1650: 20 0D     jr nz,$165F                                         ; in a moon zone
$1652: 3E 12     ld a,18 / ld [sSfxRequest0],a                       ; *** space jump sfx 18 ***
$1657: jr $1664
$1659: ld a,[sSwimming] / and a / jr z,$1664
$165F: ld a,3 / ld [sSfxRequest0],a                                  ; swim/moon-zone sfx 3
```
Effect: audio only in level 20 (`& $0F != 0`, bit 1 clear → sfx 18 instead of 1).

**(11) `$16D2` — friction gate (ported: `swim || moon & 2 || on_ground`).**
```
$16CC: ld a,[sSwimming] / and a / jr nz,$16DE
$16D2: FA 87 A2  ld a,[$A287]
$16D5: CB 4F     bit 1,a
$16D7: 20 05     jr nz,$16DE              ; in a moon zone → friction runs
$16D9: ld a,[sMarioOnGround] / and a / ret z
$16DE: (hvel → $80 by ±4)
```

**(12)/(13) `$1749` / `$179E` — hvel caps in `$1701` (ported, functionally equivalent).**
```
$1749: FA 87 A2  ld a,[$A287] ; and a / jr nz,$1755     ; *** moon → skip the B-run test ***
$174F: ldh a,[hKeysHeld] / bit 1,a / jr nz,$1763        ; B held → the fast cap
$1755: ld a,[sHVelocityIndex] / cp $C0 / jr c,$17C4 / ld a,$BF / ld [sHVelocityIndex],a
$1763: ld a,[sHVelocityIndex] / cp $F4 / jr c,$17C4 / ld a,$F3 / ld [sHVelocityIndex],a
```
and the mirrored left arm at `$179E` (`$17AA`: cap `$48`, `$17B8`: floor `$08`).
Effect: with **any** nonzero `sMoonPhysics`, Mario's horizontal speed is capped at walk speed
(`$BF`/`$48`) and B cannot raise it to `$F3`/`$08` — the port's `else if self.moon == 0 && keys.b
{...} else if hvel >= 0xC0 {...}` transcribes this exactly.

**(14) `$1835` — the horizontal speed table select (ported, but see the gap below).**
```
$181C: latch sMarioX/XU
$1826: ld a,[sGoalBell] / and a / jr nz,$1842     ; *** bell → the NORMAL table ***
$182C: 21 FF 1A  ld hl,MarioSpeedTableHWater      ; $1AFF
$182F: ld a,[sSwimming] / and a / jr nz,$1845     ; swimming → keep $1AFF
$1835: FA 87 A2  ld a,[$A287]
$1838: CB 4F     bit 1,a
$183A: 28 06     jr z,$1842                        ; no zone → the normal table
$183C: ld a,[sMarioOnGround] / and a / jr nz,$1845 ; zone + on ground → keep $1AFF
$1842: 21 FF 19  ld hl,MarioSpeedTableH            ; $19FF
$1845: index by hvel + ($FF97 & 7)
```
Port gap to flag: the port's `htable = if self.swim || (self.moon & 2 != 0 && self.on_ground)` omits
the `sGoalBell != 0 → normal table` arm, so during a bell-ringing exit in water/moon-grounded frames
the port would use the water table where the ROM uses the normal one.

**(15)/(16)/(17)/(18)/(19)/(20) `$1CAF`-`$1D4B` — the moon-zone machinery inside `$1BFF` (NOT
ported; the port's `environment_probe` comment says so explicitly).**

```
$1C82: tile & $FC == $4C  (water region) → ...
$1CAA: 3E FF      ld a,$FF / ld [sSwimming],a        ; sSwimming = $FF
$1CAF: FA 87 A2   ld a,[$A287]
$1CB2: CB 8F      res 1,a
$1CB4: EA 87 A2   ld [$A287],a                       ; *** water arm CLEARS bit 1 ***
$1CB7: jp $1D71                                      ; tail
$1CBA: tile & $FE == $4A → the exit-bell arm (clears sLowTimeFlag etc.)
$1CFB: FA 87 A2   ld a,[$A287]                       ; *** the moon arm ***
$1CFE: E6 02      and $02
$1D00: 47         ld b,a                            ; b = OLD bit 1
$1D01: FA 87 A2   ld a,[$A287]
$1D04: CB 8F      res 1,a / ld [$A287],a            ; clear bit 1
$1D09: ld a,[sCollisionTile]
$1D0C: FE 52      cp $52 / jr c,$1D28
$1D10: FE 58      cp $58 / jr nc,$1D28
$1D14: FA 87 A2   ld a,[$A287]                       ; tile in [$52,$58) → entering a moon zone
$1D17: CB CF      set 1,a / ld [$A287],a            ; *** set bit 1 ***
$1D1C: xor a / ld [sFlutterArmed],a                 ; the bunny flutter is disarmed
$1D20: FA 87 A2   ld a,[$A287]
$1D23: CB 97      res 2,a / ld [$A287],a            ; entering a zone consumes the moon item
$1D28: FA 87 A2   ld a,[$A287]
$1D2B: E6 02      and $02 / cp b
$1D2E: 28 41      jr z,$1D71                         ; no CHANGE in bit 1 → done
$1D30: ld a,[sHVelocityIndex] / cp $C0 / jr c,$1D3C / ld a,$BF / ld [$A200],a   ; hvel clamp
$1D3C: ld a,[sHVelocityIndex] / cp $48 / jr nc,$1D48 / ld a,72 / ld [sHVelocityIndex],a
$1D48: call $3735                                    ; moon FX state 1 (ground flavour)
$1D4B: FA 87 A2   ld a,[$A287]
$1D4E: CB 4F      bit 1,a
$1D50: 20 1F      jr nz,$1D71                        ; entered → out
$1D52: ldh a,[hKeysHeld] / bit 6,a / jr nz,$1D64     ; UP held → the lock arm
$1D58: ld a,70 / ld [sVVelocityIndex],a ; ld a,$FF / ld [sMarioInAir],a
$1D64: ld a,70 / ld [sVVelocityIndex],a ; $FF → sMarioInAir ; $FF → sNoJumpCut
```
Effect of *leaving* a zone: vvel = 70 (`$46`) launch + `sMarioInAir = $FF` (+ `sNoJumpCut = $FF`
when UP is held), plus the hvel floor/cap. `$3735` sets `sMoonFxState = 1` and latches Mario's
position (the same FX latches as `$3753`, the *other* flavour).

**(21) `$2E7C` — the moon item's shield, inside the harm path (port returns early without consuming).**
```
$2E6F: ld a,[sStarTimerHigh] / and a / jp nz,$2EAA     ; star defeat
$2E76: ld a,[sObjectType] / bit 6,a / ret z            ; not harmful → out
$2E7C: FA 87 A2  ld a,[$A287]
$2E7F: CB 57     bit 2,a
$2E81: 28 0B     jr z,$2E8E
$2E83: CB 97     res 2,a / ld [$A287],a                ; *** consume the moon item ***
$2E88: 3E 80     ld a,$80 / ld [sInvincibilityTimer],a ; 128 frames of invincibility
$2E8D: C9        ret
$2E8E: ld a,[sInvincibilityTimer] / and a / jp nz,$2F71
$2E95: ld a,15 / ld [sMarioDead],a ; sfx 20 ; small Mario also sfx 15
```
Port gap: `objects.rs:2072` does `if moon_physics & 0x04 != 0 { note_unported(0x2E8E); return }` —
it skips the death but leaves bit 2 set and does not grant the 128-frame invincibility, so the port
would survive *every* harmful contact instead of exactly one.

**(22) `$34E2`-`$34E7` — the item id 8 arm (ported: `self.moon |= 0x04`, `sfx1 = 20`).**
```
$34D9: FA 87 A2  ld a,[$A287] / ... cp $08 ...
$34DD: ld a,20 / ld [$A470],a          ; sSfxRequest1
$34E2: FA 87 A2  ld a,[$A287]
$34E5: CB D7     set 2,a
$34E7: EA 87 A2  ld [$A287],a
$34EA: jr $3539                        ; clear the pending id
```
(Only reached when `sPendingItemId == 8`, which is `type & $7F` of a `$88` object.)

**(23)/(24) bank 1 `$5354` / `$53CF` — the moon jump poses (NOT ported; `frame_index` says "Moon
… not modelled").**
```
01:534C: ld a,[sMarioDirection] / cp $01 / jp nz,$53CF     ; right-facing arm at $534C, left at $53CF
01:5354: FA 87 A2  ld a,[$A287]
01:5357: CB 4F     bit 1,a
01:5359: 28 1C     jr z,$5377                              ; not in a zone → normal walk poses
01:535B: ld a,[sMarioOnGround] / and a / jr nz,$5377        ; on the ground → normal poses
01:5361: 3E 12     ld a,18 / ldh [$FFC6],a                  ; pose 18 (right, in a moon zone, airborne)
01:5365: ld a,[sMoonJumpPoseTimer] / and a / jp z,$5447
01:536B: dec a / ld [sMoonJumpPoseTimer],a
01:5370: 3E 13     ld a,19 / ldh [$FFC6],a / jp $5447        ; pose 19 while the timer > 0
01:53CF: (left arm) same with poses 20 / 21
```
Effect: while airborne inside a moon zone, Mario's `$FFC6` sprite becomes 18/19 (right) or 20/21
(left), with `sMoonJumpPoseTimer` from the `$1055` arm decaying it. The port's `frame_index` would
need these arms *before* the duck/ground/air chain (they sit inside `$534C`, which the port models
at its `self.ducking`/`on_ground` branches) plus the new `sMoonJumpPoseTimer` byte.

**(25) bank 1 `$54FB` — the moon item's drawn sprite (NOT ported).**
```
01:54FB: call $5297                       ; draw [$FFC6] at the staged pos
01:54FE: xor a / ldh [$FFC7],a / ldh [$FFBB],a
01:5503: FA 87 A2  ld a,[$A287]
01:5506: CB 57     bit 2,a
01:5508: 28 14     jr z,$551E              ; no moon item → skip the extra sprite
01:550A: 3E 01 / ldh [$FFBB],a            ; no X-flip for the extra draw ($52D5 skips `set 7`)
01:550D: ldh a,[$FF97] / and $10 / swap a / add 220 / ldh [$FFC6],a   ; sprite 220/221
01:5515: call $5297                       ; draw it at Mario's own screen pos ($FFC4/$FFC5)
01:5518: xor a / ldh [$FFBB],a
01:551E: (the stomp-bounce arm)
```
Effect: bit 2 adds a second sprite (220 with `$FF97` bit 4 clear, 221 with it set) drawn at Mario's
position, unflipped, in addition to the pose. Renderer-level; the port's `render_view` feeds the
player slot from `frame_index`, so this needs an extra-draw path (or, minimally, the frame index
swap) to be visible.

Companion-byte writer lists (scanned the ROM for the `EA xx xx` stores, so these are complete):
* `sMoonFloatTimer` (`$A2B2`): `$103B` (the bit-3 arm's decrement), `$104C` (=50 on the ground),
  `$1051` (=0 when A is not held), `$15DA` (=0 on a ceiling bonk).
* `sMoonJumpPoseTimer` (`$A285`): `$1066` (=8, in the bit-1 arm), `$536D` and `$53E8` (the two pose
  arms' decrements).
* `sFlutterTimer` (`$A24D`): `$10DE` (the flutter routine) and `$1508` (the arm at `$14FA` that
  moon bit 1 skips). **The port has no `sFlutterTimer` field at all** — its `MarioContact::flutter`
  (always 0, see `main.rs:1063`) is a different byte; the `$14FA` arm therefore needs a new field
  before it can be transcribed literally.
* `sMoonFxState`/`sMoonFxTimer` (`$A2B6`/`$A2B7`): `$3737`/`$373B` (the `$3735` ground flavour),
  `$375F`/`$3763` (the `$3753` air flavour), and bank 1 `$64F5`/`$64FD` (the draw clearing them when
  the effect finishes).

### 2.4 What the port's `player_sim` / `camera` need (spec for seeding)

Already correct and to be left alone: `SPEED_V_WATER` = `$1947` selected by `moon != 0` ($156E);
`moon & 0x0F == 0` on the spin jump ($1282); `moon & 2 != 0` in the friction gate ($16D2) and in the
H-table select ($1835); `moon |= 0x04` on item id 8 ($34E2); the hvel caps ($1749/$179E);
`SPEED_H_WATER` = `$1AFF`.

To add, in frame order:
1. **Seed** — both loops (`run_replay` / `run_play`) must set `sim.moon` from
   `manifest.properties.space_physics` (raw byte), alongside `map_scroll_limit`/`auto_scroll`
   (`crates/sml2-runtime/src/main.rs:224` area). For every existing capture this is `0`, so the gate
   must stay byte-identical — verify with the whole sweep.
2. **`$0FF2`** as a new function called FIRST in the player step (before `environment_probe`), with
   the four arms; only `sGoalBell != 0 → return` and `moon == 0 → return` are needed for the gate to
   stay neutral. `bit 0` (level 20) needs nothing but `vvel` and `sfx0`; `bit 3` needs a new
   `moon_float_timer: u8` field; `bit 1` needs `moon_jump_pose_timer: u8`.
3. **`$1BFF` bit 1** — in `environment_probe`, replace the water arm's note with `self.moon &= !2`
   and add the `$1CFB` zone arm (tile `$52..$57` test, `set bit 1`, `moon &= !4`,
   `flutter_armed = 0`, the old-vs-new bit compare, the hvel clamp, the FX latch and the `vvel = 70`
   launch). **Reachability caveat: level 20's layout contains no `$52`-`$57` cell at all** (§2.5), so
   this arm is dead for level 20 and needs a different level (or a synthetic layout) to verify.
4. **`$14FA`'s flutter-timer arm** and the `$154E` vvel-$30 moon arm (see (6)/(7)).
5. **`$2E7C`** — in `harm_or_star`: on `moon & 4`, do `moon &= !4; invincibility = 0x80; return`
   (replacing the bare `note_unported`).
6. **Pose/draw arms** in `frame_index` (`$5354`/`$53CF`: bit 1 + airborne → 18/19 or 20/21 with a
   new `moon_jump_pose_timer`; `$54FB`: bit 2 → the extra 220/221 sprite in the renderer).
7. **`$181C`** — add the `goal_bell != 0 → normal table` guard (fidelity fix; no level-20 effect,
   since level 20 carries no bell and no water).

`camera.rs` needs nothing: no `sMoonPhysics` read exists in the camera/scroll code (all 33 sites are
accounted for above).

### 2.5 Reachability measurements (what can actually be observed where)

* Level 20's decompressed layout (`extracted/levels/level20.decompressed.bin`, 12288 bytes) contains
  **no cell in `$52`-`$57`** (the moon-zone tiles), no `$4A`/`$4B` (exit), no `$4C`-`$4F` (water),
  but 196 cells of `$51` (the `$50/$51` hazard pair → the hazard probe fires in level 20). Present
  ids: `$60` ×10321, `$70` ×384, `$23` ×255, `$22` ×254, `$5F` ×239, `$51` ×196, `$5C` ×188,
  `$71` ×161, `$48` ×99, `$6F` ×78, `$67` ×4 (+6 more < `$40`). So in level 20:
  * bit 0 (the float) and the V-table swap are the **live** moon effects;
  * bit 1's zone machinery, the pose 18-21 arms, the HWater table, the flutter-skip and the moon FX
    **cannot fire** (nothing ever sets bit 1) — they belong to other levels (or need a synthetic
    layout);
  * bit 2 (the moon item) can fire if a `$88` object is touched — level 20's stream has no `$88`,
    so its bit-2 arm is also unreachable there.
* Level indices with `space_physics != 0`: 18 (`$08`), 19 (`$01`), 28 (`$08`).
* The probe value space is the raw layout byte: `$1EFA` (`00:1EFA`) computes
  `hl = $B000 + row*$100 + col` from the staged point and returns the byte — the same byte the port's
  `PlayerSim::tile` reads (plus `layout_writes`), so the counts above are directly usable.

---

## 3. Flags — claims I could NOT verify to the standard of the rest

1. **Class names/identities.** I did not match any of the four classes to a named SML2 enemy (there
   are no labels in the disassembly and no romhack docs consulted). Behaviour is quoted from the
   bytes only.
2. **The extractor's `states` list over-reports for anims 92/94/95.** `anim_profiles_b[95].states`
   lists 13 addresses (`$59FFD`, `$5A37A`, `$5AED9`…`$5BE77`) while the handler's inline table at
   `$728F` has exactly 3 (`$724A`, `$7295`, `$72AA`); the extra addresses are in other code blocks
   (§ the `$6ED9`/`$74xx` helper region). I verified the 3 real sub-states by reading the table
   bytes; I did **not** verify where the extractor's scan region ends for these anims.
3. **`type_sets` under-reports for the same classes** (types `$4C`/`$48` are written in the state
   bodies). Verified by reading the bodies; the reason the extractor misses them is not established.
4. **`$7684`'s exact role in the anim-30 stub** (`$586EB`) is quoted but unreached by level 20; no
   capture or caller confirms the 4-frame toggle's effect statically beyond the code.
5. **Profile tables `$7488`/`$74A1`**: byte-exact from the ROM, but I did not simulate the resulting
   motion (no emulator run) — the "floats up/down" description is a reading of the code, not a
   measurement.
6. **`$FFBB`'s meaning** (the flag set around the extra sprite at `$5503`) is inferred from
   `$52D5` (`and a / jr nz` skips the attr `set 7` = the X-flip) — I did not trace every `$FFBB`
   writer (20+ sites) to confirm it is *only* the flip flag.
7. **The moon FX draw** (`$6485`, bank 1) was read only far enough to establish that
   `sMoonFxState`/position latch is consumed there; the per-state art paths (`$64F9` etc.) were not
   read.
8. **Level-20 reachability of the anim-92 `$FFDB` sprite mutation's *visual* result** depends on the
   port's `refresh_hitbox` after the handler (which I read: it uses `st.b[B_SPRITE]` post-handler),
   but no capture exercises it — the sprite-id animation is statically verified only.
9. **I did not run the emulator** for anything in this report (no captures, no live probes); every
   claim is ROM-bytes + static reading, except where the *port's* existing behaviour is quoted from
   its own source and comments.
10. **The `$0FF2` routine's interaction with `sFlutterTimer`** (the port has no `sFlutterTimer`
    field yet — I did not check whether the port models it under another name; a `grep flutter` shows
    only `sFlutterArmed`/`sFlutterCharge`-style fields absent from the port, so (6) may need a new
    field too).
11. **`sSsMoonPhysics` initial value in the port** (`moon: 0`, `respawn` does not touch it) is read
    from source, not measured; whether any existing capture's first record accidentally carries a
    nonzero `moon` is unknown (the captures do not record it).
