# Handler 0x09CAC

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x09CAC`, bank `$02` (2 decimal), CPU `$5CAC`.
**Selected bank at entry:** `$02`.
**Anims:** (helper / no table entry).
**Policy:** [PORTING.md](../../PORTING.md).

Implement in a focused sibling module under `crates/sml2-core/src/objects/`;
register it in `objects/mod.rs` if new, and wire `objects/dispatch.rs`.
Reuse `objects/staging.rs` and `objects/helpers.rs`. New tests belong beside the
implementation; shared integration fixtures are currently in `objects/tests.rs`.

## Exact ROM-addressed instructions and ASM locations

Addresses and edges derive from ROM bytes. Source labels are not assumed to
mark every entry. Review table bounds and dependency contracts before porting.

```text
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
0x0B40E  F0 F0    main.asm:8282 ldh a, [$FFF0]
0x0B410  E6 03    main.asm:8283 and $03
0x0B412  C0       main.asm:8284 ret nz
0x0B413  F0 DE    main.asm:8285 ldh a, [$FFDE]
0x0B415  EE 10    main.asm:8286 xor $10
0x0B417  E0 DE    main.asm:8287 ldh [$FFDE], a
0x0B419  C9       main.asm:8288 ret
```

## Recovered rst $28 state tables (CPU targets)

- None.

## Existing helper dependencies supplied by the supervisor

- ROM 0x09435: `crates/sml2-core/src/objects/boss.rs::boss_defeat_fall` (reviewed bank-preserving contract)

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
