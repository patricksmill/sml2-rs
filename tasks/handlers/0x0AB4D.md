# Handler 0x0AB4D

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x0AB4D`, bank `$02` (2 decimal), CPU `$6B4D`.
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
0x0AB4D  F0 F0    main.asm:7002 ldh a, [$FFF0]
0x0AB4F  E6 01    main.asm:7003 and $01
0x0AB51  C0       main.asm:7004 ret nz
0x0AB52  06 01    main.asm:7005 ld b, 1
0x0AB54  CD 76 72 main.asm:7006 call UnknownCall_0xB276
0x0AB57  FA 09 AD main.asm:7007 ld a, [sObject0Y]
0x0AB5A  E6 F0    main.asm:7008 and $F0
0x0AB5C  4F       main.asm:7009 ld c, a
0x0AB5D  F0 D9    main.asm:7010 ldh a, [$FFD9]
0x0AB5F  E6 F0    main.asm:7011 and $F0
0x0AB61  C6 20    main.asm:7012 add 32
0x0AB63  B9       main.asm:7013 cp c
0x0AB64  C0       main.asm:7014 ret nz
0x0AB65  3E 01    main.asm:7015 ld a, 1
0x0AB67  EA D0 AF main.asm:7016 ld [sBossDelay], a
0x0AB6A  CD 57 71 main.asm:7017 call UnknownCall_0xB157
0x0AB6D  C9       main.asm:7018 ret
0x0B157  21 C1 AF main.asm:7788 ld hl, sObjectDefCount
0x0B15A  35       main.asm:7789 dec [hl]
0x0B15B  21 D0 FF main.asm:7792 ld hl, $FFD0
0x0B15E  AF       main.asm:7793 xor a
0x0B15F  06 20    main.asm:7794 ld b, 32
0x0B161  22       main.asm:7797 ld [hli], a
0x0B162  05       main.asm:7798 dec b
0x0B163  20 FC    main.asm:7799 jr nz, UnknownRJump_0xB161
0x0B165  3E 01    main.asm:7800 ld a, 1
0x0B167  EA 3E AF main.asm:7801 ld [sObjectDone], a
0x0B16A  C9       main.asm:7802 ret
0x0B276  F0 D9    main.asm:7971 ldh a, [$FFD9]
0x0B278  80       main.asm:7972 add b
0x0B279  E0 D9    main.asm:7973 ldh [$FFD9], a
0x0B27B  C9       main.asm:7974 ret
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
