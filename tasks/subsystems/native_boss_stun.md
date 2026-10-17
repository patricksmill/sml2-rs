# Native shared boss hit stun

One shared helper ROM `$9CAC` bank2, COMPLETE
[`source brief`](../handlers/0x09CAC.md), including defeat transition `$9435`.
Native typed Actor update accepts FrameTick and saved reaction context:
remove pending, definition sprite, saved sprite/state/contact kind.
Palette flash runs first; timer zero or decrement-to-zero completes immediately.
Pending removal sets state2/timer32/definition sprite and DOES NOT clear flash
or contact kind. Recovery clears all flash and restores saved sprite/state/kind.
All unwritten actor fields survive. No world state is changed by the helper.

Read PORTING/native guide, named brief, actor.rs, boss.rs hit-stun method plus
boss_defeat_fall (bounded excerpts), objects/boss_flash_paths.rs existing tests.
Touch ONLY new objects/boss_stun.rs plus optional sibling tests/adapter,
boss.rs ONLY replace boss_hit_stun body with delegation (no unrelated helpers),
and mod.rs registration. Keep facade signature and existing tests unchanged.
Native function is callable from future Wario23 typed domain logic; expose typed
context/update within objects module. Adapter snapshots source world fields and
converts actor using only Actor methods.

Tests: timer0/1/2/255, all frame cadence, remove/recover, complete byte preservation,
unknown saved values, actual old caller tests; meaningful mutation fail/restore,
full core gate. Root controls mutation window/sweep/commit; do not touch queues
or inventory. Native Actor flash method already establishes shared behavior.
