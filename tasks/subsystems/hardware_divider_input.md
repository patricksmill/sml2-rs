# Sampled divider input prerequisite

Source status: COMPLETE for the divider register contract and its Wario reader.
This does not establish the original console's phase at any replay snapshot.

Exact generated reader: [ROM $AAA5, bank $02 / CPU $6AA5](../handlers/0x0AAA5.md),
generated with `python3 tools/handler_task.py --addr 0xAAA5` (COMPLETE).
At ROM $AB1D the `FA 04 FF` instruction samples hardware `$FF04`; $AB20
selects bit 0. Odd selects screen X `$A7` and left direction 0; even selects
screen X 0 and right direction 1. The direction was cleared before the read.

Hardware authority: Nintendo's *Game Boy Programming Manual*, section 2.4.2,
printed page 24 ([transcription](https://manualzilla.com/doc/5977717/gameboy-programming-manual)).
DIV exposes the upper byte of a sixteen-bit counter driven by a 4,194,304 Hz
clock. Writing the register clears the counter. The original ROM has no DIV
write in the audited assembly; startup clears RAM and HRAM without clearing
this hardware register. Frame waits do not synchronize its phase.

## Bounded implementation

Permitted core file: `crates/sml2-core/src/divider.rs`; registration in `lib.rs`
belongs to the supervisor. Implement this single register's counter, explicit
cycle advancement, byte sampling and conversion from elapsed duration. No CPU
instruction execution, memory bus or general hardware emulation is needed.

The native runtime will own a monotonic clock across level loads and pauses.
Sampling elapsed time at the object input boundary supplies a hardware input,
just as the native loop supplies key input. Rendering must not set its value.
The explicit initial counter is a native clock origin, not a claim about the
console boot ROM. A deterministic replay may use one documented initial
counter and independently advanced virtual time; later console values may
only be assertions. Exact console-phase comparisons require an initial phase
and sampling-time evidence, which the frozen corpus does not currently supply.

## Acceptance

- Counter lower-byte carry, upper-byte wrap, large advances and seed preservation.
- Duration conversions on both sides of a divider-byte boundary and full wrap;
  use independently calculated integer expectations, retaining sub-byte phase.
- Explicit samples cover both reader parities; object/reset code cannot reset
  the runtime clock.
- Runtime/native/replay input wiring is a separate integration step; a tested
  counter alone does not complete queue `hardware_divider_input`.
