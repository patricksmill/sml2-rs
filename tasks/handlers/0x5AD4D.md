# Handler 0x5AD4D

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x5AD4D`, bank `$16` (22 decimal), CPU `$6D4D`.
**Selected bank at entry:** `$16`.
**Anims:** b:107.
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
0x03CF1  FA E7 FF home.asm:7922 ldh_a_n $E7
0x03CF4  CB 5F    home.asm:7923 bit 3, a
0x03CF6  C8       home.asm:7924 ret z
0x03CF7  21 C2 FF home.asm:7925 ld hl, $FFC2
0x03CFA  FA 00 A2 home.asm:7926 ld a, [sHVelocityIndex]
0x03CFD  FE 81    home.asm:7927 cp $81
0x03CFF  30 07    home.asm:7928 jr nc, UnknownRJump_0x3D08
0x03D01  FA 36 A2 home.asm:7929 ld a, [sScrollDeltaLeft]
0x03D04  80       home.asm:7930 add b
0x03D05  EA 36 A2 home.asm:7931 ld [sScrollDeltaLeft], a
0x03D08  7E       home.asm:7934 ld a, [hl]
0x03D09  90       home.asm:7935 sub b
0x03D0A  22       home.asm:7936 ld [hli], a
0x03D0B  7E       home.asm:7937 ld a, [hl]
0x03D0C  DE 00    home.asm:7938 sbc 0
0x03D0E  77       home.asm:7939 ld [hl], a
0x03D0F  C9       home.asm:7940 ret
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
0x5AD4D  F0 DF    main.asm:33775 ldh a, [$FFDF]
0x5AD4F  EF       main.asm:33776 rst $28
0x5AD5A  3E 20    main.asm:33781 ld a, 32
0x5AD5C  E0 E2    main.asm:33782 ldh [$FFE2], a
0x5AD5E  3E 02    main.asm:33783 ld a, 2
0x5AD60  E0 DF    main.asm:33784 ldh [$FFDF], a
0x5AD62  3E 28    main.asm:33785 ld a, 40
0x5AD64  E0 E9    main.asm:33786 ldh [$FFE9], a
0x5AD66  87       main.asm:33787 add a
0x5AD67  E0 E8    main.asm:33788 ldh [$FFE8], a
0x5AD69  3E 10    main.asm:33789 ld a, 16
0x5AD6B  E0 DE    main.asm:33790 ldh [$FFDE], a
0x5AD6D  3E 01    main.asm:33791 ld a, 1
0x5AD6F  E0 E0    main.asm:33792 ldh [$FFE0], a
0x5AD71  C9       main.asm:33793 ret
0x5AD72  F0 EC    main.asm:33794 ldh a, [$FFEC]
0x5AD74  3D       main.asm:33795 dec a
0x5AD75  E0 EC    main.asm:33796 ldh [$FFEC], a
0x5AD77  C0       main.asm:33797 ret nz
0x5AD78  F0 DF    main.asm:33798 ldh a, [$FFDF]
0x5AD7A  3C       main.asm:33799 inc a
0x5AD7B  E0 DF    main.asm:33800 ldh [$FFDF], a
0x5AD7D  F0 E8    main.asm:33801 ldh a, [$FFE8]
0x5AD7F  E0 E9    main.asm:33802 ldh [$FFE9], a
0x5AD81  AF       main.asm:33803 xor a
0x5AD82  E0 E0    main.asm:33804 ldh [$FFE0], a
0x5AD84  C9       main.asm:33805 ret
0x5AD85  CD DF 75 main.asm:33806 call UnknownCall_0x5B5DF
0x5AD88  FA E4 A2 main.asm:33807 ld a, [sEasyMode]
0x5AD8B  E6 01    main.asm:33808 and $01
0x5AD8D  28 05    main.asm:33809 jr z, UnknownRJump_0x5AD94
0x5AD8F  F0 F0    main.asm:33810 ldh a, [$FFF0]
0x5AD91  E6 01    main.asm:33811 and $01
0x5AD93  C0       main.asm:33812 ret nz
0x5AD94  F0 E0    main.asm:33815 ldh a, [$FFE0]
0x5AD96  EE 01    main.asm:33816 xor $01
0x5AD98  E0 E0    main.asm:33817 ldh [$FFE0], a
0x5AD9A  3C       main.asm:33818 inc a
0x5AD9B  47       main.asm:33819 ld b, a
0x5AD9C  CD 32 3D main.asm:33820 call UnknownCall_0x3D32
0x5AD9F  CD C0 3C main.asm:33821 call UnknownCall_0x3CC0
0x5ADA2  F0 E9    main.asm:33822 ldh a, [$FFE9]
0x5ADA4  90       main.asm:33823 sub b
0x5ADA5  E0 E9    main.asm:33824 ldh [$FFE9], a
0x5ADA7  C0       main.asm:33825 ret nz
0x5ADA8  3E 03    main.asm:33826 ld a, 3
0x5ADAA  E0 DF    main.asm:33827 ldh [$FFDF], a
0x5ADAC  3E 30    main.asm:33828 ld a, 48
0x5ADAE  E0 EC    main.asm:33829 ldh [$FFEC], a
0x5ADB0  C9       main.asm:33830 ret
0x5ADB1  CD DF 75 main.asm:33831 call UnknownCall_0x5B5DF
0x5ADB4  FA E4 A2 main.asm:33832 ld a, [sEasyMode]
0x5ADB7  E6 01    main.asm:33833 and $01
0x5ADB9  28 05    main.asm:33834 jr z, UnknownRJump_0x5ADC0
0x5ADBB  F0 F0    main.asm:33835 ldh a, [$FFF0]
0x5ADBD  E6 01    main.asm:33836 and $01
0x5ADBF  C0       main.asm:33837 ret nz
0x5ADC0  F0 E0    main.asm:33840 ldh a, [$FFE0]
0x5ADC2  EE 01    main.asm:33841 xor $01
0x5ADC4  E0 E0    main.asm:33842 ldh [$FFE0], a
0x5ADC6  3C       main.asm:33843 inc a
0x5ADC7  47       main.asm:33844 ld b, a
0x5ADC8  CD 22 3D main.asm:33845 call UnknownCall_0x3D22
0x5ADCB  CD F1 3C main.asm:33846 call UnknownCall_0x3CF1
0x5ADCE  F0 E9    main.asm:33847 ldh a, [$FFE9]
0x5ADD0  90       main.asm:33848 sub b
0x5ADD1  E0 E9    main.asm:33849 ldh [$FFE9], a
0x5ADD3  C0       main.asm:33850 ret nz
0x5ADD4  3E 01    main.asm:33851 ld a, 1
0x5ADD6  E0 DF    main.asm:33852 ldh [$FFDF], a
0x5ADD8  3E 30    main.asm:33853 ld a, 48
0x5ADDA  E0 EC    main.asm:33854 ldh [$FFEC], a
0x5ADDC  C9       main.asm:33855 ret
0x5B5DF  F0 F0    main.asm:35014 ldh a, [$FFF0]
0x5B5E1  E6 03    main.asm:35015 and $03
0x5B5E3  C0       main.asm:35016 ret nz
0x5B5E4  18 05    main.asm:35017 jr UnknownRJump_0x5B5EB
0x5B5EB  21 EA FF main.asm:35025 ld hl, $FFEA
0x5B5EE  34       main.asm:35026 inc [hl]
0x5B5EF  7E       main.asm:35027 ld a, [hl]
0x5B5F0  FE 04    main.asm:35028 cp $04
0x5B5F2  20 02    main.asm:35029 jr nz, UnknownRJump_0x5B5F6
0x5B5F4  AF       main.asm:35030 xor a
0x5B5F5  77       main.asm:35031 ld [hl], a
0x5B5F6  21 03 76 main.asm:35034 ld hl, $7603
0x5B5F9  5F       main.asm:35035 ld e, a
0x5B5FA  16 00    main.asm:35036 ld d, 0
0x5B5FC  19       main.asm:35037 add hl, de
0x5B5FD  F0 DB    main.asm:35038 ldh a, [$FFDB]
0x5B5FF  86       main.asm:35039 add [hl]
0x5B600  E0 DB    main.asm:35040 ldh [$FFDB], a
0x5B602  C9       main.asm:35041 ret
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x5AD50: $6D5A, $6D72, $6D85, $6D72, $6DB1

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
