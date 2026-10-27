# SML2DX source contracts

Static phase-1 audit, 2026-10-27. These contracts describe the exact supported
ROM, not implementation completion or hardware verification. See the
[verification matrix](sml2dx-verification.md) for status, test mappings and
bounded tickets, and the [presentation audit](sml2dx-audit.md) for earlier
reference evidence. All bank:CPU ranges below have exclusive ends unless stated.

ROM identities were checked locally: DX SHA-256
`1dd604a91b9ee99ef381cc795da3ae28c72d04ba9de78b355df829e56ce49c46`;
base SHA-256 `5450dce1bd0c073964c374b5b5b5729dce8d00f2e807892c34af32b8bce1392e`,
SHA-1 `bba408539ecbf8d322324956d859bc86e2a9977b`.
No emulator execution, new captures, ROM modification or game behavior changes
were used. Bounded decoded instructions were checked against raw operands;
content and movement inventories have opt-in ROM assertions in
[`contracts.rs`](../crates/sml2-dx/tests/contracts.rs). These assertions validate
source differences, not native gameplay or visual parity.


## Regional palettes

Regional gap wording in this section refers to baseline `43588db`. Separate
implementation `291981a` now covers the branches; the verification matrix tracks
the remaining timing, ordering and independent-evidence work.

### Corrections to the existing audit and plan

The seven missing selectors do **not** all replace BG palettes from
`$6100+n*$40`. They include direct OBJ color patches, whole OBJ switches and
forward-only paired BG/OBJ progression. `$0C` is the only `$6100` selector.
The ROM also reads previously staged coordinates at a different point from the
existing native `$0C` implementation. Phase 3 must explicitly pin this timing.

### Invocation, coordinate semantics and initialization

- Bank `$01:$54CF..$54D8` replaces the tail of Mario's pose setup with a jump
  through home `$191A` to `$26:$6B00`. `$6B00..$6B0B` finishes screen-Y setup
  and clears star palette selection, then falls through to the level dispatcher
  `$6B0B..$6B39`. All branches return through `$26:$703A..$7042` to
  `$01:$54D8`, preserving the subsequent star/pose behavior.
- Normal frame source order is home `$0618` call `$2D0A` -> bank1 `$530D`
  pose setup -> selector; the object engine is later, home `$0636` call `$0652`.
  `$02:$4100..$4120` stages the 16-bit references:
  `ref_x = (FFC2/FFC3 + 8) mod 65536`,
  `ref_y = (FFC0/FFC1 + 32) mod 65536` into `$AF30..$AF33`.
  Raw exact stores occur at `$02:$4105/$410D/$4115/$411D`. The old audit's
  `$02:$40FD` is the preceding store, not this routine's start. These bytes are
  identical to the base ROM. A direct-store scan finds no other stores to these
  four addresses. Consequently normal pose setup consumes the preceding
  object-engine pass's references; a suspended object engine can retain them.
  Do not replace this with a fresh player coordinate without a timing decision
  and source-derived test. At baseline `43588db`, session code computes `$0C` after its
  frame from fresh `sim.x`. Separate commit `291981a` generalizes the regional
  state but still updates it after the frame from fresh player coordinates;
  branch tests establish sections, not the source order described here.
- `$FFAD` is persistent selector/progress state, not always an index into a
  palette array. Unmatched sections retain both the latch and palette state.
  Normally an action is skipped if its target state equals `$FFAD`.
- `$24:$7912..$793F` initializes the HUD and clears `$FFAD` at `$793C..$793F`.
  Its home wrapper `$07DB..$07EA` is called at home `$0576` during level load.
  This source is the explicit level/reload reset boundary. Initial BG and OBJ
  colors are supplied separately by `$26:$7CD6..$7D17`; the reset does not load
  regional set zero. The first selector can legitimately do nothing with state0.
- Boot home `$0225..$022E` clears `$FFFE` downward for 128 bytes, including
  `$FFAD`. This is an additional bulk reset, not visible as an `ldh` pattern.
- Other classified executable direct writes to `$FFAD` are the regional
  branches below and the hub palette latches at `$0F:$7844/$786B` and
  `$29:$648E/$64B5`. Those set 0/1 on crossing the hub camera color boundary;
  level load resets the shared byte. No bank-$26 default path resets it.
- A full-ROM byte-pattern search additionally finds `E0 AD` inside bank `$17`
  at `$44EE`; that occurrence lies in the layout-data bank and is not treated
  as executable evidence. The scan alone does not rule out arbitrary indirect
  stores. No new room/arena reset was established: preserve the latch unless
  the source path demonstrably invokes the level initializer. Test the actual
  pipe/arena/death entry paths rather than inventing a reset on every scene.

### Common transfer contracts

- `$26:$6BA7..$6BC4`: copy exactly 64 bytes from `$6100+BC` to WRAM bank2
  `$D200`, restore WRAM bank0, request palette job5 in `$FFF8`.
- Home `$1905..$191A`: copy 64 bytes from the selected bank-$26 source HL to
  `$D200`. Callers establish WRAM bank2 and restore it.
- Home `$18AF..$18DA` uploads 64 BG bytes using high/low source pointer
  `$A1DC/$A1DD`; `$18DA..$1905` uploads 64 OBJ bytes from `$A1DE/$A1DF`.
- Palette jobs at `$24:$7800..$7866`: job1 uploads the BG pointer; job2 uploads
  the OBJ pointer; job4 uploads BG from WRAM2 `$D200` plus the OBJ pointer;
  job5 uploads only BG from `$D200`. VBlank home `$01CC..$01D3` services `$FFF8`.
- Direct OBJ patches use auto-incrementing OBPI and wait until LCD STAT bit1
  is clear. No OBJ palette buffer is rewritten by these inline patches.
  OBPI `$AA` means OBJ palette5/color1; four bytes cover colors1 and2.
  OBPI `$AC` means OBJ palette5/color2; two bytes cover color2 only.

### Selector contracts

Let `XH = ref_x >> 8`, `XL = ref_x & 255`, `YH = ref_y >> 8`,
`YL = ref_y & 255`, `S = $FFAD`. All comparisons are unsigned bytes.

#### `$0C`, source `$26:$6B39..$6BA7` (already partly native)

- XH 1 or 14 -> target S0, BG `$6100`.
- XH 2,4,10,13 -> target S1, BG `$6140`.
- XH 5,7,9 -> target S2, BG `$6180`.
- Other XH retains state/colors. Each actual change uses `$6BA7`, job5, 64 BG
  bytes and leaves OBJ unchanged.

#### `$15`, source `$26:$6BC4..$6C5B`

- XH9 -> target S1, BG `$60C0`.
- XH11 with YH2 and YL<$15 -> target S0, BG `$6080`.
- XH5 first requires YH2. If YL >= $5D -> target S1, BG `$60C0`.
- **Source fallthrough quirk:** when XH5,YH2,YL<$5D, A still contains YL
  at `$6BDC`, so the following comparisons are against YL, not XH:
  YL9 -> target S1; YL11 -> target S0 (the repeated YH2/YL<$15 gates pass);
  all other values retain state. Do not simplify this to geometric regions.
- Both actual changes copy 64 bytes to `$D200`, set `$A1DC/$A1DD` to the same
  ROM source, request job1, and set S0/1. OBJ unchanged.

#### `$19`, source `$26:$6C5B..$6D1A`

- Compute byte Q = `swap(XH) + swap(XL & $80)` with byte wrapping, i.e. for
  ordinary XH<16 the units encode half-page progress (`XH*16 + 0 or 8`).
- If Q<=S, return unchanged. If Q>S, save S=Q **before** checking trigger values.
  Thus unrecognized increasing positions update the progress latch even though
  they do not update palette colors. Going left never restores earlier colors.
- Trigger Q values `$18,$20,$28,$30,$38,$40,$48,$50,$58,$60,$68,$70` select
  n=0..11. Copy 64 BG bytes `$5A80+n*$40` to `$D200`, set OBJ pointer
  `$5D80+n*$40`, request job4. Both BG and OBJ change together.
- Jumping past an exact trigger can skip its colors; do not implement `>=`
  palette regions or clamp to the last crossed palette.

#### `$03`, source `$26:$6D1A..$6DFD`

- XH0 -> target S0; copy BG `$5A00` to `$D200`, job5.
- XH1 -> target S1; copy BG `$5A40` to `$D200`, job5.
- XH5 -> target S0; directly patch OBJ palette5 colors1/2 to RGB555
  `$0995,$2E9C` (bytes `95 09 9C 2E`); no BG change.
- XH6 -> target S1; direct OBJ palette5 colors1/2 `$7FFF,$11C8`
  (`FF 7F C8 11`); no BG change.
- XH10 -> if S!=1 copy BG `$5A40` to `$D200`, set BG ROM pointer `$5A40`,
  request job1, then execute the XH6 OBJ patch and set S1.
- All other XH retain. The shared S0/1 gate controls both kinds of action;
  do not maintain independent inferred BG and OBJ selector indexes.

#### `$07`, source `$26:$6DFD..$6E5E`

- XH7 -> target S0; direct OBJ palette5 colors1/2 `$7BD9,$6B45`
  (`D9 7B 45 6B`).
- XH8 -> target S1; direct OBJ palette5 colors1/2 `$56B5,$294A`
  (`B5 56 4A 29`).
- All other XH retain. BG unchanged; neither action requests a palette job.

#### `$16`, source `$26:$6E5E..$6EB3`

- XH8 or12 -> target S0; direct OBJ palette5 color2 `$267D` (`7D 26`).
- XH9 or11 -> target S1; direct OBJ palette5 color2 `$5294` (`94 52`).
- All other XH retain. Other OBJ colors and all BG colors unchanged; no job.

#### `$0D`, source `$26:$6EB3..$6FBC`

- XH0 and YH!=0 -> target S0; BG `$61C0`.
- XH1 or5 and YH!=0 -> target **S$10**, BG `$6200`.
- XH6 and YH!=0 -> target S1, BG `$6240` (the XH6 comparison appears twice
  in the ROM; do not infer a missing XH7 case).
- XH8 or11 -> target S2, BG `$6280`, **without** a YH gate.
- Other cases retain. Each actual action copies 64 BG bytes to `$D200`, sets
  the BG ROM pointer to that source, requests job1, then sets its latch.
  OBJ unchanged. S$10 is a real state, not a typo or set number16.

#### `$18`, source `$26:$6FBC..$703A`

- XH8,YH2 -> target S1, OBJ source `$62C0`.
- XH0,YH2 -> target S2, OBJ source `$6300`.
- XH5,YH1 -> target S3, OBJ source `$6340`.
- Other coordinate pairs retain. Each actual action sets the OBJ pointer and
  requests job2; no BG copy. This can interact with later boss palette jobs,
  so preserve the source's update and scene-load ordering rather than globally
  suppressing the level selector whenever an arena is visible.

### Bounded implementation tickets and assertions

One source-selector ticket per level, after extracting native palette state and
pinning phase order; allowed implementation owners should be a focused runtime
palette-state module plus `session/level.rs` wiring, DX renderer wiring, and
focused unit/integration tests. No new importer bytes are needed for these
selectors: bank26 and its complete table bytes are already in the schema8 bundle.
Inline OBJ values above can be exact native constants with source references.

- Shared timing/init ticket: assert `ref_x/ref_y` wrapping at `$FFF8/$FFE0`,
  selector before object-engine staging, retained refs on a suspended pass,
  level-init reset vs ordinary pipe/arena state retention, and hub->level reset.
  Preserve existing `$0C` section tests; separately expose the source timing
  mismatch rather than silently changing it during extraction.
- Every selector: assert before/at/after XH changes (raw X thresholds are
  `(XH<<8)-8` modulo65536), equal-S no-op, unrelated-section retention, reversal,
  repeated rendering no update, pause/scene ownership and BG/OBJ isolation.
- `$15`: explicitly assert XH5,YH2 with YL8/9/10/11/12, `$14/$15`, `$5C/$5D/$5E`,
  and YH1/2/3; XH9 unconditional; XH11 YL `$14/$15`. Tests must preserve the
  accumulator fallthrough, not just the apparent intended region.
- `$19`: Q less/equal/greater S, Q non-trigger still latches, exact triggers,
  skipped trigger, reversal, byte wrapping; assert all 12 BG+OBJ byte sources.
- `$03`: assert state0/1 shared across each BG/OBJ action and XH10 combined
  operation, plus only OBJ palette5 colors1/2 change in direct patch cases.
- `$07/$16`: assert exact little-endian RGB555 bytes and untouched neighboring
  colors; no BG/job update. Use both characters because tables are shared but
  mapping attributes determine which OBJ palettes are visible.
- `$0D`: S$10 distinction; YH0 rejection for XH0/1/5/6 but not8/11; XH7 no-op.
- `$18`: all three coordinate pairs and adjacent YH; correct whole OBJ source,
  unchanged BG; actual boss entry/phase source order before any precedence rule.

### Explicit remaining uncertainty

This audit establishes bounded static contracts, not independent pixel parity.
No emulator probe was used. Normal-frame order and direct stores prove stale
references on that path; enumerate special call paths (home `$22C4/$2304` also
call `$2D0A`) in the timing ticket before claiming all scene/death timing.
The direct-writer inventory plus known boot clear is not a proof against every
possible indirect write; no additional reset on pipe/arena entry was established.
Fade/pause jobs can overwrite actual hardware palettes after direct OBJ writes;
their ordering must be combined with the fade audit, preserving a distinction
between palette RAM, original BG buffer, palette source pointers and latch state.


## Character physics

### Confirmed changed native behavior

#### Friction: Mario 4, Luigi 3

DX home `$16CC..$1701` keeps the base gating and comparisons. Friction runs if
swimming, moon-physics bit 1, or grounded. It compares `hvel & $FC` to `$80`:
- equal: snap hvel to `$80` (so hvel `$80..$83` snap, no subtraction);
- below: wrapping-add amount;
- above: wrapping-subtract amount.

Only the two amount branches change: `$16E9` jumps to `$1A16`, `$16F2` jumps
`$1A21`, both calling `$19FF..$1A16`. That helper reads
`$A560 + level` (low-byte addition, not a u16 addition). Unless the value is
`$FF` it is the friction amount directly; otherwise amount is `4 - [$FFF6]`
with byte subtraction. Supported ordinary character values: Mario0 =>4,
Luigi1 =>3. The original dead bytes `$16EC..$16F2` / `$16F5..$16FB` still
contain base +/-4 operations but are bypassed by these jumps.

The live frame order remains jump/growth/vertical displacement (`$160A`,
`$1090`, `$14FA`), horizontal acceleration `$1701`, friction `$16CC`, then
horizontal displacement `$181C` (base home.asm around lines2102..2125).
Do not move Luigi friction after the displacement lookup or run it only when
no direction is held: it also changes net acceleration under held movement.

`$24:$7C7A..$7C89` copies 96 bytes `$7D00..$7D60` to `$A540..$A5A0`.
Title setup `$20:$6AD9..$6AE1` calls this routine through home `$191A`.
The three 32-byte tables are:
- `$24:$7D00` -> `$A540`: forced character: all `$FF` for this ROM.
- `$24:$7D20` -> `$A560`: friction override: all `$FF` for this ROM.
- `$24:$7D40` -> `$A580`: autoscroll increment: all `1` for this ROM.
The importer already carries bank `$24`; no extra bank is required for these.

Native missing point: `sml2-core/src/player_sim.rs` hardcodes +/-4 in its
`// $16CC friction` branch around line1943 and has no gameplay Character input.
Existing enum/settings select presentation. Preserve neutral snapping and gates.

#### Per-character displacement tables

Home `$1A39..$1A66` chooses vertical table pointers. `$FFF6 == 0` selects Mario,
any nonzero value selects Luigi. `$156B`, `$1574`, `$157D` replace the base
moon/normal/swim table loads with calls `$1A57`, `$1A39`, `$1A48` respectively.
Moon wins over swimming; swimming wins over normal. `$1587` calls `$1A66`, which
switches ROM bank1 to read the indexed byte and adds `$10` before the existing
signed-byte displacement integration; it restores the original ROM bank.

Home `$1A95..$1AA4` selects horizontal table: zero Mario `$4000`, nonzero Luigi
`$42E4`; `$1842` calls it. `$1851` calls `$1A86` to perform pointer+index and a
bank1 read, then restores prior ROM bank. Index is wrapping `hvel + (FF97 & 7)`
(the byte addition wraps, with D zero), not a new acceleration curve.

| Table | Base ROM home | DX Mario bank1 | DX Luigi bank1 | Exact comparison |
|---|---|---|---|---|
| H, 256 bytes | `$19FF` | `$4000` | `$42E4` | Mario identical. Luigi changes indices `$F7`:1->2, `$F8`:2->3 only |
| normal V,76 bytes | `$18AF` | `$4200` | `$44E4` | Mario identical. Luigi indices59..72 inclusive `$FD`->`$FC` only |
| swim V,76 bytes | `$18FB` | `$424C` | `$4530` | Both identical to base |
| moon V,76 bytes | `$1947` | `$4298` | `$457C` | Both identical to base |

Actual vertical table lengths are `$4C` (76), not native Rust arrays'82: native
arrays deliberately include following bytes, but those overreads cannot be
called equivalent table data after relocation. Normal jump states use <=75;
validate reachable indices rather than importing unrelated following opcodes.
Luigi normal V indices59..72 are stronger upward displacement (-4 vs -3).
The jump control flow/initial indices themselves are unchanged; do not assume
all jumps gain height. Those indices are reached by the high-jump route (initial
vvel75; e.g. Bunny, running outside the normal-speed band or Up-held), whereas
the initial48 route never reaches59..72. Jump-release cutoff is unchanged.
Swimming and moon vertical curves are shared; friction still differs there.

Horizontal integration's swim/moon-grounded branch still uses HOME `$1AFF`
(literal load at `$182C`) for BOTH characters; it does NOT use the bank1 table
immediately following Luigi's H table. The DX rewrite of preceding home space
changes only home `$1AFF` from base `$FF` to`$00`; `$1B00..$1BFF` remain identical.
That index0 is outside ordinary swim/moon velocity clamps but should be recorded
for byte-level completeness. Do not invent Luigi-specific water-H selection.

The bank1 `$43E4..$44E4` happens to duplicate base water-H; no audited movement
pointer selects it. Mario bank1 `$4100..$4200` contains other code/padding and
must not be imported as the second half of a 512-byte H curve.

H indices `$F7/$F8` should both have seeded boundary tests; ground run normally
caps at hvel243 before Luigi friction subtracts3, making maximum lookup247.
Airborne movement skips ground friction and can retain higher hvel, so do not
remove `$F8` as globally unreachable merely from the ground calculation.
Native `player_sim.rs` around1859..1870 and1955..1965 selects only native arrays;
`player/motion_tables.rs` contains no alternate DX tables.

### New source hook with unchanged supported-ROM behavior

Home `$0DD3` replaces base `ld a,1 / ld b,a` with `call $1A2C`.
`$1A2C..$1A39` reads `$A580 + level` as u16 addition into a 32-byte per-level
autoscroll table. The caller's even-frame gate, 12-bit camera wrap and edge
transition branches remain unchanged. `$24:$7D40..$7D60` is all1, so the
supported ROM remains one pixel per even frame, matching native `CameraSim`.
Record this hook as source difference/behavior unchanged; no custom autoscroll
behavior ticket is necessary for the exact supported digest. Test default values
if tables are extracted/validated as part of the gameplay content profile.

### Bounded unchanged proof

Direct base/DX byte comparisons show these entire ranges identical:
- home `$0F71..$13FE` (player update dispatcher and shared pipe/platform paths);
- `$13FE..$1501` (frame prep/push/swim clamps and animation movement adjuncts);
- `$150C..$156B` (jump release, falling/apex gates up to table-selection hook);
- `$1590..$16E9` (remaining vertical integration, bonk, jump and friction gate);
- `$1701..$1842` (horizontal acceleration, run/walk caps, goal-bell setup,
  integration prep up to pointer-selection hook);
- `$1854..$18AF` (horizontal displacement integration and wall outcome);
- `$1BFF..$1D72` (zone/swim/moon probing);
- `$31EF..$324F` (death-arc displacement table).
These establish unchanged source in these bounded paths, not whole-ROM gameplay
parity. Any other gameplay differences (spawns/scripts/geometry) are separate.
Powerup hooks `$31C8`, `$345C`, `$346C`, `$347E` now call bank1 `$461E`/`$463F`
to request graphics reload and theme bits; the ordinary powerup byte stores and
Fire SFX are retained. Character mapping `$465A` is presentation only. No
additional jump, death-curve or acceleration constants were found in this scope.

### Importer decision

Current schema8 imports bank0 loader tables/headers and42 full banks, omitting
bank1. It therefore lacks the DX movement curves. Add narrow validated physics
arrays for the actual table reads, rather than adding an entire raw code bank:
- Mario and Luigi horizontal256 and normal vertical76 arrays;
- shared swim/moon vertical76 arrays (verify equality at import, or retain both
  per-character if the intended source contract requires literal table identity);
- shared home water-H256 if exact home behavior including unreachable index0
  is required; native reuse needs an explicit equality/domain assertion.
- existing bank24 forced-character/friction/autoscroll table slices need no new
  raw bank. Only expose and validate the named data required by the profile.
A persisted new physics contract requires schema/importer/cache identity bump and
re-import/malformed-shape tests at implementation time, not during this audit.
Character/gates/friction arithmetic stay native Rust, not interpreted patch code.

### Bounded phase5 followups

1. `dx-gameplay-selection`: thread typed native gameplay mode+Character through
   runtime content/session/shared level-frame and diagnostic callers at safe
   level-entry boundary; no settings-only rendering call may mutate live physics.
   Keep native default and frozen replay behavior. Tests settings/door/reload.
2. `dx-physics-import`: narrow local-ROM arrays/table validation above; reject
   malformed lengths; new schema/cache/re-import round trip; exact source counts.
3. `dx-character-friction`: PlayerSim +/-amount helper at existing phase, test
   amount4/3, retained non-Friction airborne state, both sides, `$80..$83` snap,
   hvel wrapping, swimming/moon gates and held-input acceleration composition.
4. `dx-character-displacement`: native table selection with76-byte V bounds;
   tests normalV59/72 boundaries, high/low jump, release, H `$F6..$F9` and phases,
   swim/moon unchanged behavior, waterH index0 documented disposition, existing
   wall/platform/goal-bell paths. Byte tests before whole-character routes.
5. `dx-profile-regressions`: prove Mario's curves and default scroll tables
   equivalent to base, Luigi only audited differences, render-frequency and
   save/reload/mode identity unchanged, all native replay tests green.


## Fades and scene color transitions

### Important scope corrections

* Bank `$31:$4000..$40EB` is pause-only. `$40EA` is the last nonzero byte in
  bank `$31`. There is no additional fade engine in that bank.
* `$D300` is WHITE, not black: `$20:$6A66..$6A7E` fills 32 RGB555 entries with
  `$7FFF`. Consumers upload that same 64-byte buffer to both BG and OBJ ports.
* Castle-door entry fades to black through `$0C:$706B`. This is not the ordinary
  death fade. Its sole direct caller is `$0C:$46B2`, within castle menu state 3.
* The overworld exit fade has different cadence in DX: 36 explicit VBlank waits
  instead of the base's 48. Existing runtime transition logic assumes 48.
* Credits have additional authored palette AND attribute-map changes. Treating
  all ending substates as the initial DX colors misses three source hooks.
* Some base-game fade-looking state changes still write only DMG shadows in DX.
  They must not acquire a newly invented CGB fade.

### Palette transfer jobs and scheduling

Home VBlank `$0154` decrements the 16-bit fade timer `$A266/$A267` with
saturation at zero (`$0168..$0181`). Normal VBlank tail `$01CC..$01D3` reads
pending `$FFF8`; if nonzero it calls `$191E`, which selects bank `$24` and
jumps to `$7800`. This is one dispatch per eligible VBlank tail, not an event
that rendering may replay. Early interrupt exit paths need their existing source
ownership retained; do not unconditionally execute jobs on every host render.

`$24:$7800..$7912` snapshots the pending job to `$FFF9`, executes it, then clears
`$FFF8`. If queued `$FFF7` is nonzero it becomes the next `$FFF8`, and `$FFF7`
is cleared. Jobs are serial: a chained job is not run within the same dispatch.

| Job | Source branch | Operation |
|---|---|---|
| 1 | `$7806..$7810` | Upload BG from pointer `$A1DC:$A1DD` |
| 2 | `$7810..$781A` | Upload OBJ from pointer `$A1DE:$A1DF` |
| 3 | `$781A..$7827` | Upload both pointer sources |
| 4 | `$7827..$7848` | Set BG pointer to WRAM2 `$D200`, select WRAM bank2, upload BG and current OBJ pointer, restore bank0 |
| 5 | `$7848..$7866` | Set BG pointer to WRAM2 `$D200`, upload BG only, restore bank0 |
| 6 | `$7866..$7880` | Upload BG from WRAM2 `$D240`, directly entering home `$18B7`; restore bank0 |
| 7 | `$7880..$78BD` | Reload character graphics; not a palette fade |
| 8/9 | `$78BD..$7900` | General transfer operations; not fades |

Home `$18AF..$18DA` uploads exactly 64 BG bytes; `$18DA..$1905` uploads exactly
64 OBJ bytes. `$1948..$1952` selects ROM bank `$26` and auto-increment palette
index `$80`; RAM pointers still address the selected WRAM bank. Pointer bytes
are stored high first in the A1DC/A1DE fields. The typed implementation can
resolve sources directly, without emulating the job's register plumbing.

`$26:$7CD6..$7D17` initializes level BG source into WRAM2 `$D200` from
`$4000 + level*$40`, OBJ source at BG+$1000, requests job4 then queued job7.
This establishes source-load ordering relative to character graphics. Level
regional swaps and boss loads can replace BG sources and must remain separate
from pause's temporary output.

### Channel arithmetic contract

For RGB555 color `(r,g,b)`, channels are five bits. The routines work with
`r`, `2*g`, `4*b` in three separate scratch bytes. Their encoded deltas must
therefore be divided by 1,2,4 before describing normal RGB channels.
Addition saturates at `(31,31,31)`, subtraction at `(0,0,0)` independently.
Every operation handles all 32 colors (64 bytes), not just the visible palette.
All repacking writes little-endian RGB555 bytes. The differing operation/source
ownership below matters more than sharing the assembly implementation.

#### Pause — already implemented, retain regressions

Home `$2756..$2765` enters `$31:$4000`. `$31:$406A..$4084` chooses encoded
subtract deltas `(8,8,16)` when `$FF97` bit4 is clear, `(18,16,16)` when set:
normal RGB `(8,4,4)` / `(18,8,4)`. `$31:$4084..$40EB` always reads canonical
BG `$D200` and writes derived output `$D240`. It does not accumulate darkness.
It requests job6. OBJ palettes do not change. START unpause requests job5,
restoring BG `$D200`. SELECT's exit path and START precedence are visible in
`$31:$4019..$406A`; they must not be reduced to a renderer-only boolean.

Existing `render/dx.rs::pause_palettes` tests cover the two subtraction values.
Add future interaction tests for selected regional/boss BG, repeated renders,
and unpause during other transitions, rather than duplicating those literals.

#### Castle door / castle reveal visit — missing

Entry: castle visit `$0C:$467A -> $4693` dispatches menu state3 to `$469F`.
`$46E1..$4700` enters state3 only with all six coins and a relevant key edge,
setting fade low to `$40`. `$469F..$46D1` calls `$706B` at `$46B2`; on timer low
zero it changes to level `$18`, game state `$16`. This is Wario castle entry.

`$0C:$706B..$709C`:

1. If fade low == 0, jump to `$7000..$702B` and upload the all-zero ROM table
   `$702B..$706B` to both BG and OBJ immediately.
2. Otherwise if `(fade_low & $0F) != $0F`, return unchanged.
3. Otherwise choose encoded deltas `(16,8,16)` = normal RGB `(16,4,4)`.
   `$70B5` subtracts in place from canonical BG `$D200`, `$70A9` subtracts in
   place from OBJ `$D240`, using common arithmetic `$70BE..$7118`.
4. Point OBJ to `$D240`, request job4; it uploads both mutated buffers.

The source initializer `$27:$4480..$44DF` copies selected character OBJ into
`$D240`, BG into `$D200`; it is essential to start from those canonical values.
With initial low `$40`, source VBlank decrement yields effect steps at `$3F`,
`$2F`, `$1F`, `$0F`; zero forces complete black even if green/blue have not
reached zero by subtraction. High timer is not consulted by this helper.

Test proposal `dx_castle_door_palette_fade`: construct native castle transition
with each channel 0/1/31; assert exact bytes on 64->63, 48->47, 32->31, 16->15,
1->0, unchanged intermediate ticks, both character OBJ sources, and fresh visit
reset. Repeat renders must not repeat subtraction. Isolate black output from
`$D300` white blanking.

#### Overworld exits and returns — missing, timing differs

`$0F:$7700..$77A1` brightens IN PLACE both BG `$D200` and OBJ `$D240`, encoded
add `(10,20,40)` = normal RGB `(10,10,10)`, saturating independently. It requests
job4 and points OBJ to `$D240`. Four calls reach all-white; it does not derive
colors from DMG BGP/OBP shadows.

Callers and cadence (counts are explicit source waits, not emulator-lag claims):

| Entry | Sequence | Timing contract |
|---|---|---|
| `$0F:$69FB..$6A53` | Generic map exit: four brighten calls at `$6A02`, E=4..1 | DX waits once at `$69FF`, calls brighten, then eight waits `$6A0B..$6A23`: 9/stage, 36 total; base has 12/stage, 48 total |
| `$0F:$62F7..$6367` | Zone walk-out: first 40*2 walk waits, then four brighten calls at `$631D` | 80 walk waits; each fade stage has one wait then 2*(2 waits), 5/stage, 20 fade waits; native exit sequence totals100 |
| `$0F:$6A9A..$6AD7` | Hub return/unblank: calls `$77A1` with E=1..4 at `$6ABA` | Three waits before each palette step: 12 explicit waits total |

`$77A1..$7814` implements the return fade. On E=1 it copies the original
128 bytes `$D200..$D27F` to backup `$D180..$D1FF`, then brightens by RGB +30.
On E=2/3 it RESTORES that backup first, then brightens by +20/+10. On E=4 it
restores the backup unchanged. Thus deltas are `(30,30,30)`, `(20,20,20)`,
`(10,10,10)`, `(0,0,0)`, derived afresh, not cumulative subtraction from white.
Unknown E falls into the restore-only branch, as the source does.

`$0F:$78AD..$78E5` first uploads white `$D300` to BG and OBJ then calls the
return fade `$6A9A`. Direct caller `$4528` is the completed return path after a
hub interlude. `$0F:$7873..$78AD` also whites both sets and requests job7;
caller `$6F2D` is the level-to-map handoff/minigame decision path. It is a blank
load boundary, not a color interpolation operation.

Important unchanged paths: `$0F:$63BA..$6426` walk-IN and `$6A68..$6A9A` legacy
fade-in are byte-identical to base and write DMG shadows only. They do not call
these CGB color arithmetic helpers. Reusing native grayscale fades indiscriminately
would add effects the DX ROM does not perform.

Test proposals: `dx_map_exit_palette_steps_and_36_waits`,
`dx_zone_walkout_palette_steps`, `dx_map_return_restores_original_palettes`.
Pin per-step bytes and timing independently of existing vanilla 48-frame tests.
No mode-wide change to existing vanilla timing is authorized by these findings.

#### Credits authored scene changes — missing

Initial ending loader `$29:$6718` uses BG `$6769`, OBJ `$67A9`, attributes
`$5200`. The current `render/dx_ending.rs` unconditionally uses those sources
for every loaded substate, so these verified later changes are missing:

| Transition | Hook / target | BG | OBJ | Attribute effect |
|---|---|---|---|---|
| 7->8 | `$1A:$43F8 -> $5E00 -> $5E6C` | bank1A `$5EC1`,64 bytes | `$5F81`,64 | Copy `$5FC1` for `$2A0` bytes to VRAM1 `$9800` |
| 8->9 | `$1A:$4187 -> $5E10` | bank1A `$5F01`,64 | `$5F81`,64 | Retain previously loaded attributes |
| 9->10 | `$1A:$4145 -> $5E3E` | bank1A `$5F41`,64 | `$5F81`,64 | Retain previously loaded attributes |

Dispatch `$1A:$4016..$4042` is indexed by ending substate `$A2DD`. The helpers
store the incremented substate before applying color transfers. The first hook
requires character Y to reach `$8B` under its existing parity-controlled movement;
the following two are gated by `$A248` transfer completion. They are authored
whole-set changes, not channel fades and not elapsed-time estimates. Retain
selection through later substates until a source load replaces it.

Test proposal `dx_ending_substate_palette_and_attribute_transfers`: use source-
derived substate/transfer setup, assert source palette bytes, selected pixels and
attributes beyond original `$240` coverage; verify exact transition frames and
retention into subsequent substates. Keep each transition in its own ticket if
module scope requires it. Run with both characters and verify repeated drawing
is read-only.

### Blank/load entries and unchanged DMG-only effects

These source-load paths must appear as separate matrix rows/transition cases:

* Title `$20:$6A66` initializes white `$D300`; `$6A8A..$6AA3` requests initial
  title BG+OBJ via job3. It does not implement a generic channel fade.
* Coin minigame variants `$26:$765A..$77EC` upload white `$D300` to BG+OBJ before
  their selected attributes and room palettes. Bell variant `$78F0..$7969` also
  whites then loads its own colors; the `$7C91` bell path loads its selected
  attributes/colors directly. Do not force both entry routes through a guessed
  fade curve.
* Boss arena reload hook home `$25C9 -> $36:$7746..$7786` whites BG+OBJ from
  `$D300`, then performs the source transfer and returns to home `$25D2`.
* `home $03AF..$03F1` ending-entry fade and bank1 `$6442..$647D` legacy level
  fade are byte-identical to base. They change only DMG palette shadows.
  DX VBlank no longer uploads those shadows to palette hardware. Preserve their
  state/timer/scene handoff effects without recoloring the DX frame.
* `$0C:$7118..$7158` is a separate 4-of-32-frame OBJ palette-entry flash for the
  coin/reveal display (normal words `$1A7A,$0085`, flash `$0B3F,$06B8` at index
  `$9C`). It replaces a DMG flash at the `$445B` continuation and needs its own
  effect test; generic fade handling will not reproduce it.

This inventory accounts for the actual fade arithmetic and direct white/black
boundaries above. It does not certify unrelated palette-entry effects or timing
on real hardware; those need their dedicated rows in the full verification matrix.

### Importer decision and bounded tickets

No new raw bank is required for the effects in this audit. All palette data and
attribute data are in currently imported banks `$0C,$0F,$1A,$20,$24,$26,$27,$29,$36`.
The pause routine in omitted bank `$31` is native arithmetic with immediate
constants already implemented; importing its code adds no needed data.
No schema bump is justified by these fade findings alone. If the chosen typed
API exposes cached derived palette tables, prefer deriving from existing banks;
a persisted representation change would independently require version/tests.

Suggested tickets, each source-bounded with a separate meaningful regression:

1. Palette source and job chaining contract: `$24:$7800..$7912`, home uploaders;
   shared typed simulation state, no renderer mutation. Needed by fades below.
2. Map brighten primitive + separate 36-wait exit integration and vanilla
   preservation; source `$0F:$69FB..$6A53`, `$7700..$77A1`.
3. Map original-palette backup/restore fade + white entry integration;
   `$6A9A..$6AD7`, `$77A1..$78E5`.
4. Zone walk-out integration, `$62F7..$6367`; keep walk-in DMG-only distinction.
5. Castle-door black fade, `$0C:$469F..$4700`, `$7000..$7118`.
6. Boss/minigame white boundaries; split by scene family/caller.
7. Credits substate palette/attribute transitions listed above.
8. Castle coin/reveal palette-entry flash, `$0C:$7118..$7158`.

Future code tests should be attached to the relevant focused native effect
module and scene-owner integration tests. Expected bytes must be independently
calculated from these ROM-derived contracts. Import/export and source-data tests
may remain opt-in against `SML2_DX_ROM`; no ROM fixture or extracted artwork is
needed in Git. The phase-1 deliverable itself changes no runtime behavior.
