# Handler 0x5AAB9

**Slice status: COMPLETE** — this describes control-flow coverage, not implementation verification.
**Address:** ROM `0x5AAB9`, bank `$16` (22 decimal), CPU `$6AB9`.
**Selected bank at entry:** `$16`.
**Anims:** b:98.
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
0x03D22  21 D1 FF home.asm:7953 ld hl, $FFD1
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
0x5AAB9  F0 DF    main.asm:33395 ldh a, [$FFDF]
0x5AABB  EF       main.asm:33396 rst $28
0x5AAC6  3E 10    main.asm:33401 ld a, 16
0x5AAC8  E0 DE    main.asm:33402 ldh [$FFDE], a
0x5AACA  3E 20    main.asm:33403 ld a, 32
0x5AACC  E0 E2    main.asm:33404 ldh [$FFE2], a
0x5AACE  3E 03    main.asm:33405 ld a, 3
0x5AAD0  E0 DF    main.asm:33406 ldh [$FFDF], a
0x5AAD2  3E 30    main.asm:33407 ld a, 48
0x5AAD4  E0 E8    main.asm:33408 ldh [$FFE8], a
0x5AAD6  E0 E9    main.asm:33409 ldh [$FFE9], a
0x5AAD8  C9       main.asm:33410 ret
0x5AAD9  F0 E7    main.asm:33411 ldh a, [$FFE7]
0x5AADB  CB 5F    main.asm:33412 bit 3, a
0x5AADD  C8       main.asm:33413 ret z
0x5AADE  06 08    main.asm:33414 ld b, 8
0x5AAE0  CD 3D 3D main.asm:33415 call UnknownCall_0x3D3D
0x5AAE3  3E DD    main.asm:33416 ld a, 221
0x5AAE5  E0 DB    main.asm:33417 ldh [$FFDB], a
0x5AAE7  3E 04    main.asm:33418 ld a, 4
0x5AAE9  E0 DF    main.asm:33419 ldh [$FFDF], a
0x5AAEB  3E 04    main.asm:33420 ld a, 4
0x5AAED  E0 EC    main.asm:33421 ldh [$FFEC], a
0x5AAEF  C9       main.asm:33422 ret
0x5AAF0  F0 EC    main.asm:33423 ldh a, [$FFEC]
0x5AAF2  3D       main.asm:33424 dec a
0x5AAF3  E0 EC    main.asm:33425 ldh [$FFEC], a
0x5AAF5  C0       main.asm:33426 ret nz
0x5AAF6  3E 80    main.asm:33427 ld a, 128
0x5AAF8  E0 DC    main.asm:33428 ldh [$FFDC], a
0x5AAFA  06 40    main.asm:33429 ld b, 64
0x5AAFC  CD 22 3D main.asm:33430 call UnknownCall_0x3D22
0x5AAFF  CD 2D 3D main.asm:33431 call UnknownCall_0x3D2D
0x5AB02  3E C1    main.asm:33432 ld a, 193
0x5AB04  E0 DB    main.asm:33433 ldh [$FFDB], a
0x5AB06  3E 01    main.asm:33434 ld a, 1
0x5AB08  E0 DF    main.asm:33435 ldh [$FFDF], a
0x5AB0A  3E 48    main.asm:33436 ld a, 72
0x5AB0C  E0 E2    main.asm:33437 ldh [$FFE2], a
0x5AB0E  C9       main.asm:33438 ret
0x5AB0F  FA E4 A2 main.asm:33439 ld a, [sEasyMode]
0x5AB12  E6 01    main.asm:33440 and $01
0x5AB14  20 04    main.asm:33441 jr nz, UnknownRJump_0x5AB1A
0x5AB16  06 04    main.asm:33442 ld b, 4
0x5AB18  18 02    main.asm:33443 jr UnknownRJump_0x5AB1C
0x5AB1A  06 02    main.asm:33446 ld b, 2
0x5AB1C  CD 3D 3D main.asm:33449 call UnknownCall_0x3D3D
0x5AB1F  18 10    main.asm:33450 jr UnknownRJump_0x5AB31
0x5AB21  FA E4 A2 main.asm:33451 ld a, [sEasyMode]
0x5AB24  E6 01    main.asm:33452 and $01
0x5AB26  20 04    main.asm:33453 jr nz, UnknownRJump_0x5AB2C
0x5AB28  06 04    main.asm:33454 ld b, 4
0x5AB2A  18 02    main.asm:33455 jr UnknownRJump_0x5AB2E
0x5AB2C  06 02    main.asm:33458 ld b, 2
0x5AB2E  CD 2D 3D main.asm:33461 call UnknownCall_0x3D2D
0x5AB31  F0 E8    main.asm:33464 ldh a, [$FFE8]
0x5AB33  90       main.asm:33465 sub b
0x5AB34  E0 E8    main.asm:33466 ldh [$FFE8], a
0x5AB36  C0       main.asm:33467 ret nz
0x5AB37  F0 E9    main.asm:33468 ldh a, [$FFE9]
0x5AB39  E0 E8    main.asm:33469 ldh [$FFE8], a
0x5AB3B  F0 DF    main.asm:33470 ldh a, [$FFDF]
0x5AB3D  FE 01    main.asm:33471 cp $01
0x5AB3F  CC 45 6B main.asm:33472 call z, UnknownCall_0x5AB45
0x5AB42  C3 33 5D main.asm:33473 jp UnknownJump_0x59D33
0x5AB45  3E 0A    main.asm:33476 ld a, 10
0x5AB47  EA 70 A4 main.asm:33477 ld [sSfxRequest1], a
0x5AB4A  C9       main.asm:33478 ret
```

## Checked entry and interrupt-return contract

- None.

Structurally checked interrupt terminal entries: None.

## Checked mixed state-table contract

- None.

## Recovered rst $28 state tables (CPU targets)

- ROM 0x5AABC: $6AC6, $6B0F, $6B21, $6AD9, $6AF0

## Existing helper dependencies supplied by the supervisor

- None.

## Unresolved work

- None.

An INCOMPLETE brief must not be dispatched as a ready implementation task.
Resolve the listed transfer or split a helper prerequisite and regenerate.
Byte assertions must cover state transitions, gates, arithmetic boundaries,
removal/reset, and dispatch. No generated brief constitutes behavioral proof.
