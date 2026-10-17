# Handler 0x59AAD

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x59AAD`, bank `$16` (22 decimal), CPU `$5AAD`.
**Selected bank at entry:** `$16`.
**Anims:** a:87.
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
0x59AAD  F0 DF    main.asm:31077 ldh a, [$FFDF]
0x59AAF  A7       main.asm:31078 and a
0x59AB0  20 08    main.asm:31079 jr nz, UnknownRJump_0x59ABA
0x59AB2  CD C6 73 main.asm:31080 call UnknownCall_0x5B3C6
0x59AB5  3E 48    main.asm:31081 ld a, 72
0x59AB7  E0 E2    main.asm:31082 ldh [$FFE2], a
0x59AB9  C9       main.asm:31083 ret
0x59ABA  CD 11 75 main.asm:31086 call UnknownCall_0x5B511
0x59ABD  F0 F0    main.asm:31087 ldh a, [$FFF0]
0x59ABF  E6 01    main.asm:31088 and $01
0x59AC1  C0       main.asm:31089 ret nz
0x59AC2  F0 DF    main.asm:31090 ldh a, [$FFDF]
0x59AC4  3D       main.asm:31091 dec a
0x59AC5  EF       main.asm:31092 rst $28
0x59ACA  06 01    main.asm:31097 ld b, 1
0x59ACC  CD 22 3D main.asm:31098 call UnknownCall_0x3D22
0x59ACF  18 05    main.asm:31099 jr UnknownRJump_0x59AD6
0x59AD1  06 01    main.asm:31100 ld b, 1
0x59AD3  CD 32 3D main.asm:31101 call UnknownCall_0x3D32
0x59AD6  F0 E8    main.asm:31104 ldh a, [$FFE8]
0x59AD8  3D       main.asm:31105 dec a
0x59AD9  28 0E    main.asm:31106 jr z, UnknownRJump_0x59AE9
0x59ADB  3D       main.asm:31107 dec a
0x59ADC  28 13    main.asm:31108 jr z, UnknownRJump_0x59AF1
0x59ADE  3D       main.asm:31109 dec a
0x59ADF  28 18    main.asm:31110 jr z, UnknownRJump_0x59AF9
0x59AE1  CD 6C 74 main.asm:31111 call UnknownCall_0x5B46C
0x59AE4  CD 2D 3D main.asm:31112 call UnknownCall_0x3D2D
0x59AE7  18 16    main.asm:31113 jr UnknownRJump_0x59AFF
0x59AE9  CD 45 74 main.asm:31116 call UnknownCall_0x5B445
0x59AEC  CD 3D 3D main.asm:31117 call UnknownCall_0x3D3D
0x59AEF  18 0E    main.asm:31118 jr UnknownRJump_0x59AFF
0x59AF1  CD 6C 74 main.asm:31121 call UnknownCall_0x5B46C
0x59AF4  CD 3D 3D main.asm:31122 call UnknownCall_0x3D3D
0x59AF7  18 06    main.asm:31123 jr UnknownRJump_0x59AFF
0x59AF9  CD 45 74 main.asm:31126 call UnknownCall_0x5B445
0x59AFC  CD 2D 3D main.asm:31127 call UnknownCall_0x3D2D
0x59AFF  F0 E0    main.asm:31130 ldh a, [$FFE0]
0x59B01  B9       main.asm:31131 cp c
0x59B02  C0       main.asm:31132 ret nz
0x59B03  AF       main.asm:31133 xor a
0x59B04  E0 E0    main.asm:31134 ldh [$FFE0], a
0x59B06  F0 E8    main.asm:31135 ldh a, [$FFE8]
0x59B08  3C       main.asm:31136 inc a
0x59B09  E6 03    main.asm:31137 and $03
0x59B0B  E0 E8    main.asm:31138 ldh [$FFE8], a
0x59B0D  C9       main.asm:31139 ret
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
0x5B445  0E 0C    main.asm:34717 ld c, 12
0x5B447  18 0A    main.asm:34718 jr UnknownRJump_0x5B453
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
0x5B46C  0E 0C    main.asm:34753 ld c, 12
0x5B46E  21 A7 74 main.asm:34754 ld hl, $74A7
0x5B471  18 E3    main.asm:34755 jr UnknownRJump_0x5B456
0x5B511  F0 F0    main.asm:34844 ldh a, [$FFF0]
0x5B513  E6 07    main.asm:34845 and $07
0x5B515  C0       main.asm:34846 ret nz
0x5B516  18 0E    main.asm:34847 jr UnknownRJump_0x5B526
0x5B526  F0 DB    main.asm:34864 ldh a, [$FFDB]
0x5B528  EE 01    main.asm:34865 xor $01
0x5B52A  E0 DB    main.asm:34866 ldh [$FFDB], a
0x5B52C  C9       main.asm:34867 ret
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x59AC6: $5ACA, $5AD1

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
