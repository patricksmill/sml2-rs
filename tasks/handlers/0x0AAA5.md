# Handler 0x0AAA5

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x0AAA5`, bank `$02` (2 decimal), CPU `$6AA5`.
**Selected bank at entry:** `$02`.
**Anims:** boss:21, boss:24.
**Policy:** [PORTING.md](../../PORTING.md).

Implement in a focused sibling module under `crates/sml2-core/src/objects/`;
register it in `objects/mod.rs` if new, and wire `objects/dispatch.rs`.
Reuse `objects/staging.rs` and `objects/helpers.rs`. New tests belong beside the
implementation; shared integration fixtures are currently in `objects/tests.rs`.

## Exact ROM-addressed instructions and ASM locations

Addresses and edges derive from ROM bytes. Source labels are not assumed to
mark every entry. Review table bounds and dependency contracts before porting.

```text
0x0AAA5  F0 DF    main.asm:6901 ldh a, [$FFDF]
0x0AAA7  EF       main.asm:6902 rst $28
0x0AAB0  3E 01    main.asm:6907 ld a, 1
0x0AAB2  E0 DF    main.asm:6908 ldh [$FFDF], a
0x0AAB4  C9       main.asm:6909 ret
0x0AAB5  CD CA 6A main.asm:6910 call UnknownCall_0xAACA
0x0AAB8  FA CE AF main.asm:6911 ld a, [sBossTransition]
0x0AABB  A7       main.asm:6912 and a
0x0AABC  C8       main.asm:6913 ret z
0x0AABD  AF       main.asm:6914 xor a
0x0AABE  EA CE AF main.asm:6915 ld [sBossTransition], a
0x0AAC1  3E 02    main.asm:6916 ld a, 2
0x0AAC3  E0 DF    main.asm:6917 ldh [$FFDF], a
0x0AAC5  3E 40    main.asm:6918 ld a, 64
0x0AAC7  E0 E2    main.asm:6919 ldh [$FFE2], a
0x0AAC9  C9       main.asm:6920 ret
0x0AACA  F0 F0    main.asm:6923 ldh a, [$FFF0]
0x0AACC  E6 03    main.asm:6924 and $03
0x0AACE  C0       main.asm:6925 ret nz
0x0AACF  06 03    main.asm:6926 ld b, 3
0x0AAD1  F0 E9    main.asm:6927 ldh a, [$FFE9]
0x0AAD3  A7       main.asm:6928 and a
0x0AAD4  20 05    main.asm:6929 jr nz, UnknownRJump_0xAADB
0x0AAD6  CD 64 72 main.asm:6930 call UnknownCall_0xB264
0x0AAD9  18 03    main.asm:6931 jr UnknownRJump_0xAADE
0x0AADB  CD 6A 72 main.asm:6934 call UnknownCall_0xB26A
0x0AADE  F0 DA    main.asm:6937 ldh a, [$FFDA]
0x0AAE0  FE B0    main.asm:6938 cp $B0
0x0AAE2  D8       main.asm:6939 ret c
0x0AAE3  F0 E9    main.asm:6940 ldh a, [$FFE9]
0x0AAE5  EE 01    main.asm:6941 xor $01
0x0AAE7  E0 E9    main.asm:6942 ldh [$FFE9], a
0x0AAE9  C9       main.asm:6943 ret
0x0AAEA  CD 0E 74 main.asm:6944 call UnknownCall_0xB40E
0x0AAED  CD 77 71 main.asm:6945 call UnknownCall_0xB177
0x0AAF0  CD 76 72 main.asm:6946 call UnknownCall_0xB276
0x0AAF3  F0 E0    main.asm:6947 ldh a, [$FFE0]
0x0AAF5  B9       main.asm:6948 cp c
0x0AAF6  C0       main.asm:6949 ret nz
0x0AAF7  AF       main.asm:6950 xor a
0x0AAF8  E0 DE    main.asm:6951 ldh [$FFDE], a
0x0AAFA  E0 E0    main.asm:6952 ldh [$FFE0], a
0x0AAFC  3E 03    main.asm:6953 ld a, 3
0x0AAFE  E0 DF    main.asm:6954 ldh [$FFDF], a
0x0AB00  C9       main.asm:6955 ret
0x0AB01  F0 F0    main.asm:6956 ldh a, [$FFF0]
0x0AB03  E6 07    main.asm:6957 and $07
0x0AB05  C0       main.asm:6958 ret nz
0x0AB06  F0 DB    main.asm:6959 ldh a, [$FFDB]
0x0AB08  3C       main.asm:6960 inc a
0x0AB09  E0 DB    main.asm:6961 ldh [$FFDB], a
0x0AB0B  FE 63    main.asm:6962 cp $63
0x0AB0D  C0       main.asm:6963 ret nz
0x0AB0E  3E 38    main.asm:6964 ld a, 56
0x0AB10  E0 D9    main.asm:6965 ldh [$FFD9], a
0x0AB12  3E 60    main.asm:6966 ld a, 96
0x0AB14  E0 DB    main.asm:6967 ldh [$FFDB], a
0x0AB16  AF       main.asm:6968 xor a
0x0AB17  E0 E2    main.asm:6969 ldh [$FFE2], a
0x0AB19  E0 DF    main.asm:6970 ldh [$FFDF], a
0x0AB1B  E0 E9    main.asm:6971 ldh [$FFE9], a
0x0AB1D  FA 04 FF main.asm:6972 ldh_a_n $04
0x0AB20  E6 01    main.asm:6973 and $01
0x0AB22  28 05    main.asm:6974 jr z, UnknownRJump_0xAB29
0x0AB24  3E A7    main.asm:6975 ld a, 167
0x0AB26  E0 DA    main.asm:6976 ldh [$FFDA], a
0x0AB28  C9       main.asm:6977 ret
0x0AB29  AF       main.asm:6980 xor a
0x0AB2A  E0 DA    main.asm:6981 ldh [$FFDA], a
0x0AB2C  3C       main.asm:6982 inc a
0x0AB2D  E0 E9    main.asm:6983 ldh [$FFE9], a
0x0AB2F  C9       main.asm:6984 ret
0x0B177  0E 24    main.asm:7817 ld c, 36
0x0B179  18 16    main.asm:7818 jr UnknownRJump_0xB191
0x0B191  21 A3 71 main.asm:7844 ld hl, $71A3
0x0B194  F0 E0    main.asm:7845 ldh a, [$FFE0]
0x0B196  5F       main.asm:7846 ld e, a
0x0B197  16 00    main.asm:7847 ld d, 0
0x0B199  19       main.asm:7848 add hl, de
0x0B19A  46       main.asm:7849 ld b, [hl]
0x0B19B  21 E0 FF main.asm:7850 ld hl, $FFE0
0x0B19E  7E       main.asm:7851 ld a, [hl]
0x0B19F  B9       main.asm:7852 cp c
0x0B1A0  C8       main.asm:7853 ret z
0x0B1A1  34       main.asm:7854 inc [hl]
0x0B1A2  C9       main.asm:7855 ret
```

## Recovered rst $28 state tables (CPU targets)

- ROM 0x0AAA8: $6AB0, $6AB5, $6AEA, $6B01

## Existing helper dependencies supplied by the supervisor

- ROM 0x0B264: `crates/sml2-core/src/objects/boss.rs::scr_x_sub` (reviewed bank-preserving contract)
- ROM 0x0B26A: `crates/sml2-core/src/objects/boss.rs::scr_x_add` (reviewed bank-preserving contract)
- ROM 0x0B276: `crates/sml2-core/src/objects/boss.rs::scr_y_add` (reviewed bank-preserving contract)
- ROM 0x0B40E: `crates/sml2-core/src/objects/boss_flash.rs::boss_flash_b40e` (reviewed bank-preserving contract)

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
