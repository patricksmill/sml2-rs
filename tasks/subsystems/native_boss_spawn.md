# Native shared boss child spawn

One shared helper ROM `$B224` bank2, COMPLETE brief
[`0x0B224.md`](../handlers/0x0B224.md). Source-defined count domain 0..=15.
For count<15, initialize child at count+1: definition from spawn animation,
copy ONLY parent's spawn_record/activity/screen position, sprite from request,
flip from request; preserve every other dirty target field. Increment count.
For count15 capacity abort, do not modify slots/count. BOTH successful and full
paths clear spawn animation/sprite/attribute request (source `$B259`). Current
Rust incorrectly returns early on full before clearing request; correct and test.
For malformed counts outside source domain, reject explicitly rather than
emulate out-of-array memory. No silent clamping/fabricated default child.

Read native guide/PORTING, named brief, actor.rs and bounded boss_spawn_child
body in boss.rs, plus `the_boss_child_spawn_*` existing test around tests.rs2310.
Touch new bounded objects/boss_spawn.rs and optional sibling tests/adapter,
boss.rs ONLY replace boss_spawn_child body (preserve other workers' changes),
and mod.rs registration. Expose reusable native Actor/typed request + pool
behavior for upcoming Wario23. Keep slot conversion exclusively Actor methods
at explicit compatibility boundary; game rules belong in native helper.

Native + whole byte/world tests: counts0/14/15, dirty parent/child/all other
slots, every copied/preserved field, request clearing on capacity path, invalid
count diagnostic/error, existing caller regression. Full core, mutation fail and
restore. Ask root before mutation/full-suite window; root running package sweep
soon. No sweep/commit/queue/inventory edits. Keep files bounded and preserve
other local changes in boss.rs/mod.rs.
