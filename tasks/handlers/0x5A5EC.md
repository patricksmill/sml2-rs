# Handler 0x5A5EC

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x5A5EC`, bank `$16` (22 decimal), CPU `$65EC`.
**Selected bank at entry:** `$16`.
**Anims:** b:71.
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
0x03CC0  FA E7 FF home.asm:7891 ldh_a_n $E7
0x03CC3  CB 5F    home.asm:7892 bit 3, a
0x03CC5  C8       home.asm:7893 ret z
0x03CC6  21 C2 FF home.asm:7894 ld hl, $FFC2
0x03CC9  FA 00 A2 home.asm:7895 ld a, [sHVelocityIndex]
0x03CCC  FE 80    home.asm:7896 cp $80
0x03CCE  38 07    home.asm:7897 jr c, UnknownRJump_0x3CD7
0x03CD0  FA 35 A2 home.asm:7898 ld a, [sScrollDeltaRight]
0x03CD3  80       home.asm:7899 add b
0x03CD4  EA 35 A2 home.asm:7900 ld [sScrollDeltaRight], a
0x03CD7  7E       home.asm:7903 ld a, [hl]
0x03CD8  80       home.asm:7904 add b
0x03CD9  22       home.asm:7905 ld [hli], a
0x03CDA  7E       home.asm:7906 ld a, [hl]
0x03CDB  CE 00    home.asm:7907 adc 0
0x03CDD  77       home.asm:7908 ld [hl], a
0x03CDE  C9       home.asm:7909 ret
0x03CDF  FA E7 FF home.asm:7912 ldh_a_n $E7
0x03CE2  CB 5F    home.asm:7913 bit 3, a
0x03CE4  C8       home.asm:7914 ret z
0x03CE5  FA 38 A2 home.asm:7915 ld a, [sScrollDeltaDown]
0x03CE8  80       home.asm:7916 add b
0x03CE9  EA 38 A2 home.asm:7917 ld [sScrollDeltaDown], a
0x03CEC  21 C0 FF home.asm:7918 ld hl, $FFC0
0x03CEF  18 E6    home.asm:7919 jr UnknownRJump_0x3CD7
0x03D08  7E       home.asm:7934 ld a, [hl]
0x03D09  90       home.asm:7935 sub b
0x03D0A  22       home.asm:7936 ld [hli], a
0x03D0B  7E       home.asm:7937 ld a, [hl]
0x03D0C  DE 00    home.asm:7938 sbc 0
0x03D0E  77       home.asm:7939 ld [hl], a
0x03D0F  C9       home.asm:7940 ret
0x03D10  FA E7 FF home.asm:7943 ldh_a_n $E7
0x03D13  CB 5F    home.asm:7944 bit 3, a
0x03D15  C8       home.asm:7945 ret z
0x03D16  FA 37 A2 home.asm:7946 ld a, [sScrollDeltaUp]
0x03D19  80       home.asm:7947 add b
0x03D1A  EA 37 A2 home.asm:7948 ld [sScrollDeltaUp], a
0x03D1D  21 C0 FF home.asm:7949 ld hl, $FFC0
0x03D20  18 E6    home.asm:7950 jr UnknownRJump_0x3D08
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
0x5A5EC  F0 DF    main.asm:32676 ldh a, [$FFDF]
0x5A5EE  A7       main.asm:32677 and a
0x5A5EF  20 12    main.asm:32678 jr nz, UnknownRJump_0x5A603
0x5A5F1  3E 10    main.asm:32679 ld a, 16
0x5A5F3  E0 DE    main.asm:32680 ldh [$FFDE], a
0x5A5F5  3E 20    main.asm:32681 ld a, 32
0x5A5F7  E0 E2    main.asm:32682 ldh [$FFE2], a
0x5A5F9  F0 E7    main.asm:32683 ldh a, [$FFE7]
0x5A5FB  CB 5F    main.asm:32684 bit 3, a
0x5A5FD  C8       main.asm:32685 ret z
0x5A5FE  3E 01    main.asm:32686 ld a, 1
0x5A600  E0 DF    main.asm:32687 ldh [$FFDF], a
0x5A602  C9       main.asm:32688 ret
0x5A603  F0 DF    main.asm:32691 ldh a, [$FFDF]
0x5A605  3D       main.asm:32692 dec a
0x5A606  EF       main.asm:32693 rst $28
0x5A60F  F0 F0    main.asm:32698 ldh a, [$FFF0]
0x5A611  E6 01    main.asm:32699 and $01
0x5A613  C0       main.asm:32700 ret nz
0x5A614  06 01    main.asm:32701 ld b, 1
0x5A616  CD 2D 3D main.asm:32702 call UnknownCall_0x3D2D
0x5A619  CD 10 3D main.asm:32703 call UnknownCall_0x3D10
0x5A61C  CD 32 3D main.asm:32704 call UnknownCall_0x3D32
0x5A61F  CD C0 3C main.asm:32705 call UnknownCall_0x3CC0
0x5A622  F0 E9    main.asm:32706 ldh a, [$FFE9]
0x5A624  80       main.asm:32707 add b
0x5A625  E0 E9    main.asm:32708 ldh [$FFE9], a
0x5A627  FE 30    main.asm:32709 cp $30
0x5A629  C0       main.asm:32710 ret nz
0x5A62A  AF       main.asm:32711 xor a
0x5A62B  E0 E9    main.asm:32712 ldh [$FFE9], a
0x5A62D  F0 DF    main.asm:32713 ldh a, [$FFDF]
0x5A62F  3C       main.asm:32714 inc a
0x5A630  E0 DF    main.asm:32715 ldh [$FFDF], a
0x5A632  C9       main.asm:32716 ret
0x5A633  F0 F0    main.asm:32717 ldh a, [$FFF0]
0x5A635  E6 01    main.asm:32718 and $01
0x5A637  C0       main.asm:32719 ret nz
0x5A638  06 01    main.asm:32720 ld b, 1
0x5A63A  CD 3D 3D main.asm:32721 call UnknownCall_0x3D3D
0x5A63D  CD DF 3C main.asm:32722 call UnknownCall_0x3CDF
0x5A640  CD 32 3D main.asm:32723 call UnknownCall_0x3D32
0x5A643  CD C0 3C main.asm:32724 call UnknownCall_0x3CC0
0x5A646  F0 E9    main.asm:32725 ldh a, [$FFE9]
0x5A648  80       main.asm:32726 add b
0x5A649  E0 E9    main.asm:32727 ldh [$FFE9], a
0x5A64B  FE 20    main.asm:32728 cp $20
0x5A64D  C0       main.asm:32729 ret nz
0x5A64E  AF       main.asm:32730 xor a
0x5A64F  E0 E9    main.asm:32731 ldh [$FFE9], a
0x5A651  3E 03    main.asm:32732 ld a, 3
0x5A653  E0 DF    main.asm:32733 ldh [$FFDF], a
0x5A655  3E 40    main.asm:32734 ld a, 64
0x5A657  E0 EC    main.asm:32735 ldh [$FFEC], a
0x5A659  C9       main.asm:32736 ret
0x5A65A  CD 8B 76 main.asm:32737 call UnknownCall_0x5B68B
0x5A65D  F0 EC    main.asm:32738 ldh a, [$FFEC]
0x5A65F  3D       main.asm:32739 dec a
0x5A660  E0 EC    main.asm:32740 ldh [$FFEC], a
0x5A662  C0       main.asm:32741 ret nz
0x5A663  CD BF 72 main.asm:32743 call UnknownCall_0x5B2BF
0x5A666  C9       main.asm:32744 ret
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
0x5B68B  F0 F0    main.asm:35138 ldh a, [$FFF0]
0x5B68D  E6 01    main.asm:35139 and $01
0x5B68F  C0       main.asm:35140 ret nz
0x5B690  F0 DE    main.asm:35142 ldh a, [$FFDE]
0x5B692  EE 10    main.asm:35143 xor $10
0x5B694  E0 DE    main.asm:35144 ldh [$FFDE], a
0x5B696  C9       main.asm:35145 ret
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x5A607: $660F, $6633, $660F, $665A

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
