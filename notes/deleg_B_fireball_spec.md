# Delegation B -- fireball / projectile family ($2FED / $324F / $3415) -- porting spec

Read-only investigation, 2026-10-14. Every claim below was cross-checked against the raw
bytes in baserom.gb with the scratch disassembler (`gbdis.py`, file offsets); in bank 0
(home.asm) the bank-local address IS the file offset. The home.asm listing text was used
only as a cross-check; where the two could disagree the bytes win. The phase table, the
spawn immediates, the break test and the probe tests were all re-read from the ROM bytes.

Framing correction to the queue: `$2FED` is NOT the fireball spawn. It is the
fireball-vs-object interaction pass (called only from `$3261`). The spawn is `$32C1`.
The whole family is four consecutive tail calls of the player update `$0F71`.

---

## 0. Where the family sits in the frame

The in-level frame body is `$05D5` (home.asm), once per frame from the `$FF9B = 4` state:

```
$05D5: while a fade timer runs: hKeysPressed = hKeysHeld = 0   (no firing during fades)
       call $2C61                ; frame selection
       if sMarioDead == $FF -> $0640  ; dead tail: NO $0F71, NO $2C82
       latch Y/X into sMarioY/sMarioX
       if sScreenTransition != 0 -> skip the player update ($060F)
       call $0F71               ; >>> the player update
$060F tail: $0879, $0B2A, $2062, $2D0A,
       call $2C82               ; = bank1 $5997 bool: the block-bump tick (same frame, after $0F71)
       if sPipeTravelDirection < $10: call $2D7D   ; Mario-vs-object pass
       ... moon FX $3753, $2A58, sSavedOAMIndex, $0652, $2CFF, $273E
```

`$0F71` tail (addresses exact, byte-verified):

```
$0FA8 call $319F   ; death dispatch
$0FAE call $3451   ; item consumer        (already ported)
$0FB1 call $32C1   ; >>> fireball SPAWN   (B press)
$0FB4 call $331C   ; >>> fireball MOVE    (2 slots)
$0FB7 call $324F   ; >>> fireball DRAW + CULL + OBJECT PASS (2 slots)
$0FBA ret
```

Consequences that matter to the port:
* Spawn runs before move before collide, so a fireball created on the press frame also
  moves and collides on that same frame.
* `$3451` runs before `$32C1`: an item consumed on frame N satisfies the spawn gate
  (sCurPowerup == 3) on frame N itself.
* `$2C82` ($5997 block tick) runs later in the same frame, so a bump slot allocated by
  either the player bump or a fireball break ticks (timer 1 -> 2) and performs its
  `$5CF1` level-map write during the same frame, after the player update.
* While `sMarioDead == $FF` nothing in this family runs; during a `sScreenTransition`
  the whole `$0F71` is skipped; during a fade the keys read as 0.

---

## 1. Routine boundaries (start..end file offsets, inclusive) and call sites

| Site | Range | Role | Callers |
|---|---|---|---|
| `$2FED` | `$2FED-$3006` | fireball-vs-object pass: walk the 16 object slots (`hl = $AD00`, stride `$20`, stop when `h == $AF`); for each slot with `[slot+$17] == 0` call `$3007` | only `$3261` (at `$32AE`) |
| `$3007` | `$3007-$307C` | one-slot hit test + response (internal labels `$3071`, `$307C`) | only `$2FED` (at `$2FF9`) |
| `$324F` | `$324F-$3260` | per-frame pass over the 2 projectile slots at `$A880/$A890` (stride `$10`, stop when `l == $A0`); per active slot call `$3261` | `$0F71` tail |
| `$3261` | `$3261-$32C0` | per-slot body: screen pair, off-window cull, sprite staging, `$2FED` object pass, kill on hit (internals `$329E`, `$32BD`) | only `$324F` loop (`call nz`) |
| `$32C1` | `$32C1-$331B` | SPAWN (internals `$32D9` scan, `$32E7` init, `$330E`) | `$0F71` tail |
| `$331C` | `$331C-$332D` | per-frame move pass over the same 2 slots; per active slot call `$332E` | `$0F71` tail |
| `$332E` | `$332E-$3414` | per-slot mover + collision (internals `$336E` left-move, `$3386` probe 1, `$33B4` descend arm, `$33C7` probe 2, `$33F1` phase--, `$33FB` write-back) | only `$331C` loop (`call nz`) |
| `$3415` | `$3415-$3438` | break-or-die (internals `$341C` break arm, `$3430` kill) | from `$332E` twice: `jp c` at `$3389`, `jr z` at `$33D5`-ish (probe-2 arm) |
| `$3439` | `$3439-$3450` | 24-byte phase -> vertical-step table (not code) | `$332E` |
| `$3451` | ... | item consumer (already ported; ids: 1 -> tier 1, 2 -> tier 3 fire flower, 3 -> tier 2) | `$0F71` tail |

Shared helpers the family calls (all file offsets):

| Helper | Range | What |
|---|---|---|
| `$1EFA` | `$1EFA-$1F2E` | tile read at `(Y-16, X-8)` from the WRAM layout: sets `sCollisionTile` (`$A20D`) and returns A. Address = `$B000 + ((Y-16)>>4 & $FF)*$100 + ((X-8)>>4 & $FF)` built by nibble math on `$FFB7-$FFBA` (borrow into the high bytes, hi masked `$0F`). Identical to the port's `PlayerSim::tile()` indexing including `layout_writes`. Used by Mario's probes too, and by `$5CF1`. |
| `$1F2F` | `$1F2F-$1F50` | same read without the `-16/-8` offsets (not used by this family) |
| `$2CF4` | trampoline | bank switch to bank 1 `$5297` (range `$5297-$52E6`): the shared OAM sprite stager. Inputs: `$FFC4` y, `$FFC5` x, `$FFC6` mapping id, `$FFC7` (nonzero -> set bit 4 of every attr byte = Y flip), `$FFBB` (nonzero -> attr bit 7 left CLEAR; zero -> set bit 7), `$FF8D` OAM shadow cursor. Mapping data = `[dy, dx, tile, attr]` groups terminated by `$80`. |
| `$2C6C` | trampoline | bank switch to bank 1 `$5960` (range `$5960-$5996`): the block-bump slot alloc. B register = bump type. Tail is `$2F72` (below-hit pass). Already ported as `block_bonk`/`block_slots` plumbing. |
| `$2C77` | trampoline | bank switch to bank 1 `$591F`: coin credit + coin-pop slot (the `$01` player bump's coin). NOT called by the fireball path. |
| `$2C82` | trampoline | bank switch to bank 1 `$5997`: block-bump tick. Already ported as `block_update`. |
| `$2F72` | `$2F72-$2FAC` | alloc tail: clears `sObjectBumpSpecial`, converts `$FFB7/$FFB9` to the bonk's screen pair, walks the object slots (`[slot+$17]==0`) calling `$2FAE`. Runs for EVERY successful bump alloc, including the fireball's. |
| `$2FAE` | `$2FAE-$2FEC` | if `[slot+$12]` (type) != 0, `| [slot+9] - bonkY | < $10` and `| [slot+$A] - bonkX | < $14`: sets `sObjectBumpHit = $FF`, sets bit 5 of `[slot+$17]`, and if type == `$80` sets `sObjectBumpSpecial = $FF`. |
| `$5CF1` | `$5CF1-$5D24` | the level-map write: `[$B000 + ((Y-16)>>4)*$100 + ((X-8)>>4)] = A` with the SAME address math as `$1EFA`, from `$FFB7-$FFBA` (which `$5997` reloads from the bump slot before dispatching). |
| `$5A07`/`$5A72`/`$5A82`/`$5C01` | bank 1 | the type-1 bump's tick arms: timer 2 -> `$5A72` (write `96` = `$60` into the map + jump `$5C01`: four `$FF` bytes into the BG VRAM at the cell, art only); timer `$28` -> `$5A82` (free the slot + rewrite `$60`); other timers -> `$5A07` (the two-sprite arc using table `$5A4A`, frames 101/102). |

`$324F`/`$331C` are NOT arms of a switch. They are plain consecutive calls of the `$0F71`
tail. The only dispatcher above them is the `$FF9B = 4` game-state body (`$05D5`).

`$2FED`'s scan gate is `[slot+$17] == 0` exactly. Compare `$2D7D` ($2D83: `and $F7`), which
allows bit 3 (a slot Mario is standing on is still hittable by Mario but NOT by a fireball).

---

## 2. Projectile state

### 2.1 The two slots

`$A880` and `$A890`, stride `$10`, exactly 2 slots (both scans stop at `l == $A0`).
Layout (ram.asm 710-713 agrees; byte-verified from the four accessors):

```
+0  active      (1 while live; all kills write 0)
+1  Y lo        (world Y)
+2  Y hi
+3  X lo        (world X)
+4  X hi
+5  direction   ($FF = left, $01 = right)
+6  phase       (0..24, indexes the $3439 table)
+7..+15         never touched by this family
```

Do NOT name these after the object `B_*` constants - the fireball slots are a separate
global array, not object slots and not the `$FFD0` staging copy. Suggested naming for the
port: `F_ACTIVE=0, F_YLO=1, F_YHI=2, F_XLO=3, F_XHI=4, F_DIR=5, F_PHASE=6`, stride 16,
2 slots.

Spawn init (see 3.4) writes `+0..+6` only. Nothing reads `+7` anywhere (grep `$A887`/`$A897`
and the two scan bodies: only `+0` is read for liveness). A spawn must NOT write `+7`.

### 2.2 The globals the family keeps

| Byte | Name | Written by |
|---|---|---|
| `$A25B` | `sFireballActive` | `$332E` first instruction (the slot's `+0`) |
| `$A25D/$A25E` | `sFireballY/YU` | `$3261` (Y lo only) and `$332E` (lo+hi) |
| `$A25F/$A260` | `sFireballX/XU` | `$3261` (X lo only) and `$332E` |
| `$A212` | `sFireballDirection` | `$3261`, `$332E` |
| `$A259` | `sFireballPhase` | `$332E`, `$341C`-adjacent popup (`= $18`), `$33F1` |
| `$FFB7/$FFB8` | probe Y staging | `$332E` (Y lo/hi), then `$5997` reloads them from the bump slot |
| `$FFB9/$FFBA` | probe X staging | `$332E`; also `$3261` writes the X lo back after the move? no - `$332E` only |
| `$FFC4/$FFC5` | sprite screen pair | `$3261` (the cull + draw pair) |
| `$FFC6` | sprite mapping id | `$3261` |
| `$FFBB` | priority clear flag | `$3261` (set to 1 before the draw) |
| `$FF97` | rDIV | read for the anim frame and the spin-fire direction |
| `$A20D` | `sCollisionTile` | `$1EFA` (both probes) |
| `$A246` | `sObjectHitResult` | `$3261` clears it, `$3007` sets it |
| `$A2A9-$A2AF` | `sObjectType/BoxTop/BoxBottom/BoxLeft/BoxRight/Y/X` | `$3007` scratch (same vars Mario's pass uses) |
| `$A460`/`$A470`/`$A478` | `sSfxRequest0/1/2` | sfx0 = 11 (fire), 2 (bump/death); sfx2 = 1 (smash) |
| `$A220` | `sSpinJumpBounce` | `$341C` (`= $FF`) |
| `$A25A` | `sMarioThrowTimer` | spawn sets 8; the only reader/decrementer is the Mario DRAW (`$548A`, bank 1): powerup 3 draws mapping +112, and while the timer is nonzero it decrements once and draws the throw pose (+32). Purely a pose - it is NOT a fire-rate gate. |

Object-side bytes the family touches (these DO map to the port's B_* constants from
objects.rs, because the object slots are the same table the port models):

* `$2FED`/`$3007`: reads `B_TYPE` (+18), `B_HB_TOP/BOTTOM/LEFT/RIGHT` (+19..+22),
  `B_SCRY/B_SCRX` (+9/+10), gate on `B_FLAGS` (+23) == 0; writes `B_FLAGS` bit 1
  (defeat) for a type-`$04` object.
* `$2FAE` writes `B_FLAGS` bit 5 (hit-from-below).

---

## 3. The passes in detail

### 3.1 `$32C1` spawn (gate order matters)

```
1. hKeysPressed bit 1 (B new-press) else ret      ; the fire button is B (edge).
2. sCurPowerup == $03 else ret                    ; fire flower tier only.
3. sMarioDucking == 0 else ret
4. sGoalBell == 0 else ret
5. scan $A880, $A890 (stride $10, until l == $A0): first slot with +0 == 0 wins;
   none free -> ret (silent, no sfx, no timer).
6. +0 = 1
   +1 = $FFC0 + $1C (Mario Y + 28), +2 = $FFC1 + carry
   +3 = $FFC2 + $10 (Mario X + 16), +4 = $FFC3 + carry
   +5 = sMarioDirection ($A22B: $FF left / $01 right)
        if sSpinJump != 0: +5 = ($FF97 & $02) - 1   ; bit clear -> $FF, set -> $01
   +6 = 0
   sMarioThrowTimer = 8 ; sSfxRequest0 = 11 ($0B)
```

The scan precedes the init, so a spawn only writes the free slot. Byte proof
(`$32E7-$331B`): 3E 01 22 F0 C0 C6 1C 22 F0 C1 CE 00 22 F0 C2 C6 10 22 F0 C3 CE 00 22
FA 2B A2 77 FA 68 A2 A7 28 06 F0 97 E6 02 3D 77 23 AF 77 3E 08 EA 5A A2 3E 0B EA 60 A4 C9.

### 3.2 `$324F`/`$3261` draw + cull + object pass (per active slot, after the move)

```
load +1 -> sFireballY, +3 -> sFireballX, +5 -> sFireballDirection  (lo bytes only)
sx = (Xlo - $FFCA + $50) & $FF          ; $FFCA = camera X lo
if (sx & $F0) == $C0 -> kill the slot ([slot+0] = 0), no sfx, RETURN
$FFC5 = sx
sy = (Ylo - $FFC8 + $46) & $FF          ; $FFC8 = camera Y lo
if (sy & $F0) == $C0 -> kill, RETURN
$FFC4 = sy
b = (dir == $FF) ? $B0 : $B4            ; $B0-$B3 = left frames, $B4-$B7 = right frames
$FFC6 = b + (($FF97 & $06) >> 1)        ; 4-frame flicker, rDIV bits 1-2
$FFBB = 1
call $2CF4                              ; stage/draw the 2-tile mapping
sObjectHitResult = 0
call $2FED
if sObjectHitResult != 0 -> kill the slot, RETURN
```

The cull is a silent kill. The mapping ids `$B0/$B4` + 0..3 are real entries in the bank-1
mapping pointer table at `$4000` (entry n at `$4000 + n*2`, pointers into the `$41E4`
blob); each fireball frame is 2 tile quads, e.g. `$B0 -> $4862: FC F8 73 20 FC 00 72 20`.
The `$-mask` semantics of the `$C0` window is NOT resolved (see section 9); port the two
expressions literally, byte-for-byte, and pin them in tests.

### 3.3 `$331C`/`$332E` mover (per active slot, after the spawn)

```
sFireballActive = 1
load +1..+5 into globals; $FFB7/$FFB8 = Y/YU, $FFB9/$FFBA = X/XU; phase -> sFireballPhase
b = direction << 1
if b >= $80 (dir $FF): X -= 3        ; exact: dec b; negate -> 3
else:                  X += 3        ; exact: inc b -> 3 (dir 0 would give +1; unreachable)
probe1: call $1EFA (reads the tile at (Y-16, X-8) AFTER the X move)
   if tile < $38 -> $3415   (break if == $06, else DIE)
Y step: b = $3439[phase]
   if phase >= $0C: Y -= b   (rise)
   else:            Y += b   (descend)
probe2: call $1EFA (same point, after the Y move)
   if tile >= $48 -> skip
   if phase >= $0C -> skip
   if tile == $06 -> $341C (break)
   else: phase = $18; Y -= 3            (the ground/wall popup)
if phase != 0: phase -= 1
write back +1..+4, +6 (never +5)
```

So: X is always +-3 px/frame; Y follows the `$3439` arc; the projectile never sinks more
than one 16-px row into a solid because probe 2 bounces it (phase 0 is < 12 while
descending, so the first solid touch pops it). Probe 1 kills on ANY tile `< $38` other
than `$06` - including the `$00` brick and the `$01` item block - and on `$06` it breaks.
Probe 2 bounces (pops + phase 24) on any tile `< $48` that is not `$06`; `$48+` tiles
(coins, water, hazards, decor) are transparent to the fireball.

`$3439` (24 bytes, byte-verified): `03 03 03 03 02 03 02 02 01 02 01 00 00 01 02 01 02 02 03 02 03 03 03 03`.

### 3.4 `$2FED`/`$3007` object pass (per active fireball, same frame, after the move)

```
for slot in $AD00..$AEE0 (stride $20):
   if [slot+$17] != 0: next
   call $3007:
      type = [slot+$12]; $FF98 = type
      if type == 0: ret            ; empty slot
      if type & $80: ret           ; item types ($80-$85, $89) are transparent
      top/bottom/left/right = [slot+$13..$16]
      oy = [slot+9] (B_SCRY), ox = [slot+$A] (B_SCRX)
      y0 = oy + top - 6;   h = bottom + 6 + oy - y0
      if !(($FFC4 - y0) < h)  (unsigned): ret          ; $FFC4 = the fireball's sy
      x0 = ox + left - 6;  w = right + 6 + ox - x0
      if !(($FFC5 - x0) < w)  (unsigned): ret          ; $FFC5 = the fireball's sx
      if type & $04: [slot+$17] |= $02; sObjectHitResult = [slot+$17]   ; DEFEAT the object
      else:          sSfxRequest0 = 2;  sObjectHitResult = 2            ; just die
```

Both outcomes leave `sObjectHitResult != 0`, and `$3261` then kills the fireball. So a
fireball always dies on contact with a live, non-item object; it defeats it iff the
type has bit 2 set (the port's `dispatch_below`/`$85AA` bit1 arm consumes the flag).
The +/-6 expansion applies to top/left and bottom/right symmetrically. The gate is the
slot flags byte == 0 (bit 3 included), unlike `$2D7D`.

The test uses the object's stored screen pair (slot +9/+10, written by the previous
frame's draw) against the fireball's own `$3261` pair. See section 9 for the
coordinate-frame relation (derived, not capture-verified).

### 3.5 The break: `$3415`/`$341C`/`$3430` exactly

```
$3415: if sCollisionTile != $06 -> $3430
$341C: sSfxRequest2 = 1; b = 1; sSfxRequest0 = 2; call $2C6C (bank1 $5960, TYPE 1);
       sSpinJumpBounce = $FF
$3430: [slot+$0] = 0            ; kill this fireball
       sSfxRequest0 = 2
       ret
```

Byte proof: `FA 0D A2 FE 06 20 14 3E 01 EA 78 A4 06 01 3E 02 EA 60 A4 CD 6C 2C 3E FF EA 20 A2 E1 AF 77 3E 02 EA 60 A4 C9`.

What each piece writes:

* **The bump slot** (`$5960`, shared with the player's bump): `+0 = 1` (type), `+1 = 1`
  (state), `+2 = ($FFB7 & $F0) + 8`, `+3 = $FFB8`, `+4 = ($FFB9 + 8) & $F0`, `+5 = $FFBA + carry`.
  At this moment `$FFB7/$FFB8` = the fireball's post-move Y and `$FFB9/$FFBA` = its
  post-move X (both set by `$332E`), so the cell is the fireball's probe cell. If no
  bump slot is free the alloc silently drops (the `cp $00 / ret z` wrap) and the tail
  never runs - the fireball still dies.
* **The below-hit tail** (`$2F72`): clears `sObjectBumpSpecial`, re-stages
  `$FFB7 = (($FFB7 & $F0) + 8 - sScrollY - 16) & $FF`, `$FFB9 = (((($FFB9 + 8) & $F0) - sScrollX - 8) & $FF`,
  then walks the object slots (same `[slot+$17]==0` gate, `$2FAE` with the `$10`/`$14`
  window). It runs for the fireball's bump too, so a type-`$80` item pose sitting in the
  window gets its bit 5 (below-hit) armed exactly as from a player bump. Port it through
  the existing `below_bonk` seam, but with the FIREBALL's Y/X low bytes (the current
  `block_alloc` uses `self.ceiling_probe` - a fireball variant is needed).
* **The level map** (`$5CF1`, called from the type-1 tick at timer 2 via `$59EB -> $5A72`):
  writes `96` = `$60` into `$B000 + row*$100 + col` where row/col are recovered from the
  SLOT's stored point by the same `(Y-16)>>4 / (X-8)>>4` math - i.e. exactly the cell the
  fireball's probes just read, and exactly the cell the port's `tile()` would read for
  that point. In port terms: `layout_write(row, col, 0x60)` with
  `row = ((fy - 16) >> 4) & 0xFF`, `col = ((fx - 8) >> 4) & 0xFF` (16-bit wrapping
  subtract, low-byte result; same indices as `PlayerSim::tile`). Written again at timer
  `$28` (`$5A82`), then the slot frees.
* **Art-only**: `$5C01` (from the same timer-2 arm) writes four `$FF` bytes into the BG
  VRAM at that cell (2x2 tiles, address from `$200C`); the `$5A07` arm stages sprite
  frames 101/102 on the `$5A4A` arc. These change no collision state; the port keeps
  them in the existing `block_draw`/renderer path.

Relationship to the ported brick bump:

* The fireball break IS a type-1 bump (the queue title is accurate). Type 1 = the same
  type the big-Mario `$00` brick bonk allocates (`$19B0` big arm: b=1), and its map write
  is the same `$60` empty metatile. The player path differs only in: (a) the bonk point
  comes from Mario's ceiling probe, (b) the `$19D2` coin path (`$2C77`/$591F + the type-4
  slot) is not involved, and (c) the fireball is killed afterwards.
* The fireball does NOT go through `block_bonk(tile)`'s tile classifier. It calls the
  alloc directly with b=1 hardcoded after testing `sCollisionTile == $06`. So `$06` is
  the only tile a fireball can break, and it is the only tile whose break reuses type 1
  from a projectile. The player can NEVER bump `$06` (`$1993`: `cp $04 / ret nc`), so
  nothing else can produce that write.
* Cell semantics for later interactions: `$06` is Solid (blocks walking, standable,
  probe `< $38`); `$60` is decor (passable, falls through both probe classes). After the
  write the cell must read `$60` to every subsequent probe - Mario may walk through and
  the `layout_writes` overlay is the port's mechanism for exactly that. The port's
  `block_update` already performs this write for type 1 at timer 2 (verified in
  player_sim.rs `$5CF1` doc); nothing new is needed there beyond feeding it the right
  cell via the slot.

---

## 4. Limits, cooldown, input, and the tables

* **Live limit: 2** (two `$10`-byte slots at `$A880/$A890`). A B press while both are
  live is silently eaten (the scan returns without sfx or timer). There is no other cap.
* **Cooldown: none.** The press is edge-triggered (`hKeysPressed` bit 1), so holding B
  fires once; mashing B alternates slots. `sMarioThrowTimer = 8` is only the Mario throw
  POSE (draw-side, `$548A`); it gates nothing.
* **Fire button**: B. The ROM's `hKeysHeld` layout: bit 7 down, 6 up, 5 left, 4 right,
  3 start, 2 select, 1 B, 0 A (decoded from `ReadJoypad $1FD2`). Feed the port a
  `b_pressed` edge derived exactly like the existing `a_pressed` (`f.b && !prev_b`); the
  replay `Frame` already carries `b`.
* **Tier gate**: `sCurPowerup == 3` (fire flower). Item id 2 -> tier 3 (`$3467`).
* **Tables the break/projectile consult**:
  1. the `$3439` phase -> Y-step table (section 3.3);
  2. the tile-class thresholds it tests directly: `$38` (probe 1: kill/break), `$48`
     (probe 2: bounce window), and the literal `$06` (breakable);
  3. NOT relevant to the fireball but to the target cell: the bump-type -> replacement
     tile table in `block_update` (1 -> `$60`, 5 -> `$7F`, 2 -> `$18` then `$00`,
     3 -> `$18` then `$07`, 6 -> `$18` then `$03`, 4 -> never) - already ported.

---

## 5. FEASIBILITY: which level's blocks yield a fire flower

There are two distinct fire-flower sources in the ROM, and they use different code:

**(a) Item POSE objects** (the invisible block contents). Handlers of anims 17-21 set
type `$80`; the below-hit dispatch ($86A0 -> $86E4) does: for anim 17 with Mario big, a
`cp $11` rewrite swaps the record's anim first; then $8708 sets
`type = anim + $70` and `[$FFD2] = $10`. So anim 17 -> `$81` (mushroom) or, with big
Mario, a fire flower; anim 18 -> `$82` (fire flower); 20 -> `$84` (star); 21 -> `$85`
(1-up). Collected via `$2DC6` (touch, bit 4) or `$2FAE` (below-hit, bit 5).

**(b) MARKER objects** (anims 27/28/29 -> handler `$586EE`, bytes `F0 D5 C6 66 E0 E2 C9`:
`type = anim + $66`). These are already-out, collectible items. All 9 records in the
game, from the level manifests (independently re-derived below):

| level (idx) | rec | anim | x, y | sits on |
|---|---|---|---|---|
| level08 (7) | 15 | 27 mushroom | 1816, 496 | col 113 row 31 - `$24`/`$27` solid fill area |
| level08 (7) | 19 | 29 carrot | 2104, 432 | col 131 row 27 - `$00` brick |
| level08 (7) | 20 | **28 fire flower** | 2152, 368 | col 134 row 23 - `$00` brick (bumpable + touchable) |
| level12 (11) | 22 | 27 | 2824, 368 | col 176 row 23 - `$38` platform |
| level12 (11) | 24 | **28 fire flower** | 2952, 368 | col 184 row 23 - `$38` platform (touch only) |
| level12 (11) | 25 | 29 | 3016, 368 | col 188 row 23 - `$38` platform |
| level16 (15) | 31 | **28 fire flower** | 2088, 544 | col 130 row 34 - `$07` solid (touch only) |
| level20 (19) | 4, 27 | 27, 27 | 1032/3576, 48 | near the top of the map |

Anims 27/28/29 dispatch to the same handler in BOTH mode A and mode B tables
(`$58040[27..29] = $58040+[26,30,31]`-checked: entries are `46EE 46EE 46EE` in both), so
the markers are live in every level, not only the mode-B ones. Sprite ids are
`anim_to_sprite[27..29] = 25/26/27`.

**Best candidate (already the queue's planned route, and it checks out): LEVEL 1,
level index 0, node 0.**

* Fire-flower source: spawn record 16, anim 18, at x = 3016, y = 440
  (`raw = [11, 200, 0, 1, 184, 18]`). It is a type-`$80` pose sitting over the `$01`
  block at col 188, row 28 (layout row 28 cols 187-189 = `01 01 01`, air above).
  Bump the `$01` from below: `$19D2` allocates the type-3 bump, `$2F72` catches the pose
  (withholding the coin - `sObjectBumpSpecial`), and the object emerges via `$8708` as
  `type = 18 + $70 = $82` -> touch -> `$2DC6` stages pending item 2 -> `$3451` on the
  next frame's tail grants tier 3. Level 1 has 0 unported handler classes (level_targets
  appendix), so the capture route needs no new object-side work beyond this family.
* Fireball-break target in the SAME level: `$06` cells at row 29, cols 211-214 and
  217-220 (x 3376-3440 and 3472-3520), i.e. ~360-500 px east of the flower block, same
  ground line. These are the only `$06` cells in level 1 (8 cells).
* Both facts were re-verified here directly from `level01.manifest.json` +
  `level01.decompressed.bin` (records, raw bytes, layout cells), and the `$8708` /
  `$86E4` / `$586EE` code was re-read from the ROM.

Alternates if level 1 is unsuitable: level08 idx 7 (marker fire flower at x 2152, on a
`$00` brick - bumpable AND touchable) and level12 idx 11 (markers at x 2952 on a `$38`
platform - touch only, no bump path for `$38`). Note neither of these levels needs the
pose machinery, but both have more unported handlers (9 and 4 in the appendix).

Practical note for the capture: `$06` has NO bump path from Mario, so the break can only
be shown with a live fireball; and a fireball dies on the `$00`/`$01`/`$03` cells, so the
shot must be taken west of col 211 with the fireball rolling east along row 29.

---

## 6. Port plan (what to add where)

1. **Fields** (player_sim.rs): `pub fireballs: [[u8; 16]; 2]` (raw bytes, `F_*` consts);
   `pub fire_cull: [bool; 2]`? not needed - implement the kill inline. Reuse existing
   `collision_tile`, `layout_writes`, `spin_jump_bounce`, `sfx0`, `sfx2`, `object_bump_special`,
   `below_bonk`, `block_slots`. Add `throw_timer: u8` ($A25A) and a `fire_scan: [Option<(u8,u8)>; 2]`
   (the per-slot screen pairs that need the object scan) plus a `fire_hits: [bool; 2]`
   resolved by the frame loop, OR route the object pass directly if the sim owns the
   object slots. Zero `fireballs` in the level-reset next to `block_slots` (the ROM's
   boot-time WRAM clear covers $A880-$A89F; no per-level clear found - see section 9).
2. **step() tail**: after `self.item_consumer()` add, in this order:
   `self.fire_spawn(keys, b_pressed);` -> `self.fire_move(layout);` ->
   `self.fire_pass(layout);` (stages pairs/culls). Caller passes `b_pressed` like `a_pressed`.
   The object-side scan (the `$2FED` walk) must run between the player update and
   `obj.step`, exactly like the existing `below_bonk` seam in main.rs (lines ~1317/2164);
   the returned hit flags then kill the fireballs before the object engine consumes the
   flags. The bumps from a fireball break must feed the same `below_bonk` seam (with the
   fireball's Y/X instead of the ceiling probe).
3. **Inertness for the existing replay gate**: with no active slot and no B press, all
   four calls must be pure no-ops that touch no byte (byte-identity requirement for the
   retired-capture replay). The scans only read; `$32C1` returns at the key/powerup
   gates. Add a test that pins this.

## 7. Unit tests (the project's standard: build state in memory, run the handler, assert bytes)

1. `fire_spawn_writes_the_full_slot`: powerup 3, b_pressed, camY/X/Mario Y/X set ->
   assert the 16 raw slot bytes (only `+0..+6` changed; `+7..+15` untouched),
   throw_timer 8, sfx0 11.
2. `fire_spawn_gates`: powerup != 3 / ducking / goal bell / held-B (no edge) each leave
   the slot untouched.
3. `fire_spawn_uses_the_first_free_slot_and_eats_the_third`: fill slot 0 -> second press
   writes slot 1; both live -> third press is a no-op.
4. `fire_move_moves_three_px_and_steps_the_phase_table`: dir 1 -> X+3; dir `FF` -> X-3;
   phase 0 descends by `table[0] = 3`; phase >= 12 rises.
5. `fire_move_pops_on_solid_and_breaks_on_06`: probe tile `$27` with phase < 12 ->
   phase `$18`, Y-3, then 23; probe tile `$06` -> `block_slots[0]` type 1 with the
   fireball's cell bytes, spin_jump_bounce `$FF`, sfx0 2, sfx2 1, slot killed;
   `block_update()` once -> `layout_writes` gains exactly the cell key with `$60`.
6. `fire_kills_on_other_special_tiles`: probe tile `$01`/`$00`/`$03` (< $38, != 6) ->
   slot killed, sfx0 2, no bump slot.
7. `fire_object_pass_defeats_bit2_types`: object slot with flags 0, type `$20`, hitbox,
   screen pair overlapping -> `B_FLAGS` bit 1 set, fireball killed.
8. `fire_object_pass_dies_on_plain_objects_and_skips_items`: type `$10` -> sfx0 2 and a
   kill; type `$82` -> untouched, no kill; flags bit 3 set -> skipped.
9. `fire_cull_masks`: `(sx & $F0) == $C0` and `(sy & $F0) == $C0` each kill silently
   before the draw/object pass.
10. `fireball_calls_are_inert_without_slots`: step a frame with no live slot and no B
    press; assert every fireball/global/scratch byte is unchanged (the replay-gate
    guarantee).

## 8. Byte-verified appendix

* `$2FED` head: `21 00 AD E5 7D C6 17 6F 7E A7 20 03 CD 07 30 E1 11 20 00 19 7C FE AF 20 ...`
* spawn init (section 3.1) and break (section 3.5) byte strings above.
* probe 1: `CD FA 1E FE 38 DA 15 34` (`call $1EFA; cp $38; jp c, $3415`).
* popup: `3E 18 EA 59 A2 FA 5D A2 D6 03 EA 5D A2 FA 5E A2 DE 00 EA 5E A2`.
* phase table `$3439-$3450`: `03 03 03 03 02 03 02 02 01 02 01 00 00 01 02 01 02 02 03 02 03 03 03 03`.
* `$3261` mapping id: `06 B0 FA 12 A2 FE FF 28 02 06 B4 F0 97 E6 06 CB 3F 80 E0 C6` +
  `3E 01 E0 BB CD F4 2C`.

## 9. Claims NOT verified (and what would settle each)

1. **The `(v & $F0) == $C0` cull**: I could not resolve what screen region `$C0-$CF`
   represents (off-view / wrap-edge / signed-negative). Port the two expressions
   literally; a capture of a fireball flying to either edge (or a unit test comparing
   against the console) settles it.
2. **The coordinate-frame relation in the object test**: the fireball's pair uses
   `- camX + $50` / `- camY + $46` while the objects' stored pair uses
   `(x + 8) - sScrollX` / `(y + 16) - sScrollY`. Taking the port's measured
   `sScroll = cam - 80 / cam - 72`, the fireball's tested point maps to object space as
   `(fx - 8, fy - 18)`. Derived, not independently capture-verified; a capture where a
   fireball meets an object slot would pin it.
3. **`$FFC7` at the fireball draw**: `$3261` never writes `$FFC7`, but `$5297` reads it
   per tile (nonzero -> attr bit 4 Y-flip). Its value depends on the last writer in the
   frame. If the port's sprite model tracks it, do not set it for fireballs; verify
   against a live OAM dump.
4. **The 4-frame anim `($FF97 & 6) >> 1`**: reads rDIV bits 1-2. Captures record only 3
   bits of `$FF97` (the port's `phase`), so the mapping id is reproducible only when the
   higher bits are right. Unit-pin the expression; a capture logging `$FFC6` settles it.
5. **Fireball-slot clearing at level load**: no literal writer for `$A880-$A89F` was
   found (only Init's boot WRAM clear covers it). Recommended: zero the slots in the
   port's level reset; a capture starting a level with a live fireball would settle
   whether the ROM clears them too.
6. **Fireball break of a `$06` cell that carries an item pose**: the alloc tail `$2F72`
   runs for the fireball's bump in code, so the emergence should trigger, but no level
   was found where a `$06` cell has a pose in the window; the port's `below_bonk` seam
   would do it automatically. Code-level claim only.
7. **Whether a fireball's break plays the `$2C77`/coin path**: code says no (only
   `$2C6C` b=1). Verified by reading, not by a capture.
8. **sObjectHitResult cross-talk**: `$3261` clears `$A246` before `$2FED`, so the
   `$2D7D`/`$2EDD` use of the same byte cannot interfere. Read-verified only.
