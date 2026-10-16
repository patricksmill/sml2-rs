# Boss-entry object scratch reset

Queue: `boss_entry_object_scratch_reset` (pending).

Source brief: [ROM $EC40, bank $03 / CPU $6C40](../handlers/0x0EC40.md),
generated with `python3 tools/handler_task.py --addr 0xEC40`; COMPLETE.
Its zero B counter writes exactly 256 bytes at `$AF00..$AFFF`.

Caller audit: ROM $8D9C (bank $02 / CPU $4D9C) calls home $3A28,
which maps bank $03 and calls $EC40, then restores bank $02. It next
calls home $3A00 -> bank $03 $EC31 (slot-page clear), then $8DFD/$8E6C
(boss metadata and slot staging). Slot clear and scratch clear are separate.

`reset_level_start` also restores the original stream and clears state outside
AF; it is not an equivalent scratch helper. Preserve the HRAM frame counter,
player inputs, pending-item/coin/goal state, sound/fade state, and the actual
stream buffer outside AF. Native pending events are not AF bytes either.

## First prerequisite: represent the cleared stream pointer

AF `$1E/$1F` clear to physical `$0000`; the current `stream_index == 0`
represents `$AB06`. Preserve the public relative `stream_index: usize` facade
by adding a private cleared-state bit. In that state the index is inactive;
the physical accessor returns zero and stream scanning cannot read record zero.
Construction and real reseeding select the active `$AB06` cursor. The
[Wario phase-one writer](../handlers/0x0A96D.md) explicitly reseeds `$AB06`.

Permitted files: a new `objects/stream_cursor.rs` and its local tests;
`objects/mod.rs` (registration, field, constructor/reset cursor only);
`objects/stream.rs` (cleared guard and slot-record pointer accessor only);
`objects/wario_phase1.rs` (real reseed call only); runtime `main.rs` (two replay
pointer-report expressions only). Do not implement the AF clear yet.

Acceptance: constructor/reload pointer `$AB06`, literal zero after clear,
cleared scanner does not spawn and reports unsupported pointer use, reseed
restores actual scanning, wrapping physical accessor, preservation of records,
and unchanged frozen baseline. No source claim that scanning a cleared pointer
is an intentional ROM no-op; it is outside required source reachability.

## Then port the clear and its caller

Zero each represented AF field according to `ram.asm` `$AF00..$AFD0`, including
the cursor, removal mode and saved boss/Wario latches. Preserve outside-AF
state. `spawn_boss` calls the clear before staging fresh boss metadata.
Test dirty scratch, preserved outside state, and the real reentry path before
enabling def 6 in `boss_def_portable`. The source home $0EB0 writes the returned
boss song to both `sMusicRequest` and `sLevelSong`; the native caller must do both.
Keep later Wario forms gated until their own handlers and integration are ready.
