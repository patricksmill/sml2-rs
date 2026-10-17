# Handler 0x5ADDD

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x5ADDD`, bank `$16` (22 decimal), CPU `$6DDD`.
**Selected bank at entry:** `$16`.
**Anims:** b:110.
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
0x5ADDD  CD 7E 75 main.asm:33856 call UnknownCall_0x5B57E
0x5ADE0  F0 DF    main.asm:33857 ldh a, [$FFDF]
0x5ADE2  EF       main.asm:33858 rst $28
0x5ADED  3E 48    main.asm:33863 ld a, 72
0x5ADEF  E0 E2    main.asm:33864 ldh [$FFE2], a
0x5ADF1  CD C6 73 main.asm:33865 call UnknownCall_0x5B3C6
0x5ADF4  3E 10    main.asm:33866 ld a, 16
0x5ADF6  E0 DE    main.asm:33867 ldh [$FFDE], a
0x5ADF8  C9       main.asm:33868 ret
0x5ADF9  21 62 6E main.asm:33869 ld hl, $6E62
0x5ADFC  CD 3E 73 main.asm:33870 call UnknownCall_0x5B33E
0x5ADFF  78       main.asm:33871 ld a, b
0x5AE00  FE 80    main.asm:33872 cp $80
0x5AE02  28 05    main.asm:33873 jr z, UnknownRJump_0x5AE09
0x5AE04  CD 22 3D main.asm:33874 call UnknownCall_0x3D22
0x5AE07  18 12    main.asm:33875 jr UnknownRJump_0x5AE1B
0x5AE09  3E 03    main.asm:33879 ld a, 3
0x5AE0B  18 1B    main.asm:33880 jr UnknownRJump_0x5AE28
0x5AE0D  21 62 6E main.asm:33881 ld hl, $6E62
0x5AE10  CD 3E 73 main.asm:33882 call UnknownCall_0x5B33E
0x5AE13  78       main.asm:33883 ld a, b
0x5AE14  FE 80    main.asm:33884 cp $80
0x5AE16  28 0E    main.asm:33885 jr z, UnknownRJump_0x5AE26
0x5AE18  CD 32 3D main.asm:33886 call UnknownCall_0x3D32
0x5AE1B  21 AC 6E main.asm:33889 ld hl, $6EAC
0x5AE1E  CD 3E 73 main.asm:33892 call UnknownCall_0x5B33E
0x5AE21  CD 2D 3D main.asm:33893 call UnknownCall_0x3D2D
0x5AE24  18 33    main.asm:33894 jr UnknownRJump_0x5AE59
0x5AE26  3E 04    main.asm:33898 ld a, 4
0x5AE28  E0 DF    main.asm:33901 ldh [$FFDF], a
0x5AE2A  AF       main.asm:33902 xor a
0x5AE2B  E0 E9    main.asm:33903 ldh [$FFE9], a
0x5AE2D  C9       main.asm:33904 ret
0x5AE2E  21 87 6E main.asm:33905 ld hl, $6E87
0x5AE31  CD 3E 73 main.asm:33906 call UnknownCall_0x5B33E
0x5AE34  78       main.asm:33907 ld a, b
0x5AE35  FE 80    main.asm:33908 cp $80
0x5AE37  28 05    main.asm:33909 jr z, UnknownRJump_0x5AE3E
0x5AE39  CD 32 3D main.asm:33910 call UnknownCall_0x3D32
0x5AE3C  18 12    main.asm:33911 jr UnknownRJump_0x5AE50
0x5AE3E  3E 02    main.asm:33915 ld a, 2
0x5AE40  18 E6    main.asm:33916 jr UnknownRJump_0x5AE28
0x5AE42  21 87 6E main.asm:33917 ld hl, $6E87
0x5AE45  CD 3E 73 main.asm:33918 call UnknownCall_0x5B33E
0x5AE48  78       main.asm:33919 ld a, b
0x5AE49  FE 80    main.asm:33920 cp $80
0x5AE4B  28 11    main.asm:33921 jr z, UnknownRJump_0x5AE5E
0x5AE4D  CD 22 3D main.asm:33922 call UnknownCall_0x3D22
0x5AE50  21 D0 6E main.asm:33925 ld hl, $6ED0
0x5AE53  CD 3E 73 main.asm:33928 call UnknownCall_0x5B33E
0x5AE56  CD 3D 3D main.asm:33929 call UnknownCall_0x3D3D
0x5AE59  21 E9 FF main.asm:33932 ld hl, $FFE9
0x5AE5C  34       main.asm:33933 inc [hl]
0x5AE5D  C9       main.asm:33934 ret
0x5AE5E  3E 01    main.asm:33938 ld a, 1
0x5AE60  18 C6    main.asm:33939 jr UnknownRJump_0x5AE28
0x5B33E  F0 E9    main.asm:34530 ldh a, [$FFE9]
0x5B340  5F       main.asm:34533 ld e, a
0x5B341  16 00    main.asm:34534 ld d, 0
0x5B343  19       main.asm:34535 add hl, de
0x5B344  46       main.asm:34536 ld b, [hl]
0x5B345  C9       main.asm:34537 ret
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

- ROM 0x5ADE3: $6DED, $6DF9, $6E0D, $6E2E, $6E42

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
