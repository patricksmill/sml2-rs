# Native VRAM transfer service prerequisite

One shared VRAM transfer routine ROM `$026D7` (bank $00, CPU `$26D7`), COMPLETE
in `tasks/handlers/0x026D7.md` under entry contract `ending-transfer-vblank`.

Source logic:
- Reads transfer state:
  - `sVRAMTransferBank` ($A265): source ROM bank.
  - `$FFB1/$FFB2`: source CPU address (16-bit).
  - `$FFB3/$FFB4`: destination VRAM address (16-bit).
  - `$FFB5/$FFB6`: remaining bytes (16-bit).
  - `sVRAMTransferActive` ($A248): nonzero when transfer is in progress.
- Cadence per frame:
  - Copies bytes from source to destination, incrementing source and dest,
    and decrementing remaining bytes.
  - Stops when `(remaining_bytes.low & 0x3F) == 0` (normally 64 bytes per frame;
    if unaligned, copies the remainder down to the next 64-byte boundary).
  - When remaining bytes reach 0, clears `sVRAMTransferActive = 0`.
- In state 19 (ending sequence), the conditional call to `UpdateSound` at $2716
  is proved untaken (`cp $13`).
- Hardware plumbing (bank switching, VBlank pop/reti) is eliminated;
  provide clean typed state and step method.

Implementation:
- New bounded module `crates/sml2-core/src/vram_transfer.rs` (+ tests).
- Add `tiles_mut` and `tilemap_mut` accessors to `EndingAssets` / `EndingScene`.
- Domain types: `VramTransfer`, `TransferStepResult`, `TransferError`.
- Clean methods: `new`, `step`, `apply_to_scene`.
- Tests:
  - 64-byte aligned chunk cadence across multiple frames
  - Unaligned initial length handling (remainder copy then 64-byte chunks)
  - Completion sets active = false and remaining = 0
  - Actual ending request 1 ($4656 -> $8000, len $0800 = 2048 bytes in 32 frames)
  - Actual ending request 2 ($5096 -> $9800, len $0400 = 1024 bytes in 16 frames)
  - Exact destination mapping into `EndingScene` tiles and tilemap
