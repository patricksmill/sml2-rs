# Handler 0x0A2BE

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x0A2BE`, bank `$02` (2 decimal), CPU `$62BE`.
**Selected bank at entry:** `$02`.
**Anims:** boss:18.
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
0x098F8  3E 04    main.asm:4674 ld a, 4
0x098FA  E0 D5    main.asm:4675 ldh [$FFD5], a
0x098FC  3E 09    main.asm:4676 ld a, 9
0x098FE  E0 DB    main.asm:4677 ldh [$FFDB], a
0x09900  AF       main.asm:4678 xor a
0x09901  E0 DF    main.asm:4679 ldh [$FFDF], a
0x09903  E0 E0    main.asm:4680 ldh [$FFE0], a
0x09905  E0 E2    main.asm:4681 ldh [$FFE2], a
0x09907  C9       main.asm:4682 ret
0x0A2BE  FA 26 AF main.asm:5853 ld a, [sObjectRemoveMode]
0x0A2C1  FE 02    main.asm:5854 cp $02
0x0A2C3  20 0B    main.asm:5855 jr nz, UnknownRJump_0xA2D0
0x0A2C5  3E 80    main.asm:5856 ld a, 128
0x0A2C7  E0 D9    main.asm:5857 ldh [$FFD9], a
0x0A2C9  3E 58    main.asm:5858 ld a, 88
0x0A2CB  E0 DA    main.asm:5859 ldh [$FFDA], a
0x0A2CD  C3 F8 58 main.asm:5860 jp UnknownJump_0x98F8
0x0A2D0  F0 DF    main.asm:5863 ldh a, [$FFDF]
0x0A2D2  A7       main.asm:5864 and a
0x0A2D3  20 0E    main.asm:5865 jr nz, UnknownRJump_0xA2E3
0x0A2D5  F0 DB    main.asm:5866 ldh a, [$FFDB]
0x0A2D7  FE 0E    main.asm:5867 cp $0E
0x0A2D9  C8       main.asm:5868 ret z
0x0A2DA  3E 03    main.asm:5869 ld a, 3
0x0A2DC  E0 DF    main.asm:5870 ldh [$FFDF], a
0x0A2DE  3E 40    main.asm:5871 ld a, 64
0x0A2E0  E0 E2    main.asm:5872 ldh [$FFE2], a
0x0A2E2  C9       main.asm:5873 ret
0x0A2E3  3D       main.asm:5876 dec a
0x0A2E4  EF       main.asm:5877 rst $28
0x0A2F1  CD 8F 71 main.asm:5883 call UnknownCall_0xB18F
0x0A2F4  CD 76 72 main.asm:5884 call UnknownCall_0xB276
0x0A2F7  F0 D9    main.asm:5885 ldh a, [$FFD9]
0x0A2F9  FE 80    main.asm:5886 cp $80
0x0A2FB  D8       main.asm:5887 ret c
0x0A2FC  CD B1 3C main.asm:5888 call UnknownCall_0x3CB1
0x0A2FF  C9       main.asm:5889 ret
0x0A300  F0 F0    main.asm:5890 ldh a, [$FFF0]
0x0A302  E6 01    main.asm:5891 and $01
0x0A304  C0       main.asm:5892 ret nz
0x0A305  06 03    main.asm:5893 ld b, 3
0x0A307  CD 64 72 main.asm:5894 call UnknownCall_0xB264
0x0A30A  18 0A    main.asm:5895 jr UnknownRJump_0xA316
0x0A30C  F0 F0    main.asm:5896 ldh a, [$FFF0]
0x0A30E  E6 01    main.asm:5897 and $01
0x0A310  C0       main.asm:5898 ret nz
0x0A311  06 03    main.asm:5899 ld b, 3
0x0A313  CD 6A 72 main.asm:5900 call UnknownCall_0xB26A
0x0A316  CD DB 72 main.asm:5903 call UnknownCall_0xB2DB
0x0A319  FE 10    main.asm:5904 cp $10
0x0A31B  D0       main.asm:5905 ret nc
0x0A31C  3E 04    main.asm:5906 ld a, 4
0x0A31E  E0 DF    main.asm:5907 ldh [$FFDF], a
0x0A320  C9       main.asm:5908 ret
0x0A321  F0 F0    main.asm:5909 ldh a, [$FFF0]
0x0A323  E6 03    main.asm:5910 and $03
0x0A325  C0       main.asm:5911 ret nz
0x0A326  F0 DB    main.asm:5912 ldh a, [$FFDB]
0x0A328  3C       main.asm:5913 inc a
0x0A329  E0 DB    main.asm:5914 ldh [$FFDB], a
0x0A32B  FE 3E    main.asm:5915 cp $3E
0x0A32D  C0       main.asm:5916 ret nz
0x0A32E  3E 05    main.asm:5917 ld a, 5
0x0A330  E0 DF    main.asm:5918 ldh [$FFDF], a
0x0A332  3E 30    main.asm:5919 ld a, 48
0x0A334  E0 E8    main.asm:5920 ldh [$FFE8], a
0x0A336  C9       main.asm:5921 ret
0x0A337  CD FD 72 main.asm:5922 call UnknownCall_0xB2FD
0x0A33A  06 02    main.asm:5923 ld b, 2
0x0A33C  CD 70 72 main.asm:5924 call UnknownCall_0xB270
0x0A33F  F0 E8    main.asm:5925 ldh a, [$FFE8]
0x0A341  90       main.asm:5926 sub b
0x0A342  E0 E8    main.asm:5927 ldh [$FFE8], a
0x0A344  C0       main.asm:5928 ret nz
0x0A345  3E 06    main.asm:5929 ld a, 6
0x0A347  E0 DF    main.asm:5930 ldh [$FFDF], a
0x0A349  3E 06    main.asm:5931 ld a, 6
0x0A34B  E0 EC    main.asm:5932 ldh [$FFEC], a
0x0A34D  3E 0D    main.asm:5933 ld a, 13
0x0A34F  EA 70 A4 main.asm:5934 ld [sSfxRequest1], a
0x0A352  C9       main.asm:5935 ret
0x0A353  F0 F0    main.asm:5936 ldh a, [$FFF0]
0x0A355  E6 03    main.asm:5937 and $03
0x0A357  C0       main.asm:5938 ret nz
0x0A358  F0 DB    main.asm:5939 ldh a, [$FFDB]
0x0A35A  FE 41    main.asm:5940 cp $41
0x0A35C  28 04    main.asm:5941 jr z, UnknownRJump_0xA362
0x0A35E  3C       main.asm:5942 inc a
0x0A35F  E0 DB    main.asm:5943 ldh [$FFDB], a
0x0A361  C9       main.asm:5944 ret
0x0A362  F0 EC    main.asm:5947 ldh a, [$FFEC]
0x0A364  3D       main.asm:5948 dec a
0x0A365  E0 EC    main.asm:5949 ldh [$FFEC], a
0x0A367  C0       main.asm:5950 ret nz
0x0A368  AF       main.asm:5951 xor a
0x0A369  E0 DF    main.asm:5952 ldh [$FFDF], a
0x0A36B  E0 E2    main.asm:5953 ldh [$FFE2], a
0x0A36D  3E 0E    main.asm:5954 ld a, 14
0x0A36F  E0 DB    main.asm:5955 ldh [$FFDB], a
0x0A371  C9       main.asm:5956 ret
0x0B18F  0E 15    main.asm:7841 ld c, 21
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
0x0B2DB  21 DA FF main.asm:8043 ld hl, $FFDA
0x0B2DE  FA 3C A2 main.asm:8044 ld a, [sMarioScreenX]
0x0B2E1  96       main.asm:8045 sub [hl]
0x0B2E2  CB 7F    main.asm:8046 bit 7, a
0x0B2E4  C8       main.asm:8047 ret z
0x0B2E5  2F       main.asm:8048 cpl
0x0B2E6  3C       main.asm:8049 inc a
0x0B2E7  C9       main.asm:8050 ret
0x0B2FD  F0 F0    main.asm:8071 ldh a, [$FFF0]
0x0B2FF  E6 03    main.asm:8072 and $03
0x0B301  C0       main.asm:8073 ret nz
0x0B302  F0 DB    main.asm:8076 ldh a, [$FFDB]
0x0B304  EE 01    main.asm:8077 xor $01
0x0B306  E0 DB    main.asm:8078 ldh [$FFDB], a
0x0B308  C9       main.asm:8079 ret
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x0A2E5: $6300, $630C, $62F1, $6321, $6337, $6353

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.

## Reviewed native dependency and lifecycle contracts

- Drop helper `$B18F` reads `$B1A3` with cap 21: ROM bytes
  `00 00 01 01 01 02 01 02 02 01 02 03 02 03 03 03 02 03 03 02 03 03`.
  These match the extracted `boss_picker_profile` and independent test data.
  `Actor::motion_step` supplies read-before-increment and equality-only cap.
- `$B2DB` uses the magnitude of the signed wrapping screen-X displacement;
  `ScreenPosition::horizontal_distance_to` supplies this contract. Approach
  tests it after movement, only on even ticks, with a strict distance < 16.
- `$3CB1` selects state 1/2 solely from horizontal flip bit 5. Drop checks
  wrapped Y >= 128 each tick; phase is not reset on initialization or reuse.
- `$B2FD` toggles sprite bit 0 every fourth tick. Rise moves Y and subtracts
  motion distance by two every tick; only exact zero starts the six-count
  terminal timer. Terminal animation increments until the old sprite is 65;
  only later fourth ticks decrement the timer, including wrapping zero.
- Removal mode 2 precedes state dispatch and unconditionally calls shared
  `boss_reward::morph` at screen (88,128). There is no slot-pointer guard,
  damage call, or hit-flag branch. Unknown state bytes still morph on removal.
- The existing anim-16 parent commands this actor in slot 2 before its update
  in the same driver pass. Tests exercise both flip directions, phase reuse,
  live projected Mario X, full lifecycle, and the shared anim-4/5 reward chain.
  Normal definition-4 entry remains gated pending arena lifecycle acceptance.
