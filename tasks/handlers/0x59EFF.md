# Handler 0x59EFF

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x59EFF`, bank `$16` (22 decimal), CPU `$5EFF`.
**Selected bank at entry:** `$16`.
**Anims:** b:33.
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
0x59D33  F0 DF    main.asm:31361 ldh a, [$FFDF]
0x59D35  EE 03    main.asm:31362 xor $03
0x59D37  E0 DF    main.asm:31363 ldh [$FFDF], a
0x59D39  C9       main.asm:31364 ret
0x59EFF  F0 DF    main.asm:31628 ldh a, [$FFDF]
0x59F01  A7       main.asm:31629 and a
0x59F02  20 13    main.asm:31630 jr nz, UnknownRJump_0x59F17
0x59F04  3E 10    main.asm:31631 ld a, 16
0x59F06  E0 DE    main.asm:31632 ldh [$FFDE], a
0x59F08  3E 48    main.asm:31633 ld a, 72
0x59F0A  E0 E2    main.asm:31634 ldh [$FFE2], a
0x59F0C  CD B4 74 main.asm:31635 call UnknownCall_0x5B4B4
0x59F0F  FE 30    main.asm:31636 cp $30
0x59F11  D0       main.asm:31637 ret nc
0x59F12  3E 01    main.asm:31638 ld a, 1
0x59F14  E0 DF    main.asm:31639 ldh [$FFDF], a
0x59F16  C9       main.asm:31640 ret
0x59F17  F0 F0    main.asm:31643 ldh a, [$FFF0]
0x59F19  E6 03    main.asm:31644 and $03
0x59F1B  C0       main.asm:31645 ret nz
0x59F1C  F0 DF    main.asm:31646 ldh a, [$FFDF]
0x59F1E  3D       main.asm:31647 dec a
0x59F1F  EF       main.asm:31648 rst $28
0x59F24  F0 EC    main.asm:31653 ldh a, [$FFEC]
0x59F26  A7       main.asm:31654 and a
0x59F27  28 04    main.asm:31655 jr z, UnknownRJump_0x59F2D
0x59F29  3D       main.asm:31656 dec a
0x59F2A  E0 EC    main.asm:31657 ldh [$FFEC], a
0x59F2C  C9       main.asm:31658 ret
0x59F2D  06 08    main.asm:31661 ld b, 8
0x59F2F  CD 3D 3D main.asm:31662 call UnknownCall_0x3D3D
0x59F32  F0 DB    main.asm:31663 ldh a, [$FFDB]
0x59F34  3C       main.asm:31664 inc a
0x59F35  E0 DB    main.asm:31665 ldh [$FFDB], a
0x59F37  FE 29    main.asm:31666 cp $29
0x59F39  C0       main.asm:31667 ret nz
0x59F3A  3E 08    main.asm:31668 ld a, 8
0x59F3C  E0 EC    main.asm:31669 ldh [$FFEC], a
0x59F3E  C3 33 5D main.asm:31670 jp UnknownJump_0x59D33
0x59F41  F0 EC    main.asm:31671 ldh a, [$FFEC]
0x59F43  A7       main.asm:31672 and a
0x59F44  28 04    main.asm:31673 jr z, UnknownRJump_0x59F4A
0x59F46  3D       main.asm:31674 dec a
0x59F47  E0 EC    main.asm:31675 ldh [$FFEC], a
0x59F49  C9       main.asm:31676 ret
0x59F4A  06 08    main.asm:31679 ld b, 8
0x59F4C  CD 2D 3D main.asm:31680 call UnknownCall_0x3D2D
0x59F4F  F0 DB    main.asm:31681 ldh a, [$FFDB]
0x59F51  3D       main.asm:31682 dec a
0x59F52  E0 DB    main.asm:31683 ldh [$FFDB], a
0x59F54  FE 25    main.asm:31684 cp $25
0x59F56  C0       main.asm:31685 ret nz
0x59F57  3E 08    main.asm:31686 ld a, 8
0x59F59  E0 EC    main.asm:31687 ldh [$FFEC], a
0x59F5B  C3 33 5D main.asm:31688 jp UnknownJump_0x59D33
0x5B4B4  21 D1 FF main.asm:34779 ld hl, $FFD1
0x5B4B7  FA 30 AF main.asm:34780 ld a, [sMarioRefXLo]
0x5B4BA  96       main.asm:34781 sub [hl]
0x5B4BB  CB 7F    main.asm:34782 bit 7, a
0x5B4BD  C8       main.asm:34783 ret z
0x5B4BE  2F       main.asm:34784 cpl
0x5B4BF  3C       main.asm:34785 inc a
0x5B4C0  C9       main.asm:34786 ret
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x59F20: $5F24, $5F41

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
