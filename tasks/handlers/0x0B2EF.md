# Handler 0x0B2EF

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x0B2EF`, bank `$02` (2 decimal), CPU `$72EF`.
**Selected bank at entry:** `$02`.
**Anims:** (helper / no table entry).
**Policy:** [PORTING.md](../../PORTING.md).

Implement in a focused sibling module under `crates/sml2-core/src/objects/`;
register it in `objects/mod.rs` if new, and wire `objects/dispatch.rs`.
Apply `docs/idiomatic-rust-porting.md`: use typed `Actor` state from `actor.rs`.
Keep slot conversion and legacy world input/effect mapping in an explicit adapter;
do not put raw offsets, register names or HRAM scratch in domain handler logic.
Resolve typed shared-helper prerequisites first. New tests belong beside the
implementation; shared integration fixtures are currently in `objects/tests.rs`.

## Exact ROM-addressed instructions and ASM locations

Addresses and edges derive from ROM bytes. Source labels are not assumed to
mark every entry. Review table bounds and dependency contracts before porting.

```text
0x0B2EF  F0 F0    main.asm:8059 ldh a, [$FFF0]
0x0B2F1  E6 07    main.asm:8060 and $07
0x0B2F3  C0       main.asm:8061 ret nz
0x0B2F4  18 0C    main.asm:8062 jr UnknownRJump_0xB302
0x0B302  F0 DB    main.asm:8076 ldh a, [$FFDB]
0x0B304  EE 01    main.asm:8077 xor $01
0x0B306  E0 DB    main.asm:8078 ldh [$FFDB], a
0x0B308  C9       main.asm:8079 ret
```

## Recovered rst $28 state tables (CPU targets)

- None.

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
