# Native Sound Driver & SFX Dispatcher

- **ROM addresses:**
  - `_UpdateSound` root: ROM `0x10030` (`main.asm:9411..9460`), bank `$04`, CPU `$4030`
  - Common exit & request clear: ROM `0x10067` (`main.asm:9446..9453`), CPU `$4067`
  - 1-based table pointer lookup: ROM `0x111A6` (`main.asm:10966..10977`), CPU `$51A6`
  - Slot 0 (Pulse 1) dispatch: ROM `0x100DE` (`main.asm:9469..9512`), CPU `$40DE`
  - Slot 0 Start/Tick pointer tables: ROM `0x1007E` / `0x100AE` (`main.asm:9460..9465`), CPU `$407E` / `$40AE`
  - Slot 1 (Wave) dispatch: ROM `0x106D3` (`main.asm:10054..10081`), CPU `$46D3`
  - Slot 1 Start/Tick pointer tables: ROM `0x106FA` / `0x1072A` (`main.asm:10084..10089`), CPU `$46FA` / `$472A`
  - Slot 2 (Noise) dispatch: ROM `0x10E90` (`main.asm:10706..10729`), CPU `$4E90`
  - Slot 2 Start/Tick pointer tables: ROM `0x10E64` / `0x10E7A` (`main.asm:10701..10703`), CPU `$4E64` / `$4E7A`
- **Subsystem:** Audio engine / Sound effect dispatch & channel sequencing
- **Module:** `crates/sml2-core/src/audio/driver.rs`

## 1. Domain Overview

The Game Boy sound engine (`_UpdateSound` at Bank 4 `$10030`) executes once per frame to dispatch sound effect requests, update music streams, and clear request registers:

1. **Root Dispatch (`$10030`)**:
   - Checks `sSoundMode`: mode 1 = pause handling, mode 2 = unpause handling.
   - Checks `sSoundPaused`: skips processing if audio is paused.
   - If `sMusicRequest != 0`: initiates song loading (`$112E0`) and updates music channels (`$11638`).
   - Normal path:
     - Derives master volume (`rAUDVOL`): `$77` (119) normally, or `$33` (51) when `sLowTimeFlag != 0`.
     - Dispatches Slot 0 (Pulse 1 / Channel 1) at `$100DE`.
     - Dispatches Slot 1 (Wave / Channel 2) at `$106D3`.
     - Dispatches Slot 2 (Noise / Channel 4) at `$10E90`.
     - Updates music channels at `$11638`.
   - Common Exit (`$10067`): Clears `sSfxRequest0`, `sMusicRequest`, `sSfxRequest1`, `sSfxRequest2`, and `sSoundMode` to 0.

2. **1-Based Pointer Lookup (`$111A6`)**:
   - `lookup_sfx_pointer(base, id)` computes target address from word table:
     `target = table[id - 1]`.
   - ID 0 is rejected as invalid (caller checks ensure non-zero).

3. **Channel Busy & Exception Rules**:
   - **Slot 0**: If channel is busy, new requests are ignored unless:
     - ID is an exception: `0x09, 0x0C, 0x0F, 0x10, 0x11, 0x13, 0x14, 0x17` (decimal: 9, 12, 15, 16, 17, 19, 20, 23)
     - Or `sMusicCh1VibratoTimer != 0`.
   - **Slot 1**: If channel is busy, new requests are ignored unless ID == `0x14` (20 decimal).
   - **Slot 2**: If channel is busy, all new requests are ignored (no exceptions).

4. **Ending Integration**:
   - Ending sequence substates produce:
     - `sfx_request_0 = 7`: burst / coin bounce (Effect 7)
     - `sfx_request_0 = 6`: castle transformation (Effect 6)
     - `sfx_request_1 = 24`: ending burst sound (Slot 1 ID 24)
