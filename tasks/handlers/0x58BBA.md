# Handler 0x58BBA

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x58BBA`, bank `$16` (22 decimal), CPU `$4BBA`.
**Selected bank at entry:** `$16`.
**Anims:** a:41.
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
0x03D35  7E       home.asm:7972 ld a, [hl]
0x03D36  80       home.asm:7973 add b
0x03D37  32       home.asm:7974 ld [hld], a
0x03D38  7E       home.asm:7975 ld a, [hl]
0x03D39  CE 00    home.asm:7976 adc 0
0x03D3B  77       home.asm:7977 ld [hl], a
0x03D3C  C9       home.asm:7978 ret
0x03D3D  21 D4 FF home.asm:7981 ld hl, $FFD4
0x03D40  18 F3    home.asm:7982 jr UnknownRJump_0x3D35
0x58BBA  F0 DF    main.asm:28801 ldh a, [$FFDF]
0x58BBC  A7       main.asm:28802 and a
0x58BBD  20 1D    main.asm:28803 jr nz, UnknownRJump_0x58BDC
0x58BBF  3E 20    main.asm:28804 ld a, 32
0x58BC1  E0 E2    main.asm:28805 ldh [$FFE2], a
0x58BC3  CD 07 74 main.asm:28806 call UnknownCall_0x5B407
0x58BC6  3E 2C    main.asm:28807 ld a, 44
0x58BC8  E0 E8    main.asm:28808 ldh [$FFE8], a
0x58BCA  C9       main.asm:28809 ret
0x58BDC  3D       main.asm:28821 dec a
0x58BDD  EF       main.asm:28822 rst $28
0x58BE6  F0 E8    main.asm:28827 ldh a, [$FFE8]
0x58BE8  4F       main.asm:28828 ld c, a
0x58BE9  06 01    main.asm:28829 ld b, 1
0x58BEB  CD 2D 3D main.asm:28830 call UnknownCall_0x3D2D
0x58BEE  CD 10 3D main.asm:28831 call UnknownCall_0x3D10
0x58BF1  F0 E0    main.asm:28832 ldh a, [$FFE0]
0x58BF3  3C       main.asm:28833 inc a
0x58BF4  E0 E0    main.asm:28834 ldh [$FFE0], a
0x58BF6  B9       main.asm:28835 cp c
0x58BF7  C0       main.asm:28836 ret nz
0x58BF8  AF       main.asm:28837 xor a
0x58BF9  E0 E0    main.asm:28838 ldh [$FFE0], a
0x58BFB  3E 04    main.asm:28839 ld a, 4
0x58BFD  E0 DF    main.asm:28840 ldh [$FFDF], a
0x58BFF  C9       main.asm:28841 ret
0x58C00  F0 E8    main.asm:28842 ldh a, [$FFE8]
0x58C02  4F       main.asm:28843 ld c, a
0x58C03  06 01    main.asm:28844 ld b, 1
0x58C05  CD 3D 3D main.asm:28845 call UnknownCall_0x3D3D
0x58C08  CD DF 3C main.asm:28846 call UnknownCall_0x3CDF
0x58C0B  F0 E0    main.asm:28847 ldh a, [$FFE0]
0x58C0D  3C       main.asm:28848 inc a
0x58C0E  E0 E0    main.asm:28849 ldh [$FFE0], a
0x58C10  B9       main.asm:28850 cp c
0x58C11  C0       main.asm:28851 ret nz
0x58C12  AF       main.asm:28852 xor a
0x58C13  E0 E0    main.asm:28853 ldh [$FFE0], a
0x58C15  3E 03    main.asm:28854 ld a, 3
0x58C17  E0 DF    main.asm:28855 ldh [$FFDF], a
0x58C19  C9       main.asm:28856 ret
0x58C1A  06 01    main.asm:28857 ld b, 1
0x58C1C  CD 2D 3D main.asm:28858 call UnknownCall_0x3D2D
0x58C1F  CD 10 3D main.asm:28859 call UnknownCall_0x3D10
0x58C22  F0 E8    main.asm:28862 ldh a, [$FFE8]
0x58C24  87       main.asm:28863 add a
0x58C25  4F       main.asm:28864 ld c, a
0x58C26  F0 E0    main.asm:28865 ldh a, [$FFE0]
0x58C28  3C       main.asm:28866 inc a
0x58C29  E0 E0    main.asm:28867 ldh [$FFE0], a
0x58C2B  B9       main.asm:28868 cp c
0x58C2C  C0       main.asm:28869 ret nz
0x58C2D  AF       main.asm:28870 xor a
0x58C2E  E0 E0    main.asm:28871 ldh [$FFE0], a
0x58C30  F0 DF    main.asm:28872 ldh a, [$FFDF]
0x58C32  EE 07    main.asm:28873 xor $07
0x58C34  E0 DF    main.asm:28874 ldh [$FFDF], a
0x58C36  C9       main.asm:28875 ret
0x58C37  06 01    main.asm:28876 ld b, 1
0x58C39  CD 3D 3D main.asm:28877 call UnknownCall_0x3D3D
0x58C3C  CD DF 3C main.asm:28878 call UnknownCall_0x3CDF
0x58C3F  18 E1    main.asm:28879 jr UnknownRJump_0x58C22
0x5B407  CD 1A 74 main.asm:34670 call UnknownCall_0x5B41A
0x5B40A  FA 2C AF main.asm:34671 ld a, [sObjectFacing]
0x5B40D  3D       main.asm:34672 dec a
0x5B40E  28 05    main.asm:34673 jr z, UnknownRJump_0x5B415
0x5B410  3E 01    main.asm:34674 ld a, 1
0x5B412  E0 DF    main.asm:34675 ldh [$FFDF], a
0x5B414  C9       main.asm:34676 ret
0x5B415  3E 02    main.asm:34679 ld a, 2
0x5B417  E0 DF    main.asm:34680 ldh [$FFDF], a
0x5B419  C9       main.asm:34681 ret
0x5B41A  AF       main.asm:34684 xor a
0x5B41B  EA 2C AF main.asm:34685 ld [sObjectFacing], a
0x5B41E  21 D3 FF main.asm:34686 ld hl, $FFD3
0x5B421  2A       main.asm:34687 ld a, [hli]
0x5B422  47       main.asm:34688 ld b, a
0x5B423  4E       main.asm:34689 ld c, [hl]
0x5B424  FA 33 AF main.asm:34690 ld a, [sMarioRefYHi]
0x5B427  B8       main.asm:34691 cp b
0x5B428  38 08    main.asm:34692 jr c, UnknownRJump_0x5B432
0x5B42A  28 0C    main.asm:34693 jr z, UnknownRJump_0x5B438
0x5B42C  3E 01    main.asm:34696 ld a, 1
0x5B42E  EA 2C AF main.asm:34697 ld [sObjectFacing], a
0x5B431  C9       main.asm:34698 ret
0x5B432  3E FF    main.asm:34701 ld a, 255
0x5B434  EA 2C AF main.asm:34702 ld [sObjectFacing], a
0x5B437  C9       main.asm:34703 ret
0x5B438  FA 32 AF main.asm:34706 ld a, [sMarioRefYLo]
0x5B43B  B9       main.asm:34707 cp c
0x5B43C  C8       main.asm:34708 ret z
0x5B43D  38 F3    main.asm:34709 jr c, UnknownRJump_0x5B432
0x5B43F  18 EB    main.asm:34710 jr UnknownRJump_0x5B42C
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x58BDE: $4BE6, $4C00, $4C1A, $4C37

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
