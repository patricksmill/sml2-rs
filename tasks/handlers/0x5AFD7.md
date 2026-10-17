# Handler 0x5AFD7

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x5AFD7`, bank `$16` (22 decimal), CPU `$6FD7`.
**Selected bank at entry:** `$16`.
**Anims:** b:88.
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
0x03D2D  21 D4 FF home.asm:7965 ld hl, $FFD4
0x03D30  18 F3    home.asm:7966 jr UnknownRJump_0x3D25
0x03D32  21 D1 FF home.asm:7969 ld hl, $FFD1
0x03D35  7E       home.asm:7972 ld a, [hl]
0x03D36  80       home.asm:7973 add b
0x03D37  32       home.asm:7974 ld [hld], a
0x03D38  7E       home.asm:7975 ld a, [hl]
0x03D39  CE 00    home.asm:7976 adc 0
0x03D3B  77       home.asm:7977 ld [hl], a
0x03D3C  C9       home.asm:7978 ret
0x03D3D  21 D4 FF home.asm:7981 ld hl, $FFD4
0x03D40  18 F3    home.asm:7982 jr UnknownRJump_0x3D35
0x59D2D  F0 DD    main.asm:31356 ldh a, [$FFDD]
0x59D2F  EE 20    main.asm:31357 xor $20
0x59D31  E0 DD    main.asm:31358 ldh [$FFDD], a
0x59D33  F0 DF    main.asm:31361 ldh a, [$FFDF]
0x59D35  EE 03    main.asm:31362 xor $03
0x59D37  E0 DF    main.asm:31363 ldh [$FFDF], a
0x59D39  C9       main.asm:31364 ret
0x5AFD7  F0 DF    main.asm:34005 ldh a, [$FFDF]
0x5AFD9  EF       main.asm:34006 rst $28
0x5AFE4  3E 4D    main.asm:34011 ld a, 77
0x5AFE6  E0 E2    main.asm:34012 ldh [$FFE2], a
0x5AFE8  CD C6 73 main.asm:34013 call UnknownCall_0x5B3C6
0x5AFEB  3E 01    main.asm:34014 ld a, 1
0x5AFED  E0 E8    main.asm:34015 ldh [$FFE8], a
0x5AFEF  3E 02    main.asm:34016 ld a, 2
0x5AFF1  E0 E9    main.asm:34017 ldh [$FFE9], a
0x5AFF3  C9       main.asm:34018 ret
0x5AFF4  06 01    main.asm:34019 ld b, 1
0x5AFF6  CD 22 3D main.asm:34020 call UnknownCall_0x3D22
0x5AFF9  18 05    main.asm:34021 jr UnknownRJump_0x5B000
0x5AFFB  06 01    main.asm:34022 ld b, 1
0x5AFFD  CD 32 3D main.asm:34023 call UnknownCall_0x3D32
0x5B000  F0 F0    main.asm:34026 ldh a, [$FFF0]
0x5B002  E6 01    main.asm:34027 and $01
0x5B004  C0       main.asm:34028 ret nz
0x5B005  F0 E8    main.asm:34029 ldh a, [$FFE8]
0x5B007  A7       main.asm:34030 and a
0x5B008  20 08    main.asm:34031 jr nz, UnknownRJump_0x5B012
0x5B00A  CD 81 74 main.asm:34032 call UnknownCall_0x5B481
0x5B00D  CD 3D 3D main.asm:34033 call UnknownCall_0x3D3D
0x5B010  18 06    main.asm:34034 jr UnknownRJump_0x5B018
0x5B012  CD 49 74 main.asm:34037 call UnknownCall_0x5B449
0x5B015  CD 2D 3D main.asm:34038 call UnknownCall_0x3D2D
0x5B018  F0 E0    main.asm:34041 ldh a, [$FFE0]
0x5B01A  B9       main.asm:34042 cp c
0x5B01B  C0       main.asm:34043 ret nz
0x5B01C  AF       main.asm:34044 xor a
0x5B01D  E0 E0    main.asm:34045 ldh [$FFE0], a
0x5B01F  F0 E8    main.asm:34046 ldh a, [$FFE8]
0x5B021  EE 01    main.asm:34047 xor $01
0x5B023  E0 E8    main.asm:34048 ldh [$FFE8], a
0x5B025  CA 2D 5D main.asm:34049 jp z, UnknownJump_0x59D2D
0x5B028  F0 E9    main.asm:34050 ldh a, [$FFE9]
0x5B02A  3D       main.asm:34051 dec a
0x5B02B  E0 E9    main.asm:34052 ldh [$FFE9], a
0x5B02D  C0       main.asm:34053 ret nz
0x5B02E  F0 DF    main.asm:34054 ldh a, [$FFDF]
0x5B030  E0 EF    main.asm:34055 ldh [$FFEF], a
0x5B032  3E 03    main.asm:34056 ld a, 3
0x5B034  E0 DF    main.asm:34057 ldh [$FFDF], a
0x5B036  C9       main.asm:34058 ret
0x5B037  F0 F0    main.asm:34059 ldh a, [$FFF0]
0x5B039  E6 03    main.asm:34060 and $03
0x5B03B  C0       main.asm:34061 ret nz
0x5B03C  F0 DB    main.asm:34062 ldh a, [$FFDB]
0x5B03E  3C       main.asm:34063 inc a
0x5B03F  E0 DB    main.asm:34064 ldh [$FFDB], a
0x5B041  FE 70    main.asm:34065 cp $70
0x5B043  C0       main.asm:34066 ret nz
0x5B044  3E 6C    main.asm:34067 ld a, 108
0x5B046  E0 DB    main.asm:34068 ldh [$FFDB], a
0x5B048  3E 70    main.asm:34069 ld a, 112
0x5B04A  EA 37 AF main.asm:34070 ld [sObjectSpawnSprite], a
0x5B04D  3E 59    main.asm:34071 ld a, 89
0x5B04F  EA 36 AF main.asm:34072 ld [sObjectSpawnAnim], a
0x5B052  CD 6D 73 main.asm:34073 call UnknownCall_0x5B36D
0x5B055  3E 04    main.asm:34074 ld a, 4
0x5B057  E0 DF    main.asm:34075 ldh [$FFDF], a
0x5B059  3E 08    main.asm:34076 ld a, 8
0x5B05B  E0 EC    main.asm:34077 ldh [$FFEC], a
0x5B05D  C9       main.asm:34078 ret
0x5B05E  F0 EC    main.asm:34079 ldh a, [$FFEC]
0x5B060  3D       main.asm:34080 dec a
0x5B061  E0 EC    main.asm:34081 ldh [$FFEC], a
0x5B063  C0       main.asm:34082 ret nz
0x5B064  F0 EF    main.asm:34083 ldh a, [$FFEF]
0x5B066  E0 DF    main.asm:34084 ldh [$FFDF], a
0x5B068  3E 02    main.asm:34085 ld a, 2
0x5B06A  E0 E9    main.asm:34086 ldh [$FFE9], a
0x5B06C  C9       main.asm:34087 ret
0x5B36D  21 E8 AC main.asm:34566 ld hl, sObjectScanBase
0x5B370  11 20 00 main.asm:34567 ld de, $0020
0x5B373  19       main.asm:34570 add hl, de
0x5B374  7E       main.asm:34571 ld a, [hl]
0x5B375  A7       main.asm:34572 and a
0x5B376  20 FB    main.asm:34573 jr nz, UnknownRJump_0x5B373
0x5B378  7C       main.asm:34574 ld a, h
0x5B379  FE AF    main.asm:34575 cp $AF
0x5B37B  28 31    main.asm:34576 jr z, UnknownRJump_0x5B3AE
0x5B37D  54       main.asm:34577 ld d, h
0x5B37E  7D       main.asm:34578 ld a, l
0x5B37F  E6 F0    main.asm:34579 and $F0
0x5B381  5F       main.asm:34580 ld e, a
0x5B382  21 D0 FF main.asm:34581 ld hl, $FFD0
0x5B385  06 05    main.asm:34582 ld b, 5
0x5B387  CD AA 3C main.asm:34583 call UnknownCall_0x3CAA
0x5B38A  62       main.asm:34584 ld h, d
0x5B38B  6B       main.asm:34585 ld l, e
0x5B38C  FA 36 AF main.asm:34588 ld a, [sObjectSpawnAnim]
0x5B38F  22       main.asm:34589 ld [hli], a
0x5B390  FA 28 AF main.asm:34590 ld a, [sObjectSlotPtrHi]
0x5B393  22       main.asm:34591 ld [hli], a
0x5B394  FA 29 AF main.asm:34592 ld a, [sObjectSlotPtrLo]
0x5B397  E6 F0    main.asm:34593 and $F0
0x5B399  22       main.asm:34594 ld [hli], a
0x5B39A  3E 01    main.asm:34595 ld a, 1
0x5B39C  77       main.asm:34596 ld [hl], a
0x5B39D  7D       main.asm:34597 ld a, l
0x5B39E  C6 03    main.asm:34598 add 3
0x5B3A0  6F       main.asm:34599 ld l, a
0x5B3A1  FA 37 AF main.asm:34600 ld a, [sObjectSpawnSprite]
0x5B3A4  22       main.asm:34601 ld [hli], a
0x5B3A5  2C       main.asm:34602 inc l
0x5B3A6  FA 38 AF main.asm:34603 ld a, [sObjectSpawnAttr]
0x5B3A9  77       main.asm:34604 ld [hl], a
0x5B3AA  21 40 AF main.asm:34605 ld hl, sStandbyObjectCount
0x5B3AD  34       main.asm:34606 inc [hl]
0x5B3AE  AF       main.asm:34609 xor a
0x5B3AF  EA 36 AF main.asm:34610 ld [sObjectSpawnAnim], a
0x5B3B2  EA 37 AF main.asm:34611 ld [sObjectSpawnSprite], a
0x5B3B5  EA 38 AF main.asm:34612 ld [sObjectSpawnAttr], a
0x5B3B8  C9       main.asm:34613 ret
0x5B3C6  CD E0 73 main.asm:34623 call UnknownCall_0x5B3E0
0x5B3C9  FA 2C AF main.asm:34624 ld a, [sObjectFacing]
0x5B3CC  3D       main.asm:34625 dec a
0x5B3CD  28 08    main.asm:34626 jr z, UnknownRJump_0x5B3D7
0x5B3CF  3E 01    main.asm:34627 ld a, 1
0x5B3D1  E0 DF    main.asm:34628 ldh [$FFDF], a
0x5B3D3  AF       main.asm:34629 xor a
0x5B3D4  E0 DD    main.asm:34630 ldh [$FFDD], a
0x5B3D6  C9       main.asm:34631 ret
0x5B3D7  3E 02    main.asm:34634 ld a, 2
0x5B3D9  E0 DF    main.asm:34635 ldh [$FFDF], a
0x5B3DB  3E 20    main.asm:34636 ld a, 32
0x5B3DD  E0 DD    main.asm:34637 ldh [$FFDD], a
0x5B3DF  C9       main.asm:34638 ret
0x5B3E0  AF       main.asm:34641 xor a
0x5B3E1  EA 2C AF main.asm:34642 ld [sObjectFacing], a
0x5B3E4  21 D0 FF main.asm:34643 ld hl, $FFD0
0x5B3E7  2A       main.asm:34644 ld a, [hli]
0x5B3E8  47       main.asm:34645 ld b, a
0x5B3E9  4E       main.asm:34646 ld c, [hl]
0x5B3EA  FA 31 AF main.asm:34647 ld a, [sMarioRefXHi]
0x5B3ED  B8       main.asm:34648 cp b
0x5B3EE  38 08    main.asm:34649 jr c, UnknownRJump_0x5B3F8
0x5B3F0  28 0C    main.asm:34650 jr z, UnknownRJump_0x5B3FE
0x5B3F2  3E 01    main.asm:34653 ld a, 1
0x5B3F4  EA 2C AF main.asm:34654 ld [sObjectFacing], a
0x5B3F7  C9       main.asm:34655 ret
0x5B3F8  3E FF    main.asm:34658 ld a, 255
0x5B3FA  EA 2C AF main.asm:34659 ld [sObjectFacing], a
0x5B3FD  C9       main.asm:34660 ret
0x5B3FE  FA 30 AF main.asm:34663 ld a, [sMarioRefXLo]
0x5B401  B9       main.asm:34664 cp c
0x5B402  C8       main.asm:34665 ret z
0x5B403  38 F3    main.asm:34666 jr c, UnknownRJump_0x5B3F8
0x5B405  18 EB    main.asm:34667 jr UnknownRJump_0x5B3F2
0x5B449  0E 0F    main.asm:34721 ld c, 15
0x5B44B  18 06    main.asm:34722 jr UnknownRJump_0x5B453
0x5B453  21 88 74 main.asm:34732 ld hl, $7488
0x5B456  F0 E0    main.asm:34735 ldh a, [$FFE0]
0x5B458  5F       main.asm:34736 ld e, a
0x5B459  16 00    main.asm:34737 ld d, 0
0x5B45B  19       main.asm:34738 add hl, de
0x5B45C  46       main.asm:34739 ld b, [hl]
0x5B45D  21 E0 FF main.asm:34740 ld hl, $FFE0
0x5B460  7E       main.asm:34741 ld a, [hl]
0x5B461  B9       main.asm:34742 cp c
0x5B462  C8       main.asm:34743 ret z
0x5B463  34       main.asm:34744 inc [hl]
0x5B464  C9       main.asm:34745 ret
0x5B481  0E 0F    main.asm:34768 ld c, 15
0x5B483  21 A4 74 main.asm:34769 ld hl, $74A4
0x5B486  18 CE    main.asm:34770 jr UnknownRJump_0x5B456
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x5AFDA: $6FE4, $6FF4, $6FFB, $7037, $705E

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
