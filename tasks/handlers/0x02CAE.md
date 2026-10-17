# Handler 0x02CAE

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x02CAE`, bank `$00` (0 decimal), CPU `$2CAE`.
**Selected bank at entry:** `$00`.
**Anims:** (helper / no table entry).
**Policy:** [PORTING.md](../../PORTING.md).

Implement in a focused sibling module under `crates/sml2-core/src/objects/`;
register it in its owning `mod.rs` if new, and wire `objects/dispatch.rs`.
Wario modules belong in `objects/wario/`; shared boss helpers stay in `objects/`.
Apply `docs/idiomatic-rust-porting.md`: use typed `Actor` state from `actor.rs`.
Keep slot conversion and legacy world input/effect mapping in an explicit adapter;
do not put raw offsets, register names or HRAM scratch in domain handler logic.
Resolve typed shared-helper prerequisites first. New tests belong beside the
implementation; shared integration fixtures are currently in `objects/tests.rs`.

## Exact ROM-addressed instructions and ASM locations

Addresses and edges derive from ROM bytes. Source labels are not assumed to
mark every entry. Review table bounds and dependency contracts before porting.

```text
0x02CAE  3E 1A    home.asm:5928 ld a, 26
0x02CB0  EA 4E A2 home.asm:5929 ld [sCurROMBank], a
0x02CB3  EA 00 21 home.asm:5930 ld [$2100], a
0x02CB6  C3 00 40 home.asm:5931 jp UnknownJump_0x68000
0x68000  AF       main.asm:39214 xor a
0x68001  EA DD A2 main.asm:39215 ld [sEndingState], a
0x68004  EA DC A2 main.asm:39216 ld [sBossFightPhase], a
0x68007  FA 9B FF main.asm:39217 ldh_a_n $9B
0x6800A  3C       main.asm:39218 inc a
0x6800B  EA 9B FF main.asm:39219 ldh_n_a $9B
0x6800E  C9       main.asm:39220 ret
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
