# Menus and save flow — completion plan

Planning document, 2026-10-19. This is the last whole row of
[the completion workflow](2026-10-15-rust-completion-workflow.md) milestone D
still marked `pending` in [the feature inventory](../feature-inventory.md):
original boot → title → file select → save management. Pause, game over and
the ending are already implemented and are only glue here.

Implement each ticket as a bounded, independently verified step. No new
`captures/traj*.json`; pin behavior with unit tests and typed live-play tapes,
run `captures/sweep.sh` once per work package, and do not change the frozen
`traj4_ff0` baseline.

## Status

- [x] Source inventory measured (state table, save layout, menu handlers)
- [ ] Decision D1–D4 confirmed
- [x] FE-1 save-slot core
- [x] FE-2 front-end assets
- [x] FE-3 title screen
- [x] FE-4 file select (state machine + OAM rendering)
- [x] FE-5 session/runtime integration + rendering + `--boot title`
- [x] FE-6 desktop launcher + persistence
- [ ] FE-7 attract demos (can ship after FE-4)
- [ ] FE-8 acceptance, evidence, inventory row

## Definition of done

From the completion workflow: the native game starts at the title screen,
manages save files, and reaches the overworld through the original flow.

Concretely:

1. Power-on with an empty or valid SRAM image shows the source title screen at
   the source palettes and cadence; START opens the file select.
2. The file select shows three slots from the source tilemap and sprite layout,
   with each slot's tally read from its record (the ROM draws only the tally
   digits; lives are not in the slot row).
3. New game, continue, erase, cancel, and the Easy-mode toggle behave as the
   ROM does, with the source's timers, SFX and music requests.
4. Loading a slot copies its 64-byte progress block and counters into the live
   session, enters the overworld through the map hand-off, and a later save
   writes back to the same slot.
5. Relaunching and loading a slot restores node, screen, progress, coins,
   tally, lives and money bags; the other two slots are untouched.
6. A slot with a bad checksum is reinitialized individually at file-select
   entry.
7. The desktop launcher remains a shell around this flow (D1); the existing
   `--play`/`--replay` acceptance paths keep their behavior.
8. Evidence: unit tests for every produced byte, typed live-play tapes for the
   scenarios below, `cargo test --workspace`, frozen sweep green, and the
   `menus-save-flow` inventory row moved to verified.

## Measured source map

All addresses are ROM addresses; banked labels are shown as the disassembly
names them (`$30000`-style labels are bank 12 ROMX `$4000+`).

### Game states (`$FF9B` jump table at `home.asm:$02B0`)

| State | Handler | Role |
|---|---:|---|
| `$00` | `$2CC4` → bank 5 `$4043` | title loader: `GFX_TitleScreen` (`$41E5`, `$1800` bytes) to `$8000`, tilemap `$59E5` to `$9800`, scroll/palette, music `$FF`, fade `5/$40`, state → `$01` |
| `$01` | `$2CDB` → bank 5 `$407C` | title idle: OAM clear (`$14113`), fade tick, START (`$08`) → state `$19`; the fade timer reaching zero (5×256+64 frames) starts an attract demo (state `$0B`) |
| `$19` | `$2BE5` → bank 12 `$4000` | file-select loader: `$28D9` validates/initializes all three slots, copies the file-select gfx/tilemap (`$2B8B`, tilemap `$4A52` → `$9800`), palettes `$E4/$D0/$38`, SFX 0 = 5, music 13, `sMenuState = 0`, `sEasyMode = 0`, state → `$1A` |
| `$1A` | `$2BF0` → bank 12 `$405F` | file-select loop: cursor OAM, slot tallies via `$2B3B`, menu dispatch |
| `$1B`/`$1D` | `$2AD3` → bank 12 `$45B6` | castle/golden-coin screen (ending chain, out of scope) |
| `$1C` | `$2ADE` → bank 12 `$467A` | same chain; ends at level 24 + state `$16` |
| `$1E`/`$1F` | `$2AE9`/`$2AF4` → bank 12 `$441D`/`$4451` | same chain |
| `$20`/`$21` | `$2A80`/`$2A8B` → bank 12 `$477B`/`$47A9` | all-golden-coins screen → soft reset (out of scope) |
| `$22` | `$2C98` → bank 26 `$68528` | credits scroll (already ported) |
| `$23` | `$03AF` | ending fade (already ported) |

Only `$00`, `$01`, `$19`, `$1A` are this plan's state work. `$1B`–`$21`
belong to the ending/golden-coin chain and must not be pulled in.

### File-select menu (`sMenuState`, `$A277`; jump table `$30132`)

| `sMenuState` | Handler (bank 12) | Role |
|---:|---:|---|
| 0 | `$43D6` | cursor drops from above into the slot row (`$FFC0` → 124); state → 1 |
| 1 | `$4326` | input: SELECT (`$04`) toggles Easy mode (`sEasyMode ^= 1`, sprite `$FFC6`); RIGHT/LEFT (`$10`/`$20`) move the cursor (states 2/3); DOWN (`$80`) or START (`$08`) confirms when the cursor is on a slot (X < `$80`); A (`$01`) at the far-right position (`$FFC2 == $88`) enters erase mode (4) |
| 2 | `$42F1` | cursor right one slot (`$FFC2 += 1` per frame), timer 32, state → 1 |
| 3 | `$42C8` | cursor left one slot (`$FFC2 -= 1` per frame), timer 32, state → 1 |
| 4 | `$426D` | erase-mode toggle at timer `$2B`: `sFileClearMode ^= 1`, `sMarioDirection = $FF`, SFX 0 = 8; clearing Easy mode when erase is on; state → 1 |
| 5 | `$41C6` | confirm animation (timer 64); at zero: index = `(($FFC2 & $60) >> 5) - 1`; clear mode → erase path, otherwise load path |
| 6 | `$4140` | post-erase animation (timer `$80`), then cursor Y → 124 and state → 1, clear mode cleared |

Slot display (`$300FE`): two digit sprites per slot at X = 88/96 and Y =
32/64/96, built from `sSaveSlotNTally` (`$5267`: high nibble, then low nibble,
tile + `$80`). Lives are not drawn on this screen.

Menu work RAM: `sMenuState $A277`, `sMenuTimer $A278`, `sSaveFileIndex $A2A3`,
`sFileClearMode $A2C6`, `sCursorHopActive $A2C7`, `sCursorAnimY $A2D1`,
`sEasyMode $A2E4`. Cursor position lives in the OAM shadow bytes
`$FFC0/$FFC2` used by the existing sprite path.

Load/erase body (`$30227`):

- **Load**: copy `0x40` bytes from `$A000 + index*$50` to `$A840` (the map
  save-record image), then `+$40` coin lo, `+$41` coin hi, `+$42` tally,
  `+$43` lives, `+$44` money bags into the live globals; `sMarioDead = 21`;
  state `$15`. `$353E` clears `sCurPowerup` and sets state `$14`, which runs
  the existing map hand-off (`$2C23` → bank 15 `$3EF2B`).
- **Erase**: zero the 64-byte progress block, write coins `0/0`, tally `0`,
  lives `5`, money bags `0`, checksum `2`, signature `$12 $34 $56 $78`;
  `sMenuTimer = $80`, `sMenuState = 6`, `sFileClearMode = 0`,
  `sCursorAnimY = 144`, SFX 2 = 5.

### Save slots

Battery RAM `$A000-$A0AF`: three 80-byte records at `$A000`, `$A050`, `$A0A0`.

| Offset | Field |
|---:|---|
| `+$00..$3F` | 64-byte progress block (copy of the map save image `$A840`) |
| `+$40` | coin low |
| `+$41` | coin high |
| `+$42` | level tally (BCD) |
| `+$43` | lives |
| `+$44` | money bags |
| `+$45` | checksum = sum of bytes `0..=$44` |
| `+$46..$49` | signature `$12 $34 $56 $78` (`+$4A` is also `$78` but unvalidated) |

- `$28D9` validates all three slots (called at file-select entry).
- `$28EC` validates one: sums 69 bytes (`0..$44`), compares `+$45`, then checks
  `+$46..+$49` against `$12 $34 $56 $78`. Invalid → initialize.
- `$2934` writes the active slot (`sSaveFileIndex`) from `$A840` plus counters
  and checksum. Existing triggers: map entry/exit, game over, castle.
- **Measured quirk to pin**: the fresh-slot template writes lives `5` but
  checksum `2`, although the sum of the written bytes is `5`. The template is
  therefore invalid by the ROM's own validator; a slot stays "empty" until a
  real save writes a matching checksum. Tests must assert this exact template,
  not a "corrected" one.
- `save_record_valid` in `crates/sml2-core/src/map.rs` already mirrors
  `$28EC`; `MapSim::save_record`/`apply_save_record` already produce/consume
  the `0x50` record.

### Assets

- Title: `gfx/titlescreen.2bpp` (bank 5 `$41E5`), tilemap `$59E5` → `$9800`
  (`0x400`), and a 32-entry `dw` table at `$4165` with a command payload (the
  disassembly records no direct reader; confirm whether the title animation
  uses it before porting it); palettes
  `$00/$00/$00`, `$50/$40/$14`, `$94/$90/$24`, `$E4/$D0/$38`.
- File select: `$2B8B` copies bank 7 `$5E00` → `$9200` (`0x600`), bank 7
  `$6A00` → `$8E80` (`0x380`), `GFX_Mario` → `$8000` (`0x800`), bank 27
  `$7000` → `$8800` (`0x300`), bank 12 `$6ACA` → `$8B00` (`0x380`); tilemap
  `$4A52` → `$9800` (`0x400`); palettes `$E4/$D0/$38`.
- Cursor/digits are OAM entries built by `$2B3B`/`$2B63`/`$2B13` (`$5267`,
  `$5272`, `$52E7`), i.e. the existing sprite-staging path; digits are the low
  and high nibbles + `$80`.

## Port touchpoints (measured)

| Port surface | Today | Change |
|---|---|---|
| `SessionState` (`session.rs`) | `Overworld`, `InLevel`, `GameOver` | add `Title`, `FileSelect` |
| `Session::advance` | map/level/game-over dispatch | front-end dispatch + input edges; no `prev_*` reuse across states |
| `Session::save_record` / `apply_save_record` | one `0x50` record | active-slot read/write through `SaveSlots` |
| `render/session.rs::render_frame` | map/level/game-over | title/file-select static BG + OAM |
| `content.rs::SessionAssets` | gameplay bundles | `FrontEndAssets` (title, file select) |
| `play.rs::run_play` / `replay.rs` | direct level boots | unchanged; `--boot title` is a separate entry |
| `sml2-desktop` `Game::new`/`tick` | one `.sav` record | 3-slot image + launcher entry (D1/D2) |

## Design

### D1 — how the original flow is reached

Recommendation: the desktop launcher keeps its present entries and gains
**Original Boot**; **Play/Resume** keeps today's quick-load behavior during the
migration and is switched to Original Boot at FE-8 only if the user prefers it.
The runtime gets a CLI `--boot title` (also used by tests).

Rationale: the 2026-10-19 desktop plan says to keep the original title/file
select distinct from the launcher. A CLI flag also lets the acceptance tapes
start at the title without touching `--play`.

### D2 — persistence format

Recommendation: one `.sav` file holding a 3-slot SRAM image
(`0x10`-byte header + `3 × 0x50`). Header: magic/version/active-slot/last-used.
A legacy single-record `0x50` file is migrated into slot 0. Unknown/corrupt
header → treat as empty SRAM and let `$28D9` reinitialize. Atomic replacement
stays in `settings::atomic_write`.

### D3 — attract demos

Recommendation: in scope, but as FE-7 after the file select works. Four demos
(`sDemoIndex` 0–3, levels `$4121` = `0/$11/$0E/$14`, input pages
`$7F00 - idx*$100` → `$A300` — index 0 reads `$7F00`, index 3 `$7C00` —
`sSoundDisabled` 15/240) run from the title after
the fade timer expires and return to the title on demo end. START interrupts.

### D4 — Easy mode

Recommendation: model `sEasyMode` as a session field toggled only on the file
select (SELECT) and cleared when erase mode engages. It is not part of the save
record and is not persisted. Its level-start consumer (`$3573`) is already
modelled only if `sEasyMode` is read there; confirm during FE-5 and leave any
visual easy-mode asset difference out of scope unless it changes gameplay.

## Work breakdown

Each ticket: one bounded change, tests pin produced bytes, sweep once after the
batch, commit only with a green gate.

### FE-1 — Save-slot core (M)

**Files**: new `crates/sml2-core/src/front_end/save_slots.rs`, `front_end/mod.rs`;
reuse `map::save_record_valid`.

**Deliverable**: `SaveSlots` (three `[u8; 0x50]`), `validate_and_init()`
(`$28D9`/`$28EC`), `write_slot(index, &record)`, `load_slot(index) -> Option<Record>`,
`erase_template()`. `Record` view with named offsets; exact checksum/signature
rules; index bounds.

**Tests**: fresh template byte-for-byte (lives 5, checksum 2, signature);
valid record round-trip; single-byte corruption reinitializes only that slot;
out-of-range index is an error (not a panic); a real `save_record()` validates.

**Exit**: no UI; core tests; no runtime behavior change.

### FE-2 — Front-end assets (M)

**Files**: `crates/sml2-extract` (title/file-select outputs), `sml2-runtime/src/content.rs`
(`SessionAssets` bundles), extractor tests.

**Deliverable**: extracted title gfx/tilemap/script, file-select gfx/tilemap,
palette bytes and sprite tile offsets, matching the `extracted/` layout; a
`FrontEndAssets` load path with size checks.

**Tests**: asset lengths match the source constants (`0x1800`, `0x400`, `0x600`,
`0x380`, `0x800`, `0x300`); a fresh extraction preserves all existing assets
(extractor golden test already exists).

**Exit**: no behavior change; assets available to core/runtime.

### FE-3 — Title screen (M/L)

**Files**: `crates/sml2-core/src/front_end/title.rs`, runtime presentation glue.

**Deliverable**: `TitleScreen` with the `$00` loader state (tilemap/palette/
scroll/fade `5/$40`, music `$FF`, OAM clear), the `$01` idle tick (fade
countdown, START → `FileSelect`), and the `$4165` table/payload only if
measurement shows a reader for it (the disassembly lists none). Emits typed actions
(`OpenFileSelect`, `StartDemo(index)`), never runtime types.

**Tests**: frame-exact fade and START edge (A/UP do not open); tilemap bytes
loaded from the asset at `$9800`; any title sprite script produces the
pinned cells for a fixed frame; music request `$FF` on entry.

**Exit**: a headless tape renders the title for N frames and matches the
pinned tilemap/OAM; no session integration yet.

### FE-4 — File select (L; split 4a/4b/4c)

**4a init + render (M)**: `$19` loader: `$28D9`, gfx copies, tilemap `$4A52`,
palettes, cursor seed `$FFC0=16/$FFC2=40`, SFX 0 = 5, music 13, `sEasyMode=0`,
`sMenuState=0`, `$FF9B=26`. Render static BG + OAM (three slot rows, digits).

**4b menu state machine (L)**: all seven `sMenuState` handlers with exact
timers (32/55/64/`$2B`/`$80`), cursor movement, Easy toggle, erase toggle,
confirm/cancel, SFX 0/2 requests, digit sprites from slot tallies/lives.

**4c load/erase (M)**: `$30227` index derivation, 64-byte copy to the record,
counter loads, `sMarioDead=21`, state `$15`; erase template + `sMenuState=6`.

**Tests (4a–4c)**: per-state byte tests; SELECT toggles Easy only when not in
erase mode; A at the cursor's far-right position (`$FFC2 == $88`) enters erase
mode and clears Easy; START or DOWN on slot 0/1/2 loads the right record;
erase writes the exact template; the loaded counters
match the slot; cursor sprite ids match `$3012A`.

**Exit**: core-only file-select tape exercises every state, timer and commit;
the OAM/tilemap cells are pinned by the FE-5 renderer tests. Nine tests cover
the 45-frame cursor drop, both 32-frame walks, Easy/erase toggles, confirm
blocking at the erase position, load, erase and the even-frame SFX 2 tick.

### FE-5 — Session/runtime integration (M)

**Files**: `crates/sml2-runtime/src/session.rs`, `render/session.rs`,
`content.rs`, `play.rs`/`replay.rs` if their frame heads need the new state.

**Deliverable**: `SessionState::Title(TitleScreen)`, `SessionState::FileSelect(FileSelect)`;
`Session::advance` dispatch with input edges and `Divider`; sound requests flow
to the driver (SFX 0/2, music `$FF`/13); `SessionState::Overworld` entry from a
loaded slot; `save_record` writes the active slot index; render path for the
front-end (static tilemap + OAM); `--boot title` CLI; keep `--play`/`--replay`
behavior.

**Tests**: typed live tape power-on → title → START → select slot → load →
overworld; title music stop + file-select music; `--play` unchanged; save
during overworld writes the selected slot only.

**Exit**: end-to-end headless run with the existing session tests; no capture
changes.

**Status**: done. Measured corrections while rendering: the file select's
background resolves tiles through the signed `$8800` method (the A/B/C letters
live at `$9000 + tile*16`; a `$8000` lookup renders the Mario tiles), the cursor
is the `$5297` player metasprite (16x32 frame 0..23; Easy mode adds 32 and draws
the 6-cell small frame), the tally digits are the `$5267` two-cell row, and the
Easy banner is frame 241 at OAM (68, 48). The runtime keeps `SaveSlots`, marks
the loaded slot active, and writes in-level saves into it.

### FE-6 — Desktop launcher + persistence (S/M)

**Files**: `crates/sml2-desktop/src/app/*`, `settings.rs`, `game.rs`.

**Deliverable**: launcher entry (D1), save path migration (D2), display of the
active slot, `--no-audio` unaffected; save file written atomically.

**Tests**: settings/persistence unit tests; manual smoke on Linux and Windows.

**Exit**: launcher opens the original flow; legacy save loads into slot 0;
relaunch restores the selected slot.

**Status**: done. The Home launcher keeps Play/Resume and adds `Title / File
Select`; `--title` starts the desktop on `$00`. `saves/default.sav` is now the
`$F0` three-pipe image (atomic write through the existing temp-and-rename
helper); a legacy `$50` record migrates into pipe A and is rewritten
immediately, an unknown length is reported and left alone. The Home panel lists
each pipe's BCD tally, and the debug panel shows the active slot. Five Game
tests cover migration, first-valid-slot resume, the wrong-length case and the
full-image write; the desktop smoke screenshot confirms the title frame.

### FE-7 — Attract demos (M, after FE-4)

**Deliverable**: `sDemoIndex`, level table `$4121`, input pages
`$7F00 - idx*$100` copied to `$A300`, run-length input decoding (`(keys, count)`
pairs terminated by `$FF`, `$38E5`), `sSoundDisabled`, demo end cycling
(`$38CE`: index + 1 & 3, sound stop, `Init` → title), START behavior. Typed
demo tapes; no live input may leak into the demo.

**Exit**: each of the four demos runs N frames deterministically and returns to
the title; START interrupts.

### FE-8 — Acceptance, evidence, inventory (S)

- Typed tapes for the eight scenarios below, run through the live session.
- `cargo test --workspace`, `bash captures/sweep.sh` with `traj4_ff0` baseline
  unchanged.
- Desktop screenshot of title/file-select and a manual save/reload pass.
- Update `docs/feature-inventory.md` `menus-save-flow` row with evidence and
  normal-play path; note any residual gap (e.g., easy-mode visuals).
- Update `PORT_QUEUE.json` only if new gaps are discovered; otherwise record
  the completed row in the archive per the existing convention.

## Acceptance scenarios

Each is a typed input tape with fixed frame counts and a byte-level final
assertion (no new captures).

1. **Empty SRAM, new game**: title (N frames) → START → file select → slot 0 →
   START → load → overworld node 0/screen 0/lives 5; progress block zero.
2. **Continue**: pre-seed slot 2 with a valid record (node/progress/coins/tally/
   lives/money bags) → title → file select shows its tally/lives → load →
   overworld state matches the record; slots 0/1 untouched.
3. **Save round-trip**: load slot 2, change state in the overworld (enter a
   level, return), save → relaunch → slot 2 reflects the new record.
4. **Erase**: move the cursor to the far-right erase position (`$FFC2 == $88`)
   → A enters erase mode → wait for the state-4 toggle → move to slot 2 →
   START → confirm → template bytes; load shows an empty slot; SFX 0 = 8,
   SFX 2 = 5 observed.
5. **Confirm timing**: START or DOWN arms state 5; the commit lands exactly
   64 frames later, and no other button cancels it (the ROM has no cancel
   branch in states 1 or 5; B does nothing).
6. **Corrupt slot**: flip one checksum byte in slot 1 → file-select entry
   reinitializes only slot 1; slots 0/2 stay valid.
7. **Easy toggle**: SELECT on the file select flips `sEasyMode` and the sprite id;
   erase mode clears it; it does not persist across a reload.
8. **Attract**: idle title past the fade → demo level loads and returns; START
   during the demo interrupts back to the title.

## Verification and gates

- Unit tests: `cargo test -p sml2-core`, `-p sml2-runtime`, `--workspace`.
- Frozen corpus: `bash captures/sweep.sh` (`traj4_ff0` must equal its legacy
  baseline; every other capture zero mismatches).
- Extraction: fresh `sml2-extract` run preserves the existing asset set and the
  new title/file-select outputs match the ROM slices.
- Manual: Linux/Wayland and Windows (WASAPI) smoke; save/reload across all
  three slots; erase; corrupt slot.
- No queue ticket may be marked done without the tests above; an empty queue is
  not a completion claim.

## Risks / open questions

1. **Fresh-template checksum 2 vs sum 5** — confirm on an emulator that all
   slots start invalid and show as empty; pin the exact template in FE-1.
2. **Index derivation** — `((($FFC2 & $60) >> 5) - 1)` maps the cursor X to a
   slot; confirm the X bands (`0x20`/`0x40`/`0x60`, cursor `0x28`/`0x48`/`0x68`,
   erase position `0x88`) with a tape.
3. **Title OAM script** — the `$4165` table's command payload is not yet
   decoded; budget one measurement ticket before FE-3 if the title animation
   must be exact. A static title (no animation) is a fallback, but then the
   inventory row must state the residual gap.
4. **Easy mode** — where the ROM sets it outside the file select is not yet
   audited; do not claim easy-mode gameplay parity until measured.
5. **File-select rendering** — digits are sprites (`$2B3B`); confirm the existing
   sprite path can stage 40+ OAM entries and the static BG without a new
   renderer seam.
6. **Legacy save** — the current single `.sav` is a 0x50 record; migration must
   not lose an in-progress game.
7. **State ownership** — `SessionState` currently has three variants; adding
   two affects every match arm (advance, render, save_record). Keep the
   front-end state in core and the runtime glue thin to avoid replay drift.

## Out of scope

- Bonus game / coin game (`gfx/bonusgame.2bpp`, `gfx/coingame.2bpp`) — the
  separate `minigames-ending` row.
- Golden-coin congratulations (`$20/$21`) and the castle-state chain
  (`$1B`–`$1F`); the ending/credits are already ported.
- Desktop settings/rebinding/fullscreen features beyond the launcher entry.
- New captures, texture packs, frame interpolation.

## Copy-paste prompt for the next agent

> Read `docs/plans/2026-10-19-menus-save-flow.md` and `docs/feature-inventory.md`
> row `menus-save-flow`; read `PORTING.md` for the "done" standard. Start with
> FE-1 (save-slot core): implement `SaveSlots` in `crates/sml2-core/src/front_end/`,
> mirroring `$28D9`/`$28EC`/`$2934`, including the measured fresh-template
> checksum `2` (invalid by design), and pin it with byte tests. Do not touch the
> runtime or assets yet. Run `cargo test -p sml2-core`, commit only with a
> green gate, then report the test names and the next ticket. Do not add
> captures; do not change `traj4_ff0`.
