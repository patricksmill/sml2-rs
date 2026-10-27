# Game frame clock (`$FF97`, `sTileAnimTimer`)

The ROM increments `$FF97` and `sTileAnimTimer` (`$A298`) only in its `$0300`
frame wait. Loaders that turn VBlank off and bank 15's own `$3E9EC` fade waits
block for whole console frames without reaching it. The session keeps one
`ff97` and the tile-animation clock, and `Session::main_loop_waits` decides each
frame whether that wait runs:

| Interval | Frames without the wait | Source |
|---|---|---|
| Map → level entry | `$3E9FB` fade and save (frames 1–50), then the `$03` loader (55–73); `$0B`, `$02`, `$02` and the first `$03` frame tick; the Easy card's `$17` frames tick | `MapTransition::frame_waits` |
| Door walks | the whole `MapScreenTransition` | bank 15 door presentation |
| Hub reload | the whole `MapPanReturn` | `$3F156` + `$3EA9A` |
| Level return to the hub | the 12-frame fade-in (`MapPanReturn::fade_in`) | `$3F03D` → `$3EA9A` |
| Bell room entry | `BELL_ENTRY_HOLD` (7) | `$3F4B1` copy |
| Bell room return | all but the last two frames of the hold (83 of the hub's 84) | `$3E9FB` + screen load |
| File select load | every `$19` frame | `$0348` LCD off |

DX shortens bank 15's fades: map → level entry runs 12 frames fewer (the level
goes live 12 frames sooner), and door walks fade over 36 frames instead of 48
(`MapTransition::from_map_dx`, `MapScreenTransition::new_dx`).

Within a level, every update ticks. The tile-animation clock increments
before `$3873` checks for a copy, so a copy frame reads timer 0. The PyBoy
recordings confirm this: level 1's first copies are at rows 586 and 594. A level
still starts its own movement phase (`sim.phase`) at 0, so played routes
are reproducible.

## Measured against the ROM

`tools/compare_tas.py --debug-state --trace` prints the ROM's and the port's
`$A298`/`$A2CD`/`$FF97` side by side. Across the three manual recordings, every
interval above starts and stops on the same rows as the ROM, apart from these:

- **The one-row joypad shift.** Freezes triggered by input (level entry, door,
  hub reload) start one row later, because the bridge's port reads the press a
  frame after the ROM. The level entry recovers it at `$0B`.
- **Loads the port does not model**, deliberately (see
  `file_select_loading.rs`: waits are not guessed from recordings):
  - the title load (7 frames);
  - `$19`'s LCD restart (1);
  - the file select → hub `$14` load and fade-in (33);
  - the level → map `$14` load (7–9);
  - the door transitions' screen loads (3–10).

  In each of these the port is ready sooner, as a native port should be.
- **Console lag.** The game loop sometimes overruns a frame: in level, at the
  bell arm (`$04` → `$18`), and entering a bell room (a double tick). That is
  CPU time, which a native port does not reproduce.

Because the clock's absolute value depends on those unmodelled loads and on
lag since power-on, animated tiles in a level do not land on the console's
exact frames. They keep the right period and order, but their phase differs.
Matching it would mean emulating loader and game-loop CPU time, which is
outside the native-port design ([presentation contract](../DESIGN.md#simulation-and-presentation)).
