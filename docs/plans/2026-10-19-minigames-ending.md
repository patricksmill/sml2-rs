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
bytes): `castle_screen` $4028 (1,258 B), `bell_room_a_script` $4512 (1,288 B),
`bell_slots_0/1/2` $4A1A/$4A2F/$4A58 (21/41/41 B, `$FF`-terminated),
`coin_room_script` $4A81 (1,241 B) and the bank-15 sprite/patch lists
`bell_room_a_sprites` $6734 (45 B) and `bell_room_b_sprites` $6761 (93 B).
Each code end is a `ret`; `castle_screen`/`bell_room_a_script` end where the
next entry starts, `coin_room_script` at the $4F5A lookup tables. The sprite
lists are `$3E2A2`'s two 3-byte record runs, each closed by an `$FF` record.
`CastleScripts` and `BellRoomTiles` in `sml2-core` pin the emitted bytes.

### ME-3 — Port the bank-24 map-script VM subset (L)
Implement the commands the three scripts use: far entry, waits, sprite/patch
writes, input gates, screen state writes and returns. Drive it from
`MapSim`'s screen 9-14 handlers. Tests: byte-level script traces for `$4A81`
(coin room), `$4512`/`$4028` (bell rooms), mutation-rejected opcodes.

### ME-4 — Coin-door rooms 9-12 (M)
`$3F4A0`/`$4A81`: free-walk Mario from OAM `(40,156)`, collect the room's
sprites, award coins/power-ups, set `sCastleState == 3` and return to castle
screen 8. Prices and the unpayable path are already modelled. Tests: each of
the four rooms, reward byte, no duplicate award, return screen.

### ME-5 — Bell rooms 13/14 (M)
`$3F4B1`/`$3F5A4`: timer-variant script selection, six castle slots, SFX 5,
music 21, reward (power-up/lives), return to screen 8. Tests: all three timer
variants and the node-bit variant, exact slot bytes, reward, return.

### ME-6 — Golden-coin/castle chain (M)
Port states `$1B-$1F/$20/$21` into `SessionState` (or the existing ending
sequence controller) with the six-coin unlock and congratulations; verify the
final castle entry from the map after six coins. Tests: each state's inputs
and transitions, save/reload during the chain.

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
