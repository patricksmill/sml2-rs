# Handler 0x0AB8B

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x0AB8B`, bank `$02` (2 decimal), CPU `$6B8B`.
**Selected bank at entry:** `$02`.
**Anims:** boss:23.
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
0x09725  AF       main.asm:4395 xor a
0x09726  E0 E2    main.asm:4396 ldh [$FFE2], a
0x09728  3E 01    main.asm:4397 ld a, 1
0x0972A  E0 DF    main.asm:4398 ldh [$FFDF], a
0x0972C  C9       main.asm:4399 ret
0x0A920  CD 0E 74 main.asm:6681 call UnknownCall_0xB40E
0x0A923  CD 8F 71 main.asm:6682 call UnknownCall_0xB18F
0x0A926  CD 76 72 main.asm:6683 call UnknownCall_0xB276
0x0A929  F0 D9    main.asm:6684 ldh a, [$FFD9]
0x0A92B  FE 8A    main.asm:6685 cp $8A
0x0A92D  D8       main.asm:6686 ret c
0x0A92E  3E 06    main.asm:6687 ld a, 6
0x0A930  E0 DF    main.asm:6688 ldh [$FFDF], a
0x0A932  3E 80    main.asm:6689 ld a, 128
0x0A934  E0 EC    main.asm:6690 ldh [$FFEC], a
0x0A936  C9       main.asm:6691 ret
0x0A952  FA D0 AF main.asm:6707 ld a, [sBossDelay]
0x0A955  A7       main.asm:6708 and a
0x0A956  C8       main.asm:6709 ret z
0x0A957  CD 21 73 main.asm:6710 call UnknownCall_0xB321
0x0A95A  F0 EC    main.asm:6711 ldh a, [$FFEC]
0x0A95C  3D       main.asm:6712 dec a
0x0A95D  E0 EC    main.asm:6713 ldh [$FFEC], a
0x0A95F  C0       main.asm:6714 ret nz
0x0A960  3E 0B    main.asm:6715 ld a, 11
0x0A962  E0 DF    main.asm:6716 ldh [$FFDF], a
0x0A964  3E 72    main.asm:6717 ld a, 114
0x0A966  E0 DB    main.asm:6718 ldh [$FFDB], a
0x0A968  AF       main.asm:6719 xor a
0x0A969  EA D0 AF main.asm:6720 ld [sBossDelay], a
0x0A96C  C9       main.asm:6721 ret
0x0AB8B  CD 85 52 main.asm:7036 call UnknownCall_0x9285
0x0AB8E  FA 3E AF main.asm:7037 ld a, [sObjectDone]
0x0AB91  A7       main.asm:7038 and a
0x0AB92  28 22    main.asm:7039 jr z, UnknownRJump_0xABB6
0x0AB94  F0 DB    main.asm:7040 ldh a, [$FFDB]
0x0AB96  EA 37 AF main.asm:7041 ld [sObjectSpawnSprite], a
0x0AB99  F0 DF    main.asm:7042 ldh a, [$FFDF]
0x0AB9B  EA C4 AF main.asm:7043 ld [sSavedFrameState], a
0x0AB9E  F0 E2    main.asm:7044 ldh a, [$FFE2]
0x0ABA0  EA CC AF main.asm:7045 ld [sSavedObjectType], a
0x0ABA3  FA C3 AF main.asm:7046 ld a, [sBossHitLatch]
0x0ABA6  A7       main.asm:7047 and a
0x0ABA7  CA 25 57 main.asm:7048 jp z, UnknownJump_0x9725
0x0ABAA  3E 52    main.asm:7049 ld a, 82
0x0ABAC  E0 DB    main.asm:7050 ldh [$FFDB], a
0x0ABAE  FA CD AF main.asm:7051 ld a, [sTimerReload]
0x0ABB1  E0 EC    main.asm:7052 ldh [$FFEC], a
0x0ABB3  C3 25 57 main.asm:7053 jp UnknownJump_0x9725
0x0ABB6  F0 DF    main.asm:7056 ldh a, [$FFDF]
0x0ABB8  A7       main.asm:7057 and a
0x0ABB9  20 45    main.asm:7058 jr nz, UnknownRJump_0xAC00
0x0ABBB  3E 07    main.asm:7059 ld a, 7
0x0ABBD  E0 DF    main.asm:7060 ldh [$FFDF], a
0x0ABBF  3E 20    main.asm:7061 ld a, 32
0x0ABC1  E0 EC    main.asm:7062 ldh [$FFEC], a
0x0ABC3  3E 16    main.asm:7063 ld a, 22
0x0ABC5  EA 36 AF main.asm:7064 ld [sObjectSpawnAnim], a
0x0ABC8  3E 0E    main.asm:7065 ld a, 14
0x0ABCA  EA 37 AF main.asm:7066 ld [sObjectSpawnSprite], a
0x0ABCD  CD 24 72 main.asm:7067 call UnknownCall_0xB224
0x0ABD0  C9       main.asm:7068 ret
0x0ABD1  FA 66 A2 main.asm:7069 ld a, [sFadeTimerLow]
0x0ABD4  FE 61    main.asm:7070 cp $61
0x0ABD6  CC F5 6B main.asm:7071 call z, UnknownCall_0xABF5
0x0ABD9  FA CF AF main.asm:7072 ld a, [sEndingDoorActive]
0x0ABDC  A7       main.asm:7073 and a
0x0ABDD  C4 EF 72 main.asm:7074 call nz, UnknownCall_0xB2EF
0x0ABE0  CD 4E 71 main.asm:7075 call UnknownCall_0xB14E
0x0ABE3  C0       main.asm:7076 ret nz
0x0ABE4  AF       main.asm:7077 xor a
0x0ABE5  EA CF AF main.asm:7078 ld [sEndingDoorActive], a
0x0ABE8  3E 03    main.asm:7079 ld a, 3
0x0ABEA  E0 DF    main.asm:7080 ldh [$FFDF], a
0x0ABEC  3E 41    main.asm:7081 ld a, 65
0x0ABEE  E0 E2    main.asm:7082 ldh [$FFE2], a
0x0ABF0  3E 56    main.asm:7083 ld a, 86
0x0ABF2  E0 DB    main.asm:7084 ldh [$FFDB], a
0x0ABF4  C9       main.asm:7085 ret
0x0ABF5  3E 01    main.asm:7088 ld a, 1
0x0ABF7  EA 70 A4 main.asm:7089 ld [sSfxRequest1], a
0x0ABFA  3E 01    main.asm:7090 ld a, 1
0x0ABFC  EA CF AF main.asm:7091 ld [sEndingDoorActive], a
0x0ABFF  C9       main.asm:7092 ret
0x0AC00  F0 DF    main.asm:7095 ldh a, [$FFDF]
0x0AC02  3D       main.asm:7096 dec a
0x0AC03  EF       main.asm:7097 rst $28
0x0AC1A  3E 05    main.asm:7104 ld a, 5
0x0AC1C  EA 66 A2 main.asm:7105 ld [sFadeTimerLow], a
0x0AC1F  CD 0E 74 main.asm:7106 call UnknownCall_0xB40E
0x0AC22  F0 EC    main.asm:7107 ldh a, [$FFEC]
0x0AC24  3D       main.asm:7108 dec a
0x0AC25  E0 EC    main.asm:7109 ldh [$FFEC], a
0x0AC27  C0       main.asm:7110 ret nz
0x0AC28  AF       main.asm:7111 xor a
0x0AC29  E0 EA    main.asm:7112 ldh [$FFEA], a
0x0AC2B  E0 DE    main.asm:7113 ldh [$FFDE], a
0x0AC2D  3E 08    main.asm:7114 ld a, 8
0x0AC2F  E0 DF    main.asm:7115 ldh [$FFDF], a
0x0AC31  3E 53    main.asm:7116 ld a, 83
0x0AC33  E0 DB    main.asm:7117 ldh [$FFDB], a
0x0AC35  3E 20    main.asm:7118 ld a, 32
0x0AC37  E0 DD    main.asm:7119 ldh [$FFDD], a
0x0AC39  C9       main.asm:7120 ret
0x0ACB2  CD C8 71 main.asm:7187 call UnknownCall_0xB1C8
0x0ACB5  CD 70 72 main.asm:7188 call UnknownCall_0xB270
0x0ACB8  F0 E0    main.asm:7189 ldh a, [$FFE0]
0x0ACBA  B9       main.asm:7190 cp c
0x0ACBB  C0       main.asm:7191 ret nz
0x0ACBC  AF       main.asm:7192 xor a
0x0ACBD  E0 E0    main.asm:7193 ldh [$FFE0], a
0x0ACBF  CD 7C 72 main.asm:7194 call UnknownCall_0xB27C
0x0ACC2  F0 DF    main.asm:7195 ldh a, [$FFDF]
0x0ACC4  C6 03    main.asm:7196 add 3
0x0ACC6  E0 DF    main.asm:7197 ldh [$FFDF], a
0x0ACC8  3E 02    main.asm:7198 ld a, 2
0x0ACCA  EA 78 A4 main.asm:7199 ld [sSfxRequest2], a
0x0ACCD  C9       main.asm:7200 ret
0x0ACCE  CD EF 72 main.asm:7201 call UnknownCall_0xB2EF
0x0ACD1  06 01    main.asm:7202 ld b, 1
0x0ACD3  CD 64 72 main.asm:7203 call UnknownCall_0xB264
0x0ACD6  F0 DA    main.asm:7204 ldh a, [$FFDA]
0x0ACD8  FE 20    main.asm:7205 cp $20
0x0ACDA  38 02    main.asm:7206 jr c, UnknownRJump_0xACDE
0x0ACDC  18 20    main.asm:7207 jr UnknownRJump_0xACFE
0x0ACDE  F0 DF    main.asm:7210 ldh a, [$FFDF]
0x0ACE0  EE 01    main.asm:7211 xor $01
0x0ACE2  E0 DF    main.asm:7212 ldh [$FFDF], a
0x0ACE4  F0 DD    main.asm:7213 ldh a, [$FFDD]
0x0ACE6  EE 20    main.asm:7214 xor $20
0x0ACE8  E0 DD    main.asm:7215 ldh [$FFDD], a
0x0ACEA  3E 02    main.asm:7216 ld a, 2
0x0ACEC  EA 78 A4 main.asm:7217 ld [sSfxRequest2], a
0x0ACEF  C9       main.asm:7218 ret
0x0ACF0  CD EF 72 main.asm:7219 call UnknownCall_0xB2EF
0x0ACF3  06 01    main.asm:7220 ld b, 1
0x0ACF5  CD 6A 72 main.asm:7221 call UnknownCall_0xB26A
0x0ACF8  F0 DA    main.asm:7222 ldh a, [$FFDA]
0x0ACFA  FE 90    main.asm:7223 cp $90
0x0ACFC  30 E0    main.asm:7224 jr nc, UnknownRJump_0xACDE
0x0ACFE  F0 F0    main.asm:7227 ldh a, [$FFF0]
0x0AD00  E6 07    main.asm:7228 and $07
0x0AD02  C0       main.asm:7229 ret nz
0x0AD03  CD DB 72 main.asm:7230 call UnknownCall_0xB2DB
0x0AD06  FE 20    main.asm:7231 cp $20
0x0AD08  30 14    main.asm:7232 jr nc, UnknownRJump_0xAD1E
0x0AD0A  FA 54 A2 main.asm:7233 ld a, [sTimerLow]
0x0AD0D  E6 03    main.asm:7234 and $03
0x0AD0F  20 0D    main.asm:7235 jr nz, UnknownRJump_0xAD1E
0x0AD11  3E 0A    main.asm:7236 ld a, 10
0x0AD13  E0 DF    main.asm:7237 ldh [$FFDF], a
0x0AD15  3E 56    main.asm:7238 ld a, 86
0x0AD17  E0 DB    main.asm:7239 ldh [$FFDB], a
0x0AD19  3E 20    main.asm:7240 ld a, 32
0x0AD1B  E0 EC    main.asm:7241 ldh [$FFEC], a
0x0AD1D  C9       main.asm:7242 ret
0x0AD1E  06 01    main.asm:7245 ld b, 1
0x0AD20  CD 76 72 main.asm:7246 call UnknownCall_0xB276
0x0AD23  F0 D9    main.asm:7247 ldh a, [$FFD9]
0x0AD25  FE 8A    main.asm:7248 cp $8A
0x0AD27  D8       main.asm:7249 ret c
0x0AD28  3E 03    main.asm:7252 ld a, 3
0x0AD2A  E0 DF    main.asm:7253 ldh [$FFDF], a
0x0AD2C  3E 56    main.asm:7254 ld a, 86
0x0AD2E  E0 DB    main.asm:7255 ldh [$FFDB], a
0x0AD30  C9       main.asm:7256 ret
0x0AD31  F0 EC    main.asm:7257 ldh a, [$FFEC]
0x0AD33  A7       main.asm:7258 and a
0x0AD34  28 04    main.asm:7259 jr z, UnknownRJump_0xAD3A
0x0AD36  3D       main.asm:7260 dec a
0x0AD37  E0 EC    main.asm:7261 ldh [$FFEC], a
0x0AD39  C9       main.asm:7262 ret
0x0AD3A  CD 8F 71 main.asm:7265 call UnknownCall_0xB18F
0x0AD3D  CD 76 72 main.asm:7266 call UnknownCall_0xB276
0x0AD40  F0 D9    main.asm:7267 ldh a, [$FFD9]
0x0AD42  FE 8A    main.asm:7268 cp $8A
0x0AD44  D8       main.asm:7269 ret c
0x0AD45  AF       main.asm:7270 xor a
0x0AD46  E0 E0    main.asm:7271 ldh [$FFE0], a
0x0AD48  3E 09    main.asm:7272 ld a, 9
0x0AD4A  E0 DF    main.asm:7273 ldh [$FFDF], a
0x0AD4C  3E 38    main.asm:7274 ld a, 56
0x0AD4E  EA B3 A2 main.asm:7275 ld [sCameraPanTimer], a
0x0AD51  3E 30    main.asm:7276 ld a, 48
0x0AD53  E0 EC    main.asm:7277 ldh [$FFEC], a
0x0AD55  3E 01    main.asm:7278 ld a, 1
0x0AD57  EA CE AF main.asm:7279 ld [sBossTransition], a
0x0AD5A  3E 18    main.asm:7280 ld a, 24
0x0AD5C  EA 60 A4 main.asm:7281 ld [sSfxRequest0], a
0x0AD5F  C9       main.asm:7282 ret
0x0AD60  F0 EC    main.asm:7283 ldh a, [$FFEC]
0x0AD62  A7       main.asm:7284 and a
0x0AD63  28 04    main.asm:7285 jr z, UnknownRJump_0xAD69
0x0AD65  3D       main.asm:7286 dec a
0x0AD66  E0 EC    main.asm:7287 ldh [$FFEC], a
0x0AD68  C9       main.asm:7288 ret
0x0AD69  F0 E8    main.asm:7291 ldh a, [$FFE8]
0x0AD6B  A7       main.asm:7292 and a
0x0AD6C  20 08    main.asm:7293 jr nz, UnknownRJump_0xAD76
0x0AD6E  CD E4 71 main.asm:7294 call UnknownCall_0xB1E4
0x0AD71  CD 70 72 main.asm:7295 call UnknownCall_0xB270
0x0AD74  18 06    main.asm:7296 jr UnknownRJump_0xAD7C
0x0AD76  CD 83 71 main.asm:7299 call UnknownCall_0xB183
0x0AD79  CD 76 72 main.asm:7300 call UnknownCall_0xB276
0x0AD7C  F0 E0    main.asm:7303 ldh a, [$FFE0]
0x0AD7E  B9       main.asm:7304 cp c
0x0AD7F  C0       main.asm:7305 ret nz
0x0AD80  AF       main.asm:7306 xor a
0x0AD81  E0 E0    main.asm:7307 ldh [$FFE0], a
0x0AD83  E0 EA    main.asm:7308 ldh [$FFEA], a
0x0AD85  3E 53    main.asm:7309 ld a, 83
0x0AD87  E0 DB    main.asm:7310 ldh [$FFDB], a
0x0AD89  F0 E8    main.asm:7311 ldh a, [$FFE8]
0x0AD8B  EE 01    main.asm:7312 xor $01
0x0AD8D  E0 E8    main.asm:7313 ldh [$FFE8], a
0x0AD8F  C0       main.asm:7314 ret nz
0x0AD90  C3 28 6D main.asm:7315 jp UnknownJump_0xAD28
0x0B183  0E 0C    main.asm:7829 ld c, 12
0x0B185  18 0A    main.asm:7830 jr UnknownRJump_0xB191
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
0x0B27C  CD 96 72 main.asm:7977 call UnknownCall_0xB296
0x0B27F  FA 2C AF main.asm:7978 ld a, [sObjectFacing]
0x0B282  3D       main.asm:7979 dec a
0x0B283  28 08    main.asm:7980 jr z, UnknownRJump_0xB28D
0x0B285  3E 01    main.asm:7981 ld a, 1
0x0B287  E0 DF    main.asm:7982 ldh [$FFDF], a
0x0B289  AF       main.asm:7983 xor a
0x0B28A  E0 DD    main.asm:7984 ldh [$FFDD], a
0x0B28C  C9       main.asm:7985 ret
0x0B28D  3E 02    main.asm:7988 ld a, 2
0x0B28F  E0 DF    main.asm:7989 ldh [$FFDF], a
0x0B291  3E 20    main.asm:7990 ld a, 32
0x0B293  E0 DD    main.asm:7991 ldh [$FFDD], a
0x0B295  C9       main.asm:7992 ret
0x0B296  AF       main.asm:7995 xor a
0x0B297  EA 2C AF main.asm:7996 ld [sObjectFacing], a
0x0B29A  F0 DA    main.asm:7997 ldh a, [$FFDA]
0x0B29C  47       main.asm:7998 ld b, a
0x0B29D  FA 3C A2 main.asm:7999 ld a, [sMarioScreenX]
0x0B2A0  B8       main.asm:8000 cp b
0x0B2A1  38 06    main.asm:8001 jr c, UnknownRJump_0xB2A9
0x0B2A3  3E 01    main.asm:8002 ld a, 1
0x0B2A5  EA 2C AF main.asm:8003 ld [sObjectFacing], a
0x0B2A8  C9       main.asm:8004 ret
0x0B2A9  3E FF    main.asm:8007 ld a, 255
0x0B2AB  EA 2C AF main.asm:8008 ld [sObjectFacing], a
0x0B2AE  C9       main.asm:8009 ret
0x0B2DB  21 DA FF main.asm:8043 ld hl, $FFDA
0x0B2DE  FA 3C A2 main.asm:8044 ld a, [sMarioScreenX]
0x0B2E1  96       main.asm:8045 sub [hl]
0x0B2E2  CB 7F    main.asm:8046 bit 7, a
0x0B2E4  C8       main.asm:8047 ret z
0x0B2E5  2F       main.asm:8048 cpl
0x0B2E6  3C       main.asm:8049 inc a
0x0B2E7  C9       main.asm:8050 ret
0x0B2EF  F0 F0    main.asm:8059 ldh a, [$FFF0]
0x0B2F1  E6 07    main.asm:8060 and $07
0x0B2F3  C0       main.asm:8061 ret nz
0x0B2F4  18 0C    main.asm:8062 jr UnknownRJump_0xB302
0x0B302  F0 DB    main.asm:8076 ldh a, [$FFDB]
0x0B304  EE 01    main.asm:8077 xor $01
0x0B306  E0 DB    main.asm:8078 ldh [$FFDB], a
0x0B308  C9       main.asm:8079 ret
0x0B321  F0 F0    main.asm:8100 ldh a, [$FFF0]
0x0B323  E6 07    main.asm:8101 and $07
0x0B325  C0       main.asm:8102 ret nz
0x0B326  F0 DB    main.asm:8103 ldh a, [$FFDB]
0x0B328  EE 1F    main.asm:8104 xor $1F
0x0B32A  E0 DB    main.asm:8105 ldh [$FFDB], a
0x0B32C  C9       main.asm:8106 ret
```

## Recovered rst $28 state tables (CPU targets)

- ROM 0x0AC04: $5CAC, $6920, $6CB2, $6CCE, $6CF0, $6C1A, $6952, $6C55, $6D60, $6D31, $6BD1

## Existing helper dependencies supplied by the supervisor

- ROM 0x09285: `crates/sml2-core/src/objects/boss.rs::boss_damage_pass` (reviewed bank-preserving contract)
- ROM 0x09CAC: `crates/sml2-core/src/objects/boss.rs::boss_hit_stun` (reviewed bank-preserving contract)
- ROM 0x0AC55: `crates/sml2-core/src/objects/wario/phase2_exit.rs::wario_phase2_exit` (reviewed bank-preserving contract)
- ROM 0x0B14E: `crates/sml2-core/src/objects/boss.rs::fade_idle` (reviewed bank-preserving contract)
- ROM 0x0B1C8: `crates/sml2-core/src/objects/boss_reverse_picker.rs::boss_reverse_picker_b1c8` (reviewed bank-preserving contract)
- ROM 0x0B1E4: `crates/sml2-core/src/objects/boss_reverse_picker.rs::boss_reverse_picker_b1e4` (reviewed bank-preserving contract)
- ROM 0x0B224: `crates/sml2-core/src/objects/boss.rs::boss_spawn_child` (reviewed bank-preserving contract)
- ROM 0x0B40E: `crates/sml2-core/src/objects/boss_flash.rs::boss_flash_b40e` (reviewed bank-preserving contract)

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
