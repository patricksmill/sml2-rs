# Boss-entry object scratch reset

Queue: `boss_entry_object_scratch_reset` — clear and both callers landed
2026-10-16 (`fdf40ff` cursor prerequisite, this commit's page clear).

Source brief: [ROM $EC40, bank $03 / CPU $6C40](../handlers/0x0EC40.md),
generated with `python3 tools/handler_task.py --addr 0xEC40`; COMPLETE.
Its zero B counter writes exactly 256 bytes at `$AF00..$AFFF`.

Caller audit: ROM $8D9C (bank $02 / CPU $4D9C) calls home $3A28,
which maps bank $03 and calls $EC40, then restores bank $02. It next
calls home $3A00 -> bank $03 $EC31 (slot-page clear), then $8DFD/$8E6C
(boss metadata and slot staging). Slot clear and scratch clear are separate.
The `$EBBB` level-(re)load tail runs the *same* `$EC40`, then `$EC17`
re-seeds the stream buffer. `reset_level_start` also clears state outside AF;
the scratch helper is not a substitute for it.

`objects/scratch.rs::clear_object_scratch` zeroes every represented cell by
`ram.asm` address (`$AF00/$AF01`, `$AF06`, `$AF08`+`phase`, `$AF09`,
`$AF1E/$AF1F` via the cleared cursor, `$AF22`, `$AF24/$AF25`, `$AF26`,
`$AF29`, `$AF2A`, `$AF2B`, `$AF2C`, `$AF2E`, `$AF2F`, `$AF30-$AF33`,
`$AF36-$AF38`, `$AF3A`, `$AF3E`, `$AF40`, `$AF41`, `$AFC0`, `$AFC1`,
`$AFC3-$AFC5`, `$AFCA`, `$AFCC-$AFD0`) and leaves everything outside the page
alone: the HRAM frame counter `$FFF0`, the sampled `$FF04`, the stream buffer
`$AB00-$ACFF` with its spawn markers, the slot page (cleared separately by
`$EC31`), and the player/timer/fade/audio bytes in `$A2xx`/`$A468`/`$A53F`.
Both callers also drop the per-slot draw latch with the slot-page clear.

The `$0EB0` scene-branch caller now writes the boss song to **both**
`sMusicRequest` and `sLevelSong` (`$0ECA` writes 32 to each; the `$0ED9` arm
overrides only `sMusicRequest` with `sBossFightPhase + 33` on level `$18`).

## Still gated: def 6

`boss_def_portable` still admits only def 2. Enabling Wario's fight (def 6,
level 0) takes the `$F0` park, and that park is only cleared by the level
clear the fight ends in — so it stays gated until `boss_wario_anim23`
(`$AB8B`, whose dispatch arm is the fight's next state) and the fight's
ending (`boss_clear_ending_handoff`) land. Enabling it earlier parks the
camera for a fight that cannot finish; the verified entry path itself
(`spawn_boss(6)` from a dirty, played state) is covered by
`boss_entry_spawn_boss_clears_the_page_then_stages_the_fight`.

## Test contract

- dirty every represented cell → all zero, the cursor reads physical `$0000`
  (not `$AB06`), and records/markers are untouched;
- outside state preserved (fade timer and its deferred write, BCD timer input,
  sampled DIV, pending item, coin/goal latches, sound bytes, deferred camera
  and scene writes, the unported report);
- the real reentry: `spawn_boss(6)` stages defs 20/21 and the driver renders a
  frame with no reports;
- the reload path runs the same `$EC40` and then ends on the live `$AB06`
  cursor ($EC17).