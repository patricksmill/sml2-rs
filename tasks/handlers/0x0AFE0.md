# Handler 0x0AFE0

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x0AFE0`, bank `$02` (2 decimal), CPU `$6FE0`.
**Selected bank at entry:** `$02`.
**Anims:** boss:27.
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
0x03CB1  FA DD FF home.asm:7880 ldh_a_n $DD
0x03CB4  CB 6F    home.asm:7881 bit 5, a
0x03CB6  3E 01    home.asm:7882 ld a, 1
0x03CB8  28 02    home.asm:7883 jr z, UnknownRJump_0x3CBC
0x03CBA  3E 02    home.asm:7884 ld a, 2
0x03CBC  EA DF FF home.asm:7887 ldh_n_a $DF
0x03CBF  C9       home.asm:7888 ret
0x0AFE0  CD B9 73 main.asm:7633 call UnknownCall_0xB3B9
0x0AFE3  F0 DF    main.asm:7634 ldh a, [$FFDF]
0x0AFE5  EF       main.asm:7635 rst $28
0x0AFEC  3E 40    main.asm:7640 ld a, 64
0x0AFEE  E0 E2    main.asm:7641 ldh [$FFE2], a
0x0AFF0  CD B1 3C main.asm:7642 call UnknownCall_0x3CB1
0x0AFF3  3E 68    main.asm:7643 ld a, 104
0x0AFF5  E0 DB    main.asm:7644 ldh [$FFDB], a
0x0AFF7  AF       main.asm:7645 xor a
0x0AFF8  E0 EA    main.asm:7646 ldh [$FFEA], a
0x0AFFA  06 10    main.asm:7647 ld b, 16
0x0AFFC  CD 70 72 main.asm:7648 call UnknownCall_0xB270
0x0AFFF  C9       main.asm:7649 ret
0x0B000  21 39 70 main.asm:7650 ld hl, $7039
0x0B003  CD 6B 71 main.asm:7651 call UnknownCall_0xB16B
0x0B006  78       main.asm:7652 ld a, b
0x0B007  FE 80    main.asm:7653 cp $80
0x0B009  28 05    main.asm:7654 jr z, UnknownRJump_0xB010
0x0B00B  CD 64 72 main.asm:7655 call UnknownCall_0xB264
0x0B00E  18 13    main.asm:7656 jr UnknownRJump_0xB023
0x0B010  3E 3A    main.asm:7659 ld a, 58
0x0B012  E0 E0    main.asm:7660 ldh [$FFE0], a
0x0B014  C9       main.asm:7661 ret
0x0B015  21 39 70 main.asm:7662 ld hl, $7039
0x0B018  CD 6B 71 main.asm:7663 call UnknownCall_0xB16B
0x0B01B  78       main.asm:7664 ld a, b
0x0B01C  FE 80    main.asm:7665 cp $80
0x0B01E  28 F0    main.asm:7666 jr z, UnknownRJump_0xB010
0x0B020  CD 6A 72 main.asm:7667 call UnknownCall_0xB26A
0x0B023  F0 DA    main.asm:7670 ldh a, [$FFDA]
0x0B025  FE B0    main.asm:7671 cp $B0
0x0B027  D2 57 71 main.asm:7672 jp nc, UnknownJump_0xB157
0x0B02A  21 7E 70 main.asm:7673 ld hl, $707E
0x0B02D  CD 6B 71 main.asm:7674 call UnknownCall_0xB16B
0x0B030  CD 76 72 main.asm:7675 call UnknownCall_0xB276
0x0B033  F0 E0    main.asm:7676 ldh a, [$FFE0]
0x0B035  3C       main.asm:7677 inc a
0x0B036  E0 E0    main.asm:7678 ldh [$FFE0], a
0x0B038  C9       main.asm:7679 ret
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
0x0B16B  F0 E0    main.asm:7805 ldh a, [$FFE0]
0x0B16D  5F       main.asm:7806 ld e, a
0x0B16E  16 00    main.asm:7807 ld d, 0
0x0B170  19       main.asm:7808 add hl, de
0x0B171  46       main.asm:7809 ld b, [hl]
0x0B172  C9       main.asm:7810 ret
0x0B264  F0 DA    main.asm:7953 ldh a, [$FFDA]
0x0B266  90       main.asm:7954 sub b
0x0B267  E0 DA    main.asm:7955 ldh [$FFDA], a
0x0B269  C9       main.asm:7956 ret
0x0B26A  F0 DA    main.asm:7959 ldh a, [$FFDA]
0x0B26C  80       main.asm:7960 add b
0x0B26D  E0 DA    main.asm:7961 ldh [$FFDA], a
0x0B26F  C9       main.asm:7962 ret
0x0B270  F0 D9    main.asm:7965 ldh a, [$FFD9]
0x0B272  90       main.asm:7966 sub b
0x0B273  E0 D9    main.asm:7967 ldh [$FFD9], a
0x0B275  C9       main.asm:7968 ret
0x0B276  F0 D9    main.asm:7971 ldh a, [$FFD9]
0x0B278  80       main.asm:7972 add b
0x0B279  E0 D9    main.asm:7973 ldh [$FFD9], a
0x0B27B  C9       main.asm:7974 ret
0x0B3B9  F0 F0    main.asm:8213 ldh a, [$FFF0]
0x0B3BB  E6 01    main.asm:8214 and $01
0x0B3BD  C0       main.asm:8215 ret nz
0x0B3BE  18 05    main.asm:8216 jr UnknownRJump_0xB3C5
0x0B3C5  21 EA FF main.asm:8224 ld hl, $FFEA
0x0B3C8  34       main.asm:8225 inc [hl]
0x0B3C9  7E       main.asm:8226 ld a, [hl]
0x0B3CA  FE 04    main.asm:8227 cp $04
0x0B3CC  20 20    main.asm:8228 jr nz, UnknownRJump_0xB3EE
0x0B3CE  AF       main.asm:8229 xor a
0x0B3CF  77       main.asm:8230 ld [hl], a
0x0B3D0  F0 DB    main.asm:8231 ldh a, [$FFDB]
0x0B3D2  D6 03    main.asm:8232 sub 3
0x0B3D4  E0 DB    main.asm:8233 ldh [$FFDB], a
0x0B3D6  C9       main.asm:8234 ret
0x0B3EE  F0 DB    main.asm:8253 ldh a, [$FFDB]
0x0B3F0  3C       main.asm:8254 inc a
0x0B3F1  E0 DB    main.asm:8255 ldh [$FFDB], a
0x0B3F3  C9       main.asm:8256 ret
```

## Recovered rst $28 state tables (CPU targets)

- ROM 0x0AFE6: $6FEC, $7000, $7015

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
