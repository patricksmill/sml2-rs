# Native credits text service prerequisite

One shared ending service routine ROM `$68572` (bank $1A, CPU `$4572`), COMPLETE
in `tasks/handlers/0x68572.md` under entry contract `ending-text-vblank`.

Source logic:
- Reads `sCreditsTextPhase` (starts at 4 for a 4-row page, decrements to 0).
- Destination tilemap row: `row_index = (4 - phase)`, row base `$9A20` -> tilemap
  offset `0x0220 + row_index * 32` (row 17 + row_index in 32x32 background tilemap).
- Row reading:
  - If next byte in text is `0xFF`, it is a blank row marker: advance pointer by 1
    and fill 20 tiles in the destination row with `0xFF`.
  - Else, read 20 characters from `text[ptr..ptr+20]`, advance pointer by 20.
    For each character `c`: `tile = c.wrapping_sub(48)`; if `tile == 0xF0` (ASCII space
    0x20), `tile = 0xFF` (blank tile in font). Write tile to destination row.
- Termination check:
  - If byte at current pointer is `0xFE`, sets `sCreditsTextDone = 0xFF`.
- Phase decrement:
  - Decrements phase.
  - If phase reaches 0, sets fade timer to 256 (`sFadeTimerHigh = 1`, `sFadeTimerLow = 0`).
- Hardware plumbing (VBlank register restore, RETI, bank switching) is eliminated;
  retain clean typed domain logic, state, and effects.

Implementation:
- New bounded module `crates/sml2-core/src/credits_text.rs` (+ sibling tests).
- Register in `crates/sml2-core/src/lib.rs`.
- Domain types: `CreditsTextState`, `CreditsStepResult`, `CreditsTextError`.
- Clean methods: `char_to_tile`, `step`.
- Comprehensive unit tests:
  - Blank row marker `0xFF` handling and pointer advance by 1
  - ASCII space `0x20` to `0xFF` mapping, digits '0'..'9', letters 'A'..'Z'
  - Pointer advancement by 20 on regular row
  - Phase 4 down to 0, fade timer set to 256 on phase 0
  - Done flag set when next character is `0xFE`
  - Exact tilemap destination offsets for all 4 rows
  - Truncated text and bounds errors
  - Full simulation across all 36 pages (144 rows, 1798 bytes) of original ROM credits text
