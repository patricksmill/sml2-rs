# Handler 0x5AC3C

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x5AC3C`, bank `$16` (22 decimal), CPU `$6C3C`.
**Selected bank at entry:** `$16`.
**Anims:** b:102.
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
0x01F2F  F0 B8    home.asm:4064 ldh a, [$FFB8]
0x01F31  E6 0F    home.asm:4065 and $0F
0x01F33  CB 37    home.asm:4066 swap a
0x01F35  67       home.asm:4067 ld h, a
0x01F36  F0 B7    home.asm:4068 ldh a, [$FFB7]
0x01F38  E6 F0    home.asm:4069 and $F0
0x01F3A  CB 37    home.asm:4070 swap a
0x01F3C  B4       home.asm:4071 or h
0x01F3D  C6 B0    home.asm:4072 add 176
0x01F3F  67       home.asm:4073 ld h, a
0x01F40  F0 BA    home.asm:4074 ldh a, [$FFBA]
0x01F42  E6 0F    home.asm:4075 and $0F
0x01F44  CB 37    home.asm:4076 swap a
0x01F46  6F       home.asm:4077 ld l, a
0x01F47  F0 B9    home.asm:4078 ldh a, [$FFB9]
0x01F49  E6 F0    home.asm:4079 and $F0
0x01F4B  CB 37    home.asm:4080 swap a
0x01F4D  B5       home.asm:4081 or l
0x01F4E  6F       home.asm:4082 ld l, a
0x01F4F  7E       home.asm:4083 ld a, [hl]
0x01F50  C9       home.asm:4084 ret
0x03B40  3E 02    home.asm:7689 ld a, 2
0x03B42  EA 4E A2 home.asm:7690 ld [sCurROMBank], a
0x03B45  EA 00 21 home.asm:7691 ld [$2100], a
0x03B48  CD 58 43 home.asm:7692 call UnknownCall_0x8358
0x03B4B  3E 16    home.asm:7693 ld a, 22
0x03B4D  EA 4E A2 home.asm:7694 ld [sCurROMBank], a
0x03B50  EA 00 21 home.asm:7695 ld [$2100], a
0x03B53  C9       home.asm:7696 ret
0x03B90  3E 02    home.asm:7729 ld a, 2
0x03B92  EA 4E A2 home.asm:7730 ld [sCurROMBank], a
0x03B95  EA 00 21 home.asm:7731 ld [$2100], a
0x03B98  CD D0 43 home.asm:7732 call UnknownCall_0x83D0
0x03B9B  3E 16    home.asm:7733 ld a, 22
0x03B9D  EA 4E A2 home.asm:7734 ld [sCurROMBank], a
0x03BA0  EA 00 21 home.asm:7735 ld [$2100], a
0x03BA3  C9       home.asm:7736 ret
0x03D25  7E       home.asm:7956 ld a, [hl]
0x03D26  90       home.asm:7957 sub b
0x03D27  32       home.asm:7958 ld [hld], a
0x03D28  7E       home.asm:7959 ld a, [hl]
0x03D29  DE 00    home.asm:7960 sbc 0
0x03D2B  77       home.asm:7961 ld [hl], a
0x03D2C  C9       home.asm:7962 ret
0x03D2D  21 D4 FF home.asm:7965 ld hl, $FFD4
0x03D30  18 F3    home.asm:7966 jr UnknownRJump_0x3D25
0x03D35  7E       home.asm:7972 ld a, [hl]
0x03D36  80       home.asm:7973 add b
0x03D37  32       home.asm:7974 ld [hld], a
0x03D38  7E       home.asm:7975 ld a, [hl]
0x03D39  CE 00    home.asm:7976 adc 0
0x03D3B  77       home.asm:7977 ld [hl], a
0x03D3C  C9       home.asm:7978 ret
0x03D3D  21 D4 FF home.asm:7981 ld hl, $FFD4
0x03D40  18 F3    home.asm:7982 jr UnknownRJump_0x3D35
0x08358  3E 88    main.asm:2654 ld a, 136
0x0835A  EA 2F AF main.asm:2655 ld [sMoveFlags], a
0x0835D  F0 D4    main.asm:2656 ldh a, [$FFD4]
0x0835F  EA B7 FF main.asm:2657 ldh_n_a $B7
0x08362  E6 0C    main.asm:2658 and $0C
0x08364  20 15    main.asm:2659 jr nz, UnknownRJump_0x837B
0x08366  F0 D3    main.asm:2660 ldh a, [$FFD3]
0x08368  EA B8 FF main.asm:2661 ldh_n_a $B8
0x0836B  F0 D1    main.asm:2664 ldh a, [$FFD1]
0x0836D  EA B9 FF main.asm:2665 ldh_n_a $B9
0x08370  F0 D0    main.asm:2666 ldh a, [$FFD0]
0x08372  EA BA FF main.asm:2667 ldh_n_a $BA
0x08375  CD 2F 1F main.asm:2668 call UnknownCall_0x1F2F
0x08378  FE 48    main.asm:2669 cp $48
0x0837A  D8       main.asm:2670 ret c
0x0837B  21 2F AF main.asm:2673 ld hl, sMoveFlags
0x0837E  CB 9E    main.asm:2674 res 3, [hl]
0x08380  C9       main.asm:2675 ret
0x083CA  21 2F AF main.asm:2715 ld hl, sMoveFlags
0x083CD  CB 96    main.asm:2716 res 2, [hl]
0x083CF  C9       main.asm:2717 ret
0x083D0  3E 44    main.asm:2720 ld a, 68
0x083D2  EA 2F AF main.asm:2721 ld [sMoveFlags], a
0x083D5  06 20    main.asm:2722 ld b, 32
0x083D7  CD 83 45 main.asm:2723 call UnknownCall_0x8583
0x083DA  F0 D1    main.asm:2724 ldh a, [$FFD1]
0x083DC  EA B9 FF main.asm:2725 ldh_n_a $B9
0x083DF  F0 D0    main.asm:2726 ldh a, [$FFD0]
0x083E1  EA BA FF main.asm:2727 ldh_n_a $BA
0x083E4  CD 2F 1F main.asm:2728 call UnknownCall_0x1F2F
0x083E7  FE 38    main.asm:2729 cp $38
0x083E9  D8       main.asm:2730 ret c
0x083EA  18 DE    main.asm:2731 jr UnknownRJump_0x83CA
0x08583  21 B7 FF main.asm:2946 ld hl, $FFB7
0x08586  F0 D4    main.asm:2947 ldh a, [$FFD4]
0x08588  90       main.asm:2948 sub b
0x08589  22       main.asm:2949 ld [hli], a
0x0858A  F0 D3    main.asm:2950 ldh a, [$FFD3]
0x0858C  DE 00    main.asm:2951 sbc 0
0x0858E  77       main.asm:2952 ld [hl], a
0x0858F  C9       main.asm:2953 ret
0x593FB  CD 40 3B main.asm:30085 call UnknownCall_0x3B40
0x593FE  FA 2F AF main.asm:30086 ld a, [sMoveFlags]
0x59401  CB 5F    main.asm:30087 bit 3, a
0x59403  20 08    main.asm:30088 jr nz, UnknownRJump_0x5940D
0x59405  CD 51 74 main.asm:30089 call UnknownCall_0x5B451
0x59408  04       main.asm:30090 inc b
0x59409  CD 3D 3D main.asm:30091 call UnknownCall_0x3D3D
0x5940C  C9       main.asm:30092 ret
0x5940D  AF       main.asm:30095 xor a
0x5940E  E0 E0    main.asm:30096 ldh [$FFE0], a
0x59410  3E 02    main.asm:30097 ld a, 2
0x59412  E0 DF    main.asm:30098 ldh [$FFDF], a
0x59414  3E 0A    main.asm:30099 ld a, 10
0x59416  EA 70 A4 main.asm:30100 ld [sSfxRequest1], a
0x59419  C9       main.asm:30101 ret
0x5AC3C  F0 DF    main.asm:33620 ldh a, [$FFDF]
0x5AC3E  EF       main.asm:33621 rst $28
0x5AC47  3E 10    main.asm:33626 ld a, 16
0x5AC49  E0 DE    main.asm:33627 ldh [$FFDE], a
0x5AC4B  3E 48    main.asm:33628 ld a, 72
0x5AC4D  E0 E2    main.asm:33629 ldh [$FFE2], a
0x5AC4F  3E 03    main.asm:33630 ld a, 3
0x5AC51  E0 DF    main.asm:33631 ldh [$FFDF], a
0x5AC53  C9       main.asm:33632 ret
0x5AC54  CD B4 74 main.asm:33633 call UnknownCall_0x5B4B4
0x5AC57  FE 30    main.asm:33634 cp $30
0x5AC59  D0       main.asm:33635 ret nc
0x5AC5A  3E 01    main.asm:33636 ld a, 1
0x5AC5C  E0 DF    main.asm:33637 ldh [$FFDF], a
0x5AC5E  C9       main.asm:33638 ret
0x5AC5F  CD B6 75 main.asm:33639 call UnknownCall_0x5B5B6
0x5AC62  C3 FB 53 main.asm:33640 jp UnknownJump_0x593FB
0x5AC65  CD B6 75 main.asm:33641 call UnknownCall_0x5B5B6
0x5AC68  CD 90 3B main.asm:33642 call UnknownCall_0x3B90
0x5AC6B  FA 2F AF main.asm:33643 ld a, [sMoveFlags]
0x5AC6E  CB 57    main.asm:33644 bit 2, a
0x5AC70  20 06    main.asm:33645 jr nz, UnknownRJump_0x5AC78
0x5AC72  06 01    main.asm:33646 ld b, 1
0x5AC74  CD 2D 3D main.asm:33647 call UnknownCall_0x3D2D
0x5AC77  C9       main.asm:33648 ret
0x5AC78  3E 03    main.asm:33651 ld a, 3
0x5AC7A  E0 DF    main.asm:33652 ldh [$FFDF], a
0x5AC7C  C9       main.asm:33653 ret
0x5B451  0E 15    main.asm:34729 ld c, 21
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
0x5B4B4  21 D1 FF main.asm:34779 ld hl, $FFD1
0x5B4B7  FA 30 AF main.asm:34780 ld a, [sMarioRefXLo]
0x5B4BA  96       main.asm:34781 sub [hl]
0x5B4BB  CB 7F    main.asm:34782 bit 7, a
0x5B4BD  C8       main.asm:34783 ret z
0x5B4BE  2F       main.asm:34784 cpl
0x5B4BF  3C       main.asm:34785 inc a
0x5B4C0  C9       main.asm:34786 ret
0x5B5B6  F0 F0    main.asm:34980 ldh a, [$FFF0]
0x5B5B8  E6 01    main.asm:34981 and $01
0x5B5BA  C0       main.asm:34982 ret nz
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

- ROM 0x5AC3F: $6C47, $6C5F, $6C65, $6C54

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
