# Final-form Wario rise profile prerequisite

One shared profile subsystem extension: COMPLETE bank2 briefs
`tasks/handlers/0x0B1CF.md` and `0x0B1EB.md`. Read PORTING/native guide,
actor_motion.rs and these briefs. Touch ONLY actor_motion.rs (including local
native tests), no other handler/shared code or registrations.

Extend existing RiseProfile with High (cap27, table offset9 at `$B208`) and
Low (cap15, offset21 at `$B214`) or equally clear intermediate heights. Keep
Full36/Medium21/Short12 existing API and single literal table unchanged.
Actor::rise_step already implements source semantics via motion_step. Verify
cap/start instruction bytes and table offsets independently from ROM.
Test every literal distance for both curves, repeated terminal reads, invalid
phase errors and complete unrelated-slot preservation. Existing profile tests
must remain unchanged and pass. These are source-ready prerequisites for Wario26;
no facade/dispatch needed for not-yet-ported callers. Meaningful fail/restore
sensitivity and full core tests; ask root for test lane (alternation then Wario23
own windows next). No commit/sweep/inventory/queue edits.
