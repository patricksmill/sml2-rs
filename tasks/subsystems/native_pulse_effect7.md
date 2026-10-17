# Native ending pulse effect7

One next sound unit, source COMPLETE tasks/handlers/0x10217.md and0x1021D.md.
Read PORTING/native guide and existing audio/pulse.rs only, plus named exact
source packets. Root ROM check: frame1022F is [00,80,A7,83,87,04]; second
frequency-only frame10235 is [9D,07,50]. Reuse native effect/music state and
source start/stop helpers; do not duplicate common logic or emulate CPU/RAM.

Touch ONLY audio/pulse.rs and optional audio/pulse/tests.rs (move existing local
tests out if production+tests grows beyond400 lines). Implement start_effect7
copying request unchanged as shared start, five initial writes/timer4. Tick
wrapping decrement, at zero wrapping step++, equality2 invokes shared stop,
otherwise write ONLY FrequencyLow9D,FrequencyControl07 then timer80. The second
frame must NOT reset sweep/duty/envelope or trigger flags. Noncanonical steps
still use equality-only stop. Retain effect6 behavior/fixtures.

Tests exact initial and partial ordered write lists,4+80 complete cadence,
source timer0 and step255 wrap, stop equality and unrelated state preserved,
actual shared helper consumption. Mutation frequency-only vs full frame, timer
80 and stop equality fail assertions then restore. Root will independently
check ROM frames. Full core/runtime/extractor after focused tests. Lane free;
you may run scoped sequential mutants/fullsuite without extra permission.
No commit/sweep/queue/inventory/captures edits; report exact files/results.
