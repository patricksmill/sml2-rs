# Typed actor boundary and first handler migration

Apply the user's [native Rust guide](../../docs/idiomatic-rust-porting.md).
This package preserves game behavior while moving logic off raw slot storage.

## Actor prerequisite

Source layout: `objects/staging.rs` documents the persistent 32-byte actor
record. World X/Y and spawn-record addresses are big-endian pairs; screen
coordinates remain separate. Signed hitbox extents and every reserved or
unknown value must survive conversion. Probe scratch is not persistent actor
state and does not enter the new model.

Source contracts for reusable operations are COMPLETE generated briefs:
[`$B40E`](../handlers/0x0B40E.md) for palette flash cadence and
[`$B1C8`](../handlers/0x0B1C8.md) for profile read-before-increment and equality
cap. Named direction and interaction methods use the existing source-verified
interaction rules; do not conflate contact kind with interaction result flags.

Permitted files: new `crates/sml2-core/src/actor.rs`, optional sibling
`actor_tests.rs`, and `lib.rs` registration only. Acceptance includes native
signed world/screen movement, explicit frame cadence, exact conversion fixtures,
all-byte preservation (including unknown direction/flags/reserved values),
profile boundaries and invalid-profile reporting, and full core tests.

## First domain handler

Target: shared Wario child animations 21/24, ROM `$AAA5`, bank `$02`, CPU `$6AA5`.
[`$AAA5` brief](../handlers/0x0AAA5.md) is COMPLETE. Preserve all four states,
the 36-step fall profile, cadence and movement order, transition consumption,
sprite completion, and sampled respawn side.

Use a native state enum and typed actor fields. Snapshot legacy world inputs
in an explicit adapter; the domain update must not name registers, slot offsets
or CPU scratch. Return domain effects to that adapter, including transition
consumption and unsupported state/data. Slot arrays enter and leave only through
the actor's two conversion methods. Retain the existing handler entry/dispatch
facade and all ROM-derived byte tests.

Permitted migration files: `objects/wario/moving_child.rs`, a new bounded native context/
adapter module if needed, `objects/mod.rs` registration only, and relevant local
tests. Shared actor helper additions require their own source contract and tests.
Acceptance: existing handler/dispatch and real Wario producer-consumer tests;
native tests for all states, boundaries and effects; sensitivity checks; full
core/runtime tests; frozen sweep with `pipefail`. Leave other legacy handlers
pending for subsequent bounded migrations.
