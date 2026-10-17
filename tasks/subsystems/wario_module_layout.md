# Wario object module organization

User requests a Wario subfolder under objects. This is a mechanical layout step;
behavior and source contracts stay unchanged. Read PORTING/native guide and inspect
only relevant import/module/visibility/path declarations (no need to load whole
tests/source). Move existing Wario-specific production and test files to:

old basename => new `objects/wario/` basename
- wario20.rs => form1.rs
- wario20_tests.rs => form1_tests.rs
- wario20_integration.rs => form1_integration.rs
- wario21.rs => moving_child.rs
- wario21_tests.rs => moving_child_tests.rs
- wario21_native_tests.rs => moving_child_native_tests.rs
- native_actor_adapter.rs => moving_child_adapter.rs
- wario23.rs => form2.rs
- wario23_adapter.rs => form2_adapter.rs
- wario23_tests.rs => form2_tests.rs
- wario23_integration.rs => form2_integration.rs
- wario_child22.rs => child22.rs
- wario_child25.rs => child25.rs
- wario_child_fall.rs => child_fall.rs
- wario_phase1.rs => phase1_exit.rs
- wario_phase2.rs => phase2_exit.rs
- wario_phase2_tests.rs => phase2_exit_tests.rs

Touch ONLY these moves/import/path/module/visibility adjustments and
objects/mod.rs registration + new wario/mod.rs. Shared boss_damage, boss_spawn,
boss_stun, boss_sprite_cycle, boss_reverse_picker remain at objects root. Root
will separately update docs/tools/task path references. No dispatch behavior or
body changes. Existing source-derived tests move intact. Prefer explicit
crate::objects imports for root types/shared helpers instead of glob-reexporting
root namespace through wario/mod.rs. Facade inherent methods called from objects
root need pub(in crate::objects); native sibling APIs stay pub(super).

Compile/fix only namespace issues then full cargo test -p sml2-core and runtime.
No new tests needed for mechanical move. No mutation/sweep/commit/queue/inventory
editing. Root checks complete rename diff and frozen sweep. Preserve untracked
unrelated tasks/handlers/0x5B36D.md and all other concurrent files.
