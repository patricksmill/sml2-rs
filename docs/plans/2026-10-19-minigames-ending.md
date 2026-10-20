# Minigames, final castle and ending — measured plan

Status: **ending/credits and the final castle verified; the bonus minigames are not
ported**. The `minigames-ending` inventory row stays `pending` until ME-2..ME-7
land. This plan is based on ROM measurement on 2026-10-19, not on the old
"unaudited" row.

## What is already done

| Area | Evidence |
|---|---|
| Ending/credits | `ending_entry`, `ending_sequence`, `ending_scene`, `ending_sprites`, `ending_raster`, `credits_text`; 43 core + 4 runtime tests (`cargo test -p sml2-core --lib ending`, `-p sml2-runtime --lib ending`); Wario ending and castle fanfare covered by `player/tests/boss_clear.rs` |
| Golden coins | `PlayerSim::golden_coin_flags/count`, six-coin fanfare request (track 26), boss-clear reward tests |
| Final castle | level id 24 boots headless with zero unported handlers (`--level 24 --headless 120`) |
| Castle screen | `MapSim::load_screen_castle` (screen 8), castle-slot node gating, save record |
| Coin doors | screens 9-12 tilemaps extracted (`bank 15 $43240/$43480/$436C0/$43900`), prices `$0999/$0200/$0050/$001E` and loaders modelled in `MapSim` |
| Coin icons | hub/zone coin icons follow the six golden-coin flags; renderer tests |

## What is missing

The bonus rooms are **bank 24 map scripts**, not object code. The port has the
node walk streams but no general map-script VM.

- Coin-door rooms (screens 9-12): per-frame handler `$3F4A0` far-jumps to
  `$4A81` (bank 24); `sCastleState == 3` returns to the castle screen 8. The
  scripts load sprites with `$3E2A2` (`$66D4/$671C/...`) and stage Mario with
  `$3F35A` at OAM `(40,156)`.
- Bell rooms (screens 13/14): loaders `$3F4B1` / `$3F5A4`; tilemaps at bank 17
  `$7800` / `$7A40` (`0x400` each, copied to `$9800`); scripts
  `$4A1A/$4A2F/$4A58` (chosen by `sTimerHigh & 0x0F`) and `$6761`; six castle
  slots driven by `sCastleState`, `sCastleSlotCounter/Delay/Index`,
  `sMapPatchList`; SFX 5, music 21.
- Golden-coin/castle chain: game states `$1B`/`$1D` (bank 12 `$45B6`), `$1C`
  (`$467A`), `$1E` (`$441D`), `$1F` (`$4451`), `$20` (`$477B`), `$21` (`$47A9`).
  None are in the runtime state machine.
- The bank-24 script engine used by these rooms (`$3E00` far-jump, then the
  script at the entry address) has no port. The relevant entry points are
  `$4A81` (coin rooms), `$4512`/`$4028` (castle/bell screens) and the data
  tables at `$4A1A`/`$4A2F`/`$4A58`/`$6761`.

## Tickets

### ME-1 — Audit and plan (done)
Read the state table, the map-screen dispatch, the loaders, the bank-24 entry
points, the existing ending/castle code. Verified ending/credits and level 24
as above. This document.

### ME-2 — Extract the bell rooms and script blobs (done)
Screens 13/14 are in `SCREEN_TILEMAP_SPECS` and `map/tilemaps.bin` is 18,432
bytes (14 screens + the hub). **Corrected**: the two room pointers are decimal
bank 17 ($11), `$47800`/`$47A40`; the first cut used `$5F800`/`$5FA40`, which is
bank 23 (`$17` read as hex) and renders as packed data. Bank 25 `$5000` (256
tiles) is emitted as `map/bell_room_tiles.bin` and both screens render to
`map/bell_bonus_a.png`/`_b.png`; neither loader writes `sBGPalette`, so both
keep the $E4 of screen 8.

`map/castle_scripts.bin` (+ `.json`) carries eight measured blobs (4,028
bytes): `bell_room_b_script` $4028 (1,258 B), `bell_room_a_script` $4512 (1,288 B),
`bell_slots_0/1/2` $4A1A/$4A2F/$4A58 (21/41/41 B, `$FF`-terminated),
`coin_room_script` $4A81 (1,241 B) and the bank-15 sprite/patch lists
`bell_room_a_sprites` $6734 (45 B) and `bell_room_b_sprites` $6761 (93 B).
Each code end is a `ret`; `bell_room_b_script`/`bell_room_a_script` end where the
next entry starts, `coin_room_script` at the $4F5A lookup tables. The sprite
lists are `$3E2A2`'s two 3-byte record runs, each closed by an `$FF` record.
`CastleScripts` and `BellRoomTiles` in `sml2-core` pin the emitted bytes.

### ME-3 — Port the bank-24 script engine and coin script (done)
The three entries are not bytecode: `$3E00` is a bank switch plus `jp hl`, and
`$4028`/`$4512`/`$4A81` are per-screen state machines dispatched on
`sCastleState`. They share the helper set in the `$4A81` region: `$60C41`
preview tile, `$60D08` walk, `$60D32` reveal decode, `$60D80` marker, `$60DC7`
award, `$60CF9` commit.

`CastleRoom` models the RAM block (sprite table at `$A6B5` as 3-byte records —
the `sMapSpriteN*` names describe the hub's 4-byte view; five `$0D` slots; OAM
entry 0) and ports the coin script `$4A81` (states 0-6). `MapSim` screens 9-14
now tick the room instead of `$3C38C`, and skip `$3C169` (the ROM returns
without it).

New inputs: `coin_room_items` (bank 24 `$4000`, the 40-byte item table `$60D49`
reads through `$4F76`'s `$4000/$400A/$4014/$401E` row pointers) and
`coin_room_tables` (`$4F5A`: per-node sprite-3 frames, item sfx, columns, the
`$4FA8` walk offsets and the `$4FCD` reveal amounts). `$4C4D`'s twelve
preview tiles live inside the coin blob.

Tests: five byte-level room traces (preview table, state-0 gate, award +
100-frame hold + state 3, reveal/walk tables, item rows) and a `MapSim`
dispatch test.

Bell room A `$4512` (screen 13) is ported on the same engine: states 0-11
(the `$20` walk-in, the A gate, the sprite-1/2/0 id walks, the `$4E` item
crossing, `$469C`'s preview flicker, the `$4764` walk-out) plus `$6083C`'s
five-slot engine (`$60965` patch entries from `$4A1A`/`$4A2F`/`$4A58`,
`$60917` preview/lives award, `$609E4` slot commit). Four tests pin the A
gate, the `$21` page walk, a slot patch step and both award arms.

Bell room B `$4028` (screen 14) is ported too: its own `$6046A` decor walk,
`$60269` frame flash, `$60290` `$9800` tile scan, `$60314`/`$6032A` bell wobble,
the `$421A` slide, patch records `$A7D2`/`$A7ED`, sprites 10/11 and states 0-11.
Both loaders are `CastleRoom::enter_bell_a` (timer nibble -> `$4A1A`/`$4A2F`/
`$4A58`, slots 2/26/50/74/98, `$6734`) and `enter_bell_b` (`$6761`). The map
entry from screen 8 and the return landing stay with ME-5's acceptance work.

### ME-4 — Coin-door rooms 9-12 (done)
`$3F3B9`..`$3F45E` stage screens 9-12 from `$66D4` (the four lists are
identical) through `$3E2A2`, `$3F35A`'s OAM entry (156,40) and `$3F495`'s SFX
5. `MapSim`'s coin door builds `CastleRoom::enter_coin` from
`castle_scripts`, and `$3F4A0` returns through `$3D880` (screen 8, music 13)
when the script writes state 3.

The room's item comes from the door node's ten-byte row in the `$4000` table
(`$4F76` -> `$4000/$400A/$4014/$401E`), so nodes `$6E`/`$6F`/`$70`/`$71` give
rows 0-3. The session mirrors the room's `sLives`/`sCurPowerup`/`sCoin*` while
it owns the screen.

Tests: the node -> item row mapping, all four screens' staging, award-once
(state 6 only counts down), and the state-3 return to screen 8.

### ME-5 — Bell rooms 13/14 (done)
Both scripts and both loaders are ported (ME-3): `$4512` (screen 13), `$4028`
(screen 14), `enter_bell_a` (three timer variants) and `enter_bell_b`.

The entry is the level-exit hand-off, not a castle door: `$3EF2B` sets BGP 228
and OAM palettes 208/57, and when `sGoalBell != 0` picks screen 13/14 by
`sMapNode` bit 0 (even -> `$3F4B1`, odd -> `$3F5A4`) and sets `sGameState = 12`.
The port diverts the cleared-level landing to a `CastleRoom` (the timer variant
comes from `sTimerHigh`), and when the script writes state `$0A` runs the
deferred `$3EF2B` landing with the screen/node/exit-tile flag saved at entry.
`$3E9FB`'s palette fade and the loaders' VRAM/OAM copies are render-side.

### ME-6 — Golden-coin/castle chain (done)
`CastleSequence` (`sml2-core/src/castle_sequence.rs`) models `sGameState` `$1B`
to `$21`: the `$45B6` six-coin test (tile `$74` only when every `sGoldenCoinN`
flag has bit 7), the `$446B` menu dispatcher (`$453B` walk-in with the `$499D`
target, `$44AC` progress bit and five-coin fork, `$4477` five 64-frame row
shifts with SFX 2=10, `$44A1` hand-back), `$441D`'s BCD tally, `$477B`'s
4/`$FF` fade and `$47A9`'s fade-or-START exit. `$44F1`'s level ->
`sLevelProgress` offsets and `$499D`'s walk targets are pinned.

Entries: bank 2's boss-clear arm sets `$1D` (`h[$FFD9] < 8`); bank 0's state
`$16` handler sets `$1B` when `sCurLevel == $FF`. `Session` now builds the
sequence on a boss-clear exit, seeds the six coin flags from `sLevelProgress`
offsets 5/9/14/17/23/29, ticks it with START and the frame counter, and on
completion lands the map with the reveal's progress bytes.

The VRAM/OAM staging (castle tiles, `$498A` coin positions, congratulations
tilemap) is render-side; the renderer paints the sequence black for now.
Eight core tests cover the six-coin test, tally, walk-in, menu flow, row
shifts, progress-bit write and START wait.

### ME-7 — Acceptance and inventory (S)
End-to-end typed tapes: six coins → castle entry → Wario fight → ending/credits;
coin-door room; bell room; coin refusal; save/reload. Update the
`minigames-ending` row only when all are green.

## Definition of done

1. Coin-door rooms 9-12 and bell rooms 13/14 are playable through the map and
   return exactly as the ROM does; every reward byte is pinned.
2. The six-coin chain reaches the castle and the ending/credits without
   diagnostics; ending tests remain green.
3. No new captures, no `traj4_ff0` change, workspace and sweep green.
