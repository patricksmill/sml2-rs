# Handler 0x09CAC

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x09CAC`, bank `$02` (2 decimal), CPU `$5CAC`.
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
0x09435  3E 02    main.asm:4029 ld a, 2
0x09437  E0 DF    main.asm:4030 ldh [$FFDF], a
0x09439  3E 20    main.asm:4031 ld a, 32
0x0943B  E0 EC    main.asm:4032 ldh [$FFEC], a
0x0943D  FA CA AF main.asm:4033 ld a, [sObjectDefSprite]
0x09440  E0 DB    main.asm:4034 ldh [$FFDB], a
0x09442  C9       main.asm:4035 ret
0x09CAC  CD 0E 74 main.asm:4967 call UnknownCall_0xB40E
0x09CAF  F0 EC    main.asm:4968 ldh a, [$FFEC]
0x09CB1  A7       main.asm:4969 and a
0x09CB2  28 06    main.asm:4970 jr z, UnknownRJump_0x9CBA
0x09CB4  F0 EC    main.asm:4971 ldh a, [$FFEC]
0x09CB6  3D       main.asm:4972 dec a
0x09CB7  E0 EC    main.asm:4973 ldh [$FFEC], a
0x09CB9  C0       main.asm:4974 ret nz
0x09CBA  FA 26 AF main.asm:4977 ld a, [sObjectRemoveMode]
0x09CBD  A7       main.asm:4978 and a
0x09CBE  C2 35 54 main.asm:4979 jp nz, UnknownJump_0x9435
0x09CC1  AF       main.asm:4980 xor a
0x09CC2  E0 DE    main.asm:4981 ldh [$FFDE], a
0x09CC4  FA 37 AF main.asm:4982 ld a, [sObjectSpawnSprite]
0x09CC7  E0 DB    main.asm:4983 ldh [$FFDB], a
0x09CC9  FA C4 AF main.asm:4984 ld a, [sSavedFrameState]
0x09CCC  E0 DF    main.asm:4985 ldh [$FFDF], a
0x09CCE  FA CC AF main.asm:4986 ld a, [sSavedObjectType]
0x09CD1  E0 E2    main.asm:4987 ldh [$FFE2], a
0x09CD3  C9       main.asm:4988 ret
```

## Recovered rst $28 state tables (CPU targets)

- None.

## Existing helper dependencies supplied by the supervisor

- ROM 0x0B40E: `crates/sml2-core/src/objects/boss_flash.rs::boss_flash_b40e` (reviewed bank-preserving contract)

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
