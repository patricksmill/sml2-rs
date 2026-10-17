# Native shared boss damage

One shared helper ROM `$9285` bank 2, COMPLETE
[`source brief`](../handlers/0x09285.md). Source audit: projectile primary-slot
path executes AND A at `$9298` (clears carry), then DEC at `$92A2` (preserves
carry). Zero hits therefore wraps to 255 WITHOUT defeat; current overflowing_sub
Rust wrongly treats that case as borrow. Stomp uses SUB2 and defeats on zero or
borrow. Prioritize stomp if both interaction flags are set.

Native function takes Actor interaction flags plus typed context (primary slot,
remaining hits, boss definition), returns descriptive effects for processed hit,
stomp latch, hit count update, sound selection and defeat/clear-fade. Do not expose
CPU flags/registers in the domain API. Adapter applies only source writes, retaining
unwritten old latch/sounds/done/remove/fade values. Preserve slot entirely.
Secondary-slot hits set done and stomp latch as appropriate, but never HP/sounds.
Defeat sets remove1; only definition<6 sets fade0280 and boss-clear1.

Read PORTING, native guide, named brief, actor.rs, boss.rs lines1-40 and bounded
existing test `the_boss_damage_pass_books_hp_sfx_and_the_kill` in objects/tests.rs.
Touch new bounded `objects/boss_damage.rs` (+ optional sibling tests/adapter),
`boss.rs` ONLY replace boss_damage_pass body with delegation (no new handler),
and mod.rs registration. Keep facade and existing tests unchanged.

Test native outcomes and adapter exact persistent/world bytes: no hit/other flags,
both precedence, secondary, HP0/1/2/3/255 with both damage types, definition5/6
boundary, unchanged old fields and entire actor, real existing caller path.
Add source-derived regression for zero-count projectile correction. Full core and
meaningful fail-and-restore mutations. Ask root for mutation window. No sweep,
commit, queue/inventory editing. Report the correction distinctly from migration.
