# Handler 0x0B2DB

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x0B2DB`, bank `$02` (2 decimal), CPU `$72DB`.
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
0x0B2DB  21 DA FF main.asm:8043 ld hl, $FFDA
0x0B2DE  FA 3C A2 main.asm:8044 ld a, [sMarioScreenX]
0x0B2E1  96       main.asm:8045 sub [hl]
0x0B2E2  CB 7F    main.asm:8046 bit 7, a
0x0B2E4  C8       main.asm:8047 ret z
0x0B2E5  2F       main.asm:8048 cpl
0x0B2E6  3C       main.asm:8049 inc a
0x0B2E7  C9       main.asm:8050 ret
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
