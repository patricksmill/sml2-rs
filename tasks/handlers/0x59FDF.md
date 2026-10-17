# Handler 0x59FDF

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x59FDF`, bank `$16` (22 decimal), CPU `$5FDF`.
**Selected bank at entry:** `$16`.
**Anims:** b:35.
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
0x59FDF  F0 DF    main.asm:31767 ldh a, [$FFDF]
0x59FE1  A7       main.asm:31768 and a
0x59FE2  20 0C    main.asm:31769 jr nz, UnknownRJump_0x59FF0
0x59FE4  3E 4D    main.asm:31770 ld a, 77
0x59FE6  E0 E2    main.asm:31771 ldh [$FFE2], a
0x59FE8  CD C6 73 main.asm:31772 call UnknownCall_0x5B3C6
0x59FEB  3E 18    main.asm:31773 ld a, 24
0x59FED  E0 EC    main.asm:31774 ldh [$FFEC], a
0x59FEF  C9       main.asm:31775 ret
0x59FF0  3D       main.asm:31778 dec a
0x59FF1  EF       main.asm:31779 rst $28
0x59FF8  F0 EC    main.asm:31784 ldh a, [$FFEC]
0x59FFA  A7       main.asm:31785 and a
0x59FFB  28 04    main.asm:31786 jr z, UnknownRJump_0x5A001
0x59FFD  3D       main.asm:31787 dec a
0x59FFE  E0 EC    main.asm:31788 ldh [$FFEC], a
0x5A000  C0       main.asm:31789 ret nz
0x5A001  3E 3E    main.asm:31792 ld a, 62
0x5A003  EA 37 AF main.asm:31793 ld [sObjectSpawnSprite], a
0x5A006  F0 DD    main.asm:31794 ldh a, [$FFDD]
0x5A008  EA 38 AF main.asm:31795 ld [sObjectSpawnAttr], a
0x5A00B  3E 24    main.asm:31796 ld a, 36
0x5A00D  EA 36 AF main.asm:31797 ld [sObjectSpawnAnim], a
0x5A010  CD 6D 73 main.asm:31798 call UnknownCall_0x5B36D
0x5A013  3E 10    main.asm:31799 ld a, 16
0x5A015  E0 EC    main.asm:31800 ldh [$FFEC], a
0x5A017  3E 03    main.asm:31801 ld a, 3
0x5A019  E0 DF    main.asm:31802 ldh [$FFDF], a
0x5A01B  C9       main.asm:31803 ret
0x5A01C  CD 7E 75 main.asm:31804 call UnknownCall_0x5B57E
0x5A01F  F0 EC    main.asm:31805 ldh a, [$FFEC]
0x5A021  3D       main.asm:31806 dec a
0x5A022  E0 EC    main.asm:31807 ldh [$FFEC], a
0x5A024  C0       main.asm:31808 ret nz
0x5A025  3E 3B    main.asm:31809 ld a, 59
0x5A027  E0 DB    main.asm:31810 ldh [$FFDB], a
0x5A029  CD C6 73 main.asm:31811 call UnknownCall_0x5B3C6
0x5A02C  3E 70    main.asm:31812 ld a, 112
0x5A02E  E0 EC    main.asm:31813 ldh [$FFEC], a
0x5A030  C9       main.asm:31814 ret
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
0x5B57E  F0 F0    main.asm:34931 ldh a, [$FFF0]
0x5B580  E6 03    main.asm:34932 and $03
0x5B582  C0       main.asm:34933 ret nz
0x5B583  18 05    main.asm:34934 jr UnknownRJump_0x5B58A
0x5B58A  21 EA FF main.asm:34942 ld hl, $FFEA
0x5B58D  34       main.asm:34943 inc [hl]
0x5B58E  7E       main.asm:34944 ld a, [hl]
0x5B58F  FE 04    main.asm:34945 cp $04
0x5B591  20 02    main.asm:34946 jr nz, UnknownRJump_0x5B595
0x5B593  AF       main.asm:34947 xor a
0x5B594  77       main.asm:34948 ld [hl], a
0x5B595  21 A2 75 main.asm:34951 ld hl, $75A2
0x5B598  5F       main.asm:34952 ld e, a
0x5B599  16 00    main.asm:34953 ld d, 0
0x5B59B  19       main.asm:34954 add hl, de
0x5B59C  F0 DB    main.asm:34955 ldh a, [$FFDB]
0x5B59E  86       main.asm:34956 add [hl]
0x5B59F  E0 DB    main.asm:34957 ldh [$FFDB], a
0x5B5A1  C9       main.asm:34958 ret
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x59FF2: $5FF8, $5FF8, $601C

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
