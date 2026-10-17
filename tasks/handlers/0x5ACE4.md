# Handler 0x5ACE4

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x5ACE4`, bank `$16` (22 decimal), CPU `$6CE4`.
**Selected bank at entry:** `$16`.
**Anims:** b:104, b:105.
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
0x03AA0  3E 02    home.asm:7608 ld a, 2
0x03AA2  EA 4E A2 home.asm:7609 ld [sCurROMBank], a
0x03AA5  EA 00 21 home.asm:7610 ld [$2100], a
0x03AA8  CD 3C 46 home.asm:7611 call UnknownCall_0x863C
0x03AAB  3E 16    home.asm:7612 ld a, 22
0x03AAD  EA 4E A2 home.asm:7613 ld [sCurROMBank], a
0x03AB0  EA 00 21 home.asm:7614 ld [$2100], a
0x03AB3  C9       home.asm:7615 ret
0x03CAA  2A       home.asm:7872 ld a, [hli]
0x03CAB  12       home.asm:7873 ld [de], a
0x03CAC  1C       home.asm:7874 inc e
0x03CAD  05       home.asm:7875 dec b
0x03CAE  20 FA    home.asm:7876 jr nz, UnknownRJump_0x3CAA
0x03CB0  C9       home.asm:7877 ret
0x03D22  21 D1 FF home.asm:7953 ld hl, $FFD1
0x03D25  7E       home.asm:7956 ld a, [hl]
0x03D26  90       home.asm:7957 sub b
0x03D27  32       home.asm:7958 ld [hld], a
0x03D28  7E       home.asm:7959 ld a, [hl]
0x03D29  DE 00    home.asm:7960 sbc 0
0x03D2B  77       home.asm:7961 ld [hl], a
0x03D2C  C9       home.asm:7962 ret
0x08240  F0 D6    main.asm:2485 ldh a, [$FFD6]
0x08242  FE AD    main.asm:2486 cp $AD
0x08244  D0       main.asm:2487 ret nc
0x08245  F0 D5    main.asm:2488 ldh a, [$FFD5]
0x08247  5F       main.asm:2489 ld e, a
0x08248  16 00    main.asm:2490 ld d, 0
0x0824A  21 C1 4B main.asm:2491 ld hl, $4BC1
0x0824D  FA 06 AF main.asm:2492 ld a, [sSpriteMapIndex]
0x08250  A7       main.asm:2493 and a
0x08251  20 03    main.asm:2494 jr nz, UnknownRJump_0x8256
0x08253  21 61 4B main.asm:2495 ld hl, $4B61
0x08256  19       main.asm:2498 add hl, de
0x08257  7E       main.asm:2499 ld a, [hl]
0x08258  21 DB FF main.asm:2500 ld hl, $FFDB
0x0825B  22       main.asm:2501 ld [hli], a
0x0825C  2C       main.asm:2502 inc l
0x0825D  FA 38 AF main.asm:2503 ld a, [sObjectSpawnAttr]
0x08260  22       main.asm:2504 ld [hli], a
0x08261  06 11    main.asm:2505 ld b, 17
0x08263  AF       main.asm:2506 xor a
0x08264  22       main.asm:2509 ld [hli], a
0x08265  05       main.asm:2510 dec b
0x08266  20 FC    main.asm:2511 jr nz, UnknownRJump_0x8264
0x08268  AF       main.asm:2512 xor a
0x08269  EA 38 AF main.asm:2513 ld [sObjectSpawnAttr], a
0x0826C  C9       main.asm:2514 ret
0x0863C  F0 DD    main.asm:3070 ldh a, [$FFDD]
0x0863E  EA 38 AF main.asm:3071 ld [sObjectSpawnAttr], a
0x08641  CD 40 42 main.asm:3072 call UnknownCall_0x8240
0x08644  3E 01    main.asm:3073 ld a, 1
0x08646  EA 3E AF main.asm:3074 ld [sObjectDone], a
0x08649  C9       main.asm:3075 ret
0x5ACE4  F0 DF    main.asm:33715 ldh a, [$FFDF]
0x5ACE6  EF       main.asm:33716 rst $28
0x5ACED  3E 20    main.asm:33723 ld a, 32
0x5ACEF  E0 EC    main.asm:33724 ldh [$FFEC], a
0x5ACF1  3E 02    main.asm:33725 ld a, 2
0x5ACF3  E0 DF    main.asm:33726 ldh [$FFDF], a
0x5ACF5  3E 10    main.asm:33727 ld a, 16
0x5ACF7  E0 DE    main.asm:33728 ldh [$FFDE], a
0x5ACF9  C9       main.asm:33729 ret
0x5ACFA  CD 11 6D main.asm:33730 call UnknownCall_0x5AD11
0x5ACFD  F0 EC    main.asm:33731 ldh a, [$FFEC]
0x5ACFF  3D       main.asm:33732 dec a
0x5AD00  E0 EC    main.asm:33733 ldh [$FFEC], a
0x5AD02  C0       main.asm:33734 ret nz
0x5AD03  3E 01    main.asm:33735 ld a, 1
0x5AD05  E0 DF    main.asm:33736 ldh [$FFDF], a
0x5AD07  3E 50    main.asm:33737 ld a, 80
0x5AD09  E0 E2    main.asm:33738 ldh [$FFE2], a
0x5AD0B  3E 06    main.asm:33739 ld a, 6
0x5AD0D  EA 78 A4 main.asm:33740 ld [sSfxRequest2], a
0x5AD10  C9       main.asm:33741 ret
0x5AD11  F0 EC    main.asm:33744 ldh a, [$FFEC]
0x5AD13  E6 06    main.asm:33745 and $06
0x5AD15  C8       main.asm:33746 ret z
0x5AD16  3E 01    main.asm:33747 ld a, 1
0x5AD18  EA 3E AF main.asm:33748 ld [sObjectDone], a
0x5AD1B  C9       main.asm:33749 ret
0x5AD1C  CD AD 75 main.asm:33750 call UnknownCall_0x5B5AD
0x5AD1F  06 02    main.asm:33751 ld b, 2
0x5AD21  CD 22 3D main.asm:33752 call UnknownCall_0x3D22
0x5AD24  F0 DA    main.asm:33753 ldh a, [$FFDA]
0x5AD26  FE B0    main.asm:33754 cp $B0
0x5AD28  D8       main.asm:33755 ret c
0x5AD29  CD BF 72 main.asm:33756 call UnknownCall_0x5B2BF
0x5AD2C  C9       main.asm:33757 ret
0x5B2BF  F0 D6    main.asm:34445 ldh a, [$FFD6]
0x5B2C1  67       main.asm:34446 ld h, a
0x5B2C2  F0 D7    main.asm:34447 ldh a, [$FFD7]
0x5B2C4  6F       main.asm:34448 ld l, a
0x5B2C5  2B       main.asm:34449 dec hl
0x5B2C6  2B       main.asm:34450 dec hl
0x5B2C7  11 D0 FF main.asm:34451 ld de, $FFD0
0x5B2CA  06 05    main.asm:34452 ld b, 5
0x5B2CC  CD AA 3C main.asm:34453 call UnknownCall_0x3CAA
0x5B2CF  CD A0 3A main.asm:34454 call UnknownCall_0x3AA0
0x5B2D2  3E 01    main.asm:34455 ld a, 1
0x5B2D4  E0 D8    main.asm:34456 ldh [$FFD8], a
0x5B2D6  C9       main.asm:34457 ret
0x5B5AD  F0 F0    main.asm:34971 ldh a, [$FFF0]
0x5B5AF  E6 03    main.asm:34972 and $03
0x5B5B1  C0       main.asm:34973 ret nz
0x5B5B2  18 07    main.asm:34974 jr UnknownRJump_0x5B5BB
0x5B5BB  21 EA FF main.asm:34985 ld hl, $FFEA
0x5B5BE  34       main.asm:34986 inc [hl]
0x5B5BF  7E       main.asm:34987 ld a, [hl]
0x5B5C0  FE 03    main.asm:34988 cp $03
0x5B5C2  20 02    main.asm:34989 jr nz, UnknownRJump_0x5B5C6
0x5B5C4  AF       main.asm:34990 xor a
0x5B5C5  77       main.asm:34991 ld [hl], a
0x5B5C6  21 D3 75 main.asm:34994 ld hl, $75D3
0x5B5C9  5F       main.asm:34995 ld e, a
0x5B5CA  16 00    main.asm:34996 ld d, 0
0x5B5CC  19       main.asm:34997 add hl, de
0x5B5CD  F0 DB    main.asm:34998 ldh a, [$FFDB]
0x5B5CF  86       main.asm:34999 add [hl]
0x5B5D0  E0 DB    main.asm:35000 ldh [$FFDB], a
0x5B5D2  C9       main.asm:35001 ret
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x5ACE7: $6CED, $6D1C, $6CFA

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
