# Handler 0x5A0AA

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x5A0AA`, bank `$16` (22 decimal), CPU `$60AA`.
**Selected bank at entry:** `$16`.
**Anims:** b:37.
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
0x03BB8  3E 02    home.asm:7749 ld a, 2
0x03BBA  EA 4E A2 home.asm:7750 ld [sCurROMBank], a
0x03BBD  EA 00 21 home.asm:7751 ld [$2100], a
0x03BC0  CD 3C 44 home.asm:7752 call UnknownCall_0x843C
0x03BC3  3E 16    home.asm:7753 ld a, 22
0x03BC5  EA 4E A2 home.asm:7754 ld [sCurROMBank], a
0x03BC8  EA 00 21 home.asm:7755 ld [$2100], a
0x03BCB  C9       home.asm:7756 ret
0x03C1C  3E 02    home.asm:7799 ld a, 2
0x03C1E  EA 4E A2 home.asm:7800 ld [sCurROMBank], a
0x03C21  EA 00 21 home.asm:7801 ld [$2100], a
0x03C24  CD EA 44 home.asm:7802 call UnknownCall_0x84EA
0x03C27  3E 16    home.asm:7803 ld a, 22
0x03C29  EA 4E A2 home.asm:7804 ld [sCurROMBank], a
0x03C2C  EA 00 21 home.asm:7805 ld [$2100], a
0x03C2F  C9       home.asm:7806 ret
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
0x03D32  21 D1 FF home.asm:7969 ld hl, $FFD1
0x03D35  7E       home.asm:7972 ld a, [hl]
0x03D36  80       home.asm:7973 add b
0x03D37  32       home.asm:7974 ld [hld], a
0x03D38  7E       home.asm:7975 ld a, [hl]
0x03D39  CE 00    home.asm:7976 adc 0
0x03D3B  77       home.asm:7977 ld [hl], a
0x03D3C  C9       home.asm:7978 ret
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
0x0843C  3E 11    main.asm:2775 ld a, 17
0x0843E  EA 2F AF main.asm:2776 ld [sMoveFlags], a
0x08441  06 0B    main.asm:2777 ld b, 11
0x08443  CD 83 45 main.asm:2778 call UnknownCall_0x8583
0x08446  06 07    main.asm:2779 ld b, 7
0x08448  CD 90 45 main.asm:2780 call UnknownCall_0x8590
0x0844B  CD 2F 1F main.asm:2781 call UnknownCall_0x1F2F
0x0844E  FE 38    main.asm:2782 cp $38
0x08450  D8       main.asm:2783 ret c
0x08451  06 05    main.asm:2784 ld b, 5
0x08453  CD 83 45 main.asm:2785 call UnknownCall_0x8583
0x08456  CD 2F 1F main.asm:2786 call UnknownCall_0x1F2F
0x08459  FE 38    main.asm:2787 cp $38
0x0845B  D8       main.asm:2788 ret c
0x0845C  21 2F AF main.asm:2791 ld hl, sMoveFlags
0x0845F  CB 86    main.asm:2792 res 0, [hl]
0x08461  C9       main.asm:2793 ret
0x084EA  3E 22    main.asm:2864 ld a, 34
0x084EC  EA 2F AF main.asm:2865 ld [sMoveFlags], a
0x084EF  06 0B    main.asm:2866 ld b, 11
0x084F1  CD 83 45 main.asm:2867 call UnknownCall_0x8583
0x084F4  06 07    main.asm:2868 ld b, 7
0x084F6  CD 9D 45 main.asm:2869 call UnknownCall_0x859D
0x084F9  CD 2F 1F main.asm:2870 call UnknownCall_0x1F2F
0x084FC  FE 38    main.asm:2871 cp $38
0x084FE  D8       main.asm:2872 ret c
0x084FF  06 05    main.asm:2873 ld b, 5
0x08501  CD 83 45 main.asm:2874 call UnknownCall_0x8583
0x08504  CD 2F 1F main.asm:2875 call UnknownCall_0x1F2F
0x08507  FE 38    main.asm:2876 cp $38
0x08509  D8       main.asm:2877 ret c
0x0850A  21 2F AF main.asm:2880 ld hl, sMoveFlags
0x0850D  CB 8E    main.asm:2881 res 1, [hl]
0x0850F  C9       main.asm:2882 ret
0x08583  21 B7 FF main.asm:2946 ld hl, $FFB7
0x08586  F0 D4    main.asm:2947 ldh a, [$FFD4]
0x08588  90       main.asm:2948 sub b
0x08589  22       main.asm:2949 ld [hli], a
0x0858A  F0 D3    main.asm:2950 ldh a, [$FFD3]
0x0858C  DE 00    main.asm:2951 sbc 0
0x0858E  77       main.asm:2952 ld [hl], a
0x0858F  C9       main.asm:2953 ret
0x08590  21 B9 FF main.asm:2956 ld hl, $FFB9
0x08593  F0 D1    main.asm:2957 ldh a, [$FFD1]
0x08595  80       main.asm:2958 add b
0x08596  22       main.asm:2959 ld [hli], a
0x08597  F0 D0    main.asm:2960 ldh a, [$FFD0]
0x08599  CE 00    main.asm:2961 adc 0
0x0859B  77       main.asm:2962 ld [hl], a
0x0859C  C9       main.asm:2963 ret
0x0859D  21 B9 FF main.asm:2966 ld hl, $FFB9
0x085A0  F0 D1    main.asm:2967 ldh a, [$FFD1]
0x085A2  90       main.asm:2968 sub b
0x085A3  22       main.asm:2969 ld [hli], a
0x085A4  F0 D0    main.asm:2970 ldh a, [$FFD0]
0x085A6  DE 00    main.asm:2971 sbc 0
0x085A8  77       main.asm:2972 ld [hl], a
0x085A9  C9       main.asm:2973 ret
0x582FB  F0 F0    main.asm:27461 ldh a, [$FFF0]
0x582FD  E6 01    main.asm:27462 and $01
0x582FF  20 38    main.asm:27463 jr nz, UnknownRJump_0x58339
0x58301  06 01    main.asm:27464 ld b, 1
0x58303  CD 22 3D main.asm:27465 call UnknownCall_0x3D22
0x58306  CD 1C 3C main.asm:27466 call UnknownCall_0x3C1C
0x58309  FA 2F AF main.asm:27467 ld a, [sMoveFlags]
0x5830C  CB 4F    main.asm:27468 bit 1, a
0x5830E  C8       main.asm:27469 ret z
0x5830F  3E 02    main.asm:27470 ld a, 2
0x58311  E0 DF    main.asm:27471 ldh [$FFDF], a
0x58313  F0 DD    main.asm:27472 ldh a, [$FFDD]
0x58315  EE 20    main.asm:27473 xor $20
0x58317  E0 DD    main.asm:27474 ldh [$FFDD], a
0x58319  C9       main.asm:27475 ret
0x5831A  F0 F0    main.asm:27479 ldh a, [$FFF0]
0x5831C  E6 01    main.asm:27480 and $01
0x5831E  20 19    main.asm:27481 jr nz, UnknownRJump_0x58339
0x58320  06 01    main.asm:27482 ld b, 1
0x58322  CD 32 3D main.asm:27483 call UnknownCall_0x3D32
0x58325  CD B8 3B main.asm:27484 call UnknownCall_0x3BB8
0x58328  FA 2F AF main.asm:27485 ld a, [sMoveFlags]
0x5832B  CB 47    main.asm:27486 bit 0, a
0x5832D  C8       main.asm:27487 ret z
0x5832E  3E 01    main.asm:27488 ld a, 1
0x58330  E0 DF    main.asm:27489 ldh [$FFDF], a
0x58332  F0 DD    main.asm:27490 ldh a, [$FFDD]
0x58334  EE 20    main.asm:27491 xor $20
0x58336  E0 DD    main.asm:27492 ldh [$FFDD], a
0x58338  C9       main.asm:27493 ret
0x58339  CD 40 3B main.asm:27496 call UnknownCall_0x3B40
0x5833C  FA 2F AF main.asm:27497 ld a, [sMoveFlags]
0x5833F  CB 5F    main.asm:27498 bit 3, a
0x58341  C0       main.asm:27499 ret nz
0x58342  F0 DF    main.asm:27500 ldh a, [$FFDF]
0x58344  EE 03    main.asm:27501 xor $03
0x58346  E0 DF    main.asm:27502 ldh [$FFDF], a
0x58348  F0 DD    main.asm:27503 ldh a, [$FFDD]
0x5834A  EE 20    main.asm:27504 xor $20
0x5834C  E0 DD    main.asm:27505 ldh [$FFDD], a
0x5834E  C9       main.asm:27506 ret
0x5A0AA  F0 DF    main.asm:31890 ldh a, [$FFDF]
0x5A0AC  A7       main.asm:31891 and a
0x5A0AD  20 0C    main.asm:31892 jr nz, UnknownRJump_0x5A0BB
0x5A0AF  3E 4D    main.asm:31893 ld a, 77
0x5A0B1  E0 E2    main.asm:31894 ldh [$FFE2], a
0x5A0B3  CD C6 73 main.asm:31895 call UnknownCall_0x5B3C6
0x5A0B6  3E 30    main.asm:31896 ld a, 48
0x5A0B8  E0 E9    main.asm:31897 ldh [$FFE9], a
0x5A0BA  C9       main.asm:31898 ret
0x5A0BB  F0 E9    main.asm:31901 ldh a, [$FFE9]
0x5A0BD  3D       main.asm:31902 dec a
0x5A0BE  E0 E9    main.asm:31903 ldh [$FFE9], a
0x5A0C0  28 13    main.asm:31904 jr z, UnknownRJump_0x5A0D5
0x5A0C2  F0 DF    main.asm:31905 ldh a, [$FFDF]
0x5A0C4  FE 03    main.asm:31906 cp $03
0x5A0C6  DC 85 75 main.asm:31907 call c, UnknownCall_0x5B585
0x5A0C9  F0 DF    main.asm:31908 ldh a, [$FFDF]
0x5A0CB  3D       main.asm:31909 dec a
0x5A0CC  EF       main.asm:31910 rst $28
0x5A0D5  3E 42    main.asm:31917 ld a, 66
0x5A0D7  E0 DB    main.asm:31918 ldh [$FFDB], a
0x5A0D9  3E 45    main.asm:31919 ld a, 69
0x5A0DB  EA 37 AF main.asm:31920 ld [sObjectSpawnSprite], a
0x5A0DE  F0 DD    main.asm:31921 ldh a, [$FFDD]
0x5A0E0  EA 38 AF main.asm:31922 ld [sObjectSpawnAttr], a
0x5A0E3  3E 26    main.asm:31923 ld a, 38
0x5A0E5  EA 36 AF main.asm:31924 ld [sObjectSpawnAnim], a
0x5A0E8  CD 6D 73 main.asm:31925 call UnknownCall_0x5B36D
0x5A0EB  3E 10    main.asm:31926 ld a, 16
0x5A0ED  E0 EC    main.asm:31927 ldh [$FFEC], a
0x5A0EF  F0 DF    main.asm:31928 ldh a, [$FFDF]
0x5A0F1  C6 02    main.asm:31929 add 2
0x5A0F3  E0 DF    main.asm:31930 ldh [$FFDF], a
0x5A0F5  AF       main.asm:31931 xor a
0x5A0F6  E0 EA    main.asm:31932 ldh [$FFEA], a
0x5A0F8  3E 11    main.asm:31933 ld a, 17
0x5A0FA  EA 70 A4 main.asm:31934 ld [sSfxRequest1], a
0x5A0FD  C9       main.asm:31935 ret
0x5A0FE  F0 EC    main.asm:31936 ldh a, [$FFEC]
0x5A100  3D       main.asm:31937 dec a
0x5A101  E0 EC    main.asm:31938 ldh [$FFEC], a
0x5A103  C0       main.asm:31939 ret nz
0x5A104  06 03    main.asm:31940 ld b, 3
0x5A106  F0 DD    main.asm:31941 ldh a, [$FFDD]
0x5A108  A7       main.asm:31942 and a
0x5A109  20 0E    main.asm:31943 jr nz, UnknownRJump_0x5A119
0x5A10B  CD 32 3D main.asm:31944 call UnknownCall_0x3D32
0x5A10E  3E 58    main.asm:31947 ld a, 88
0x5A110  E0 E9    main.asm:31948 ldh [$FFE9], a
0x5A112  F0 DF    main.asm:31949 ldh a, [$FFDF]
0x5A114  D6 02    main.asm:31950 sub 2
0x5A116  E0 DF    main.asm:31951 ldh [$FFDF], a
0x5A118  C9       main.asm:31952 ret
0x5A119  CD 22 3D main.asm:31955 call UnknownCall_0x3D22
0x5A11C  18 F0    main.asm:31956 jr UnknownRJump_0x5A10E
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
0x5B585  F0 F0    main.asm:34937 ldh a, [$FFF0]
0x5B587  E6 01    main.asm:34938 and $01
0x5B589  C0       main.asm:34939 ret nz
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

- ROM 0x5A0CD: $42FB, $431A, $60FE, $60FE

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
