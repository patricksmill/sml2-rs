# Boss handler region map (bank 22), measured 2026-10-14

The 29 `$92E8` boss anim -> handler entries (28 distinct), read from `baserom.gb`
directly: the decomp has **no labels** for bank 22 (`main.asm`'s sections stop at
bank `$1F`), and the port dispatches on *file* addresses (`local + $54000`).

Method: for each entry, scan the region up to the next entry (plus 80 bytes) for
`rst $28` (`$EF`), and read the inline `dw` run that follows while the targets stay
inside the bank-22 window. That is the state machine each entry drives; the bodies
live *elsewhere in the same region*, often shared between entries, and some states
jump *back* into bodies the port already keys. Verified example, from the ROM bytes
(`sm83.py 582FB 58320`):

```
582FB: F0 F0     ldh a,[$FFF0]       ; odd frames -> the $58339 tail
582FD: E6 01     and $01
582FF: 20 38     jr nz,$58339
58301: 06 01     ld b,1
58303: CD 22 3D  call $3D22          ; X += 1
58306: CD 1C 3C  call $3C1C
58309: FA 2F AF  ld a,[sMoveFlags]
5830C: CB 4F     bit 1,a
5830E: C8        ret z
5830F: 3E 02     ld a,2
58311: E0 DF     ldh [$FFDF],a       ; B_FSTATE = 2
58313: F0 DD     ldh a,[$FFDD]
58315: EE 20     xor $20             ; flip bit 5
58317: E0 DD     ldh [$FFDD],a
```

`$5831A` is its mirror (X -= 1, `$3BB8`, `sMoveFlags` bit 0 -> state 1). These are the
walker's state-1/state-2 bodies, which the port already implements *inside*
`handler_walker` (`$58201`), so the boss states that jump to them map onto existing
port logic rather than new code. That reuse is also why the region cannot be ported
entry-by-entry: the unit of work is the region.

Region extent measured from the state-table targets: roughly `$59092`-`$5B1D8`
(~8.4 KB), of which 28 entry points and 387 state-table slots are now mapped below.

| # | entry | anims | states |
| --- | --- | --- | ---: |
- `$59322` (anims [0], len 1)
    - `rst $28` @ `$59353` -> `$59092`, `$590A8`, `$5935C`, `$590D8`
- `$59323` (anims [3], len 132)
    - `rst $28` @ `$59353` -> `$59092`, `$590A8`, `$5935C`, `$590D8`
    - `rst $28` @ `$5938D` -> `$59394`, `$593AE`, `$593BC`, `$580CD`
    - `rst $28` @ `$593F3` -> `$593F8`, `$5941A`
- `$593A7` (anims [1], len 607)
    - `rst $28` @ `$593F3` -> `$593F8`, `$5941A`
    - `rst $28` @ `$5949F` -> `$594A4`, `$594C9`
    - `rst $28` @ `$5950D` -> `$59512`, `$59534`
    - `rst $28` @ `$595D2` -> `$595D7`, `$595F5`
    - `rst $28` @ `$5961C` -> `$59621`, `$5964D`
- `$59606` (anims [2], len 60)
    - `rst $28` @ `$5961C` -> `$59621`, `$5964D`
    - `rst $28` @ `$59689` -> `$5968E`, `$596BA`
- `$59642` (anims [4], len 86)
    - `rst $28` @ `$59689` -> `$5968E`, `$596BA`
    - `rst $28` @ `$596E7` -> `$596EE`, `$596EE`, `$59716`
- `$59698` (anims [5], len 102)
    - `rst $28` @ `$596E7` -> `$596EE`, `$596EE`, `$59716`
- `$596FE` (anims [6], len 294)
    - `rst $28` @ `$597DD` -> `$597EE`, `$5985C`, `$59806`, `$59874`, `$59827`, `$59899`, `$59843`, `$598B5`
- `$59824` (anims [7], len 164)
    - (no inline `rst $28` table: linear routine)
- `$598C8` (anims [8], len 95)
    - (no inline `rst $28` table: linear routine)
- `$59927` (anims [9], len 783)
    - `rst $28` @ `$599B5` -> `$59A0F`, `$59A5D`, `$59A6D`, `$59A7D`, `$599C4`, `$599DA`, `$599EE`
    - `rst $28` @ `$59AC5` -> `$59ACA`, `$59AD1`
    - `rst $28` @ `$59B30` -> `$59B35`, `$59B54`
    - `rst $28` @ `$59BC0` -> `$582FB`, `$5831A`
    - `rst $28` @ `$59BCA` -> `$59BD1`, `$59D05`, `$59D41`
    - `rst $28` @ `$59BE3` -> `$59BE8`, `$59BF5`
    - `rst $28` @ `$59C06` -> `$581B1`, `$582D1`, `$5834F`, `$5839B`, `$58444`, `$59CE7`, `$59D5A`, `$59E06`, `$5862E`, `$58201`, `$582A9`, `$59E5B`, `$584E0`, `$58541`, `$585ED`, `$586E1`, `$586F5`, `$58712`, `$58758`, `$58758`, `$58755`, `$58712`, `$587BD`, `$587D7`, `$58804`, `$58758`, `$587A1`, `$586EE`, `$586EE`, `$586EE`, `$586EB`, `$586E6`, `$59FB9`, `$59EFF`, `$59FCE`, `$59FDF`, `$5A031`, `$5A0AA`, `$5A11E`, `$5A179`, `$58100`, `$58100`, `$58100`, `$58C41`, `$58CE0`, `$58100`, `$58DD1`, `$58E12`, `$58100`, `$58100`, `$58100`, `$58F4C`, `$5A1F6`, `$5A8D2`, `$5A8E4`, `$5A969`, `$58100`, `$591DE`, `$591F9`, `$58201`, `$582A9`, `$59CE7`, `$5A236`, `$5A258`, `$5A2D0`, `$5A30E`, `$584CB`, `$5A355`, `$5A545`, `$5A5C7`, `$58100`, `$5A5EC`, `$5A667`, `$5A683`, `$5A6E7`, `$5A721`, `$5A739`, `$5A773`, `$5A816`, `$5A85C`, `$5A992`, `$5A8D2`, `$5A9D3`, `$5A9E8`, `$5AA68`, `$59F5E`, `$5AA71`, `$5AA71`, `$5AFD7`, `$5B06D`, `$5B0D5`, `$5B177`, `$5B1C8`, `$59BAF`, `$5B241`, `$5B28C`, `$58DD1`, `$58E12`, `$5AAB9`, `$5AB4B`, `$5AB79`, `$5AC01`, `$5AC3C`, `$5AC7D`, `$5ACE4`, `$5ACE4`, `$5AD2D`, `$5AD4D`, `$58100`, `$58100`, `$5ADDD`, `$5AEF4`
- `$59C36` (anims [10], len 281)
    - `rst $28` @ `$59D00` -> `$59D05`, `$59D41`
    - `rst $28` @ `$59D71` -> `$59D7A`, `$59D8B`, `$59DA8`, `$59DC7`, `$584CD`
- `$59D4F` (anims [11], len 184)
    - `rst $28` @ `$59D71` -> `$59D7A`, `$59D8B`, `$59DA8`, `$59DC7`, `$584CD`
    - `rst $28` @ `$59E27` -> `$59E2C`, `$59E4B`
- `$59E07` (anims [12], len 220)
    - `rst $28` @ `$59E27` -> `$59E2C`, `$59E4B`
    - `rst $28` @ `$59E71` -> `$59E7A`, `$59EA0`, `$59EB5`, `$59EE1`
    - `rst $28` @ `$59F1F` -> `$59F24`, `$59F41`
- `$59EE3` (anims [13], len 17)
    - `rst $28` @ `$59F1F` -> `$59F24`, `$59F41`
- `$59EF4` (anims [14], len 309)
    - `rst $28` @ `$59F1F` -> `$59F24`, `$59F41`
    - `rst $28` @ `$59F7E` -> `$59F83`, `$59FA3`
    - `rst $28` @ `$59FF1` -> `$59FF8`, `$59FF8`, `$5A01C`
    - `rst $28` @ `$5A050` -> `$5A055`, `$5A06F`
- `$5A029` (anims [15], len 144)
    - `rst $28` @ `$5A050` -> `$5A055`, `$5A06F`
    - `rst $28` @ `$5A0CC` -> `$582FB`, `$5831A`, `$5A0FE`, `$5A0FE`, `$5823E`
- `$5A0B9` (anims [16], len 453)
    - `rst $28` @ `$5A0CC` -> `$582FB`, `$5831A`, `$5A0FE`, `$5A0FE`, `$5823E`
    - `rst $28` @ `$5A13F` -> `$5A148`, `$5A154`, `$5A16D`, `$5A173`
    - `rst $28` @ `$5A18E` -> `$5A197`, `$5A19C`, `$5A1BE`, `$5A1D4`
    - `rst $28` @ `$5A205` -> `$5A221`, `$5A228`, `$5A20C`
    - `rst $28` @ `$5A247` -> `$5A24A`
    - `rst $28` @ `$5A274` -> `$5A27B`, `$5A2A5`, `$5A2CC`
- `$5A27E` (anims [17], len 64)
    - `rst $28` @ `$5A2EC` -> `$5864E`, `$5866D`, `$5864E`, `$5866D`, `$5A2F7`
- `$5A2BE` (anims [18], len 180)
    - `rst $28` @ `$5A2EC` -> `$5864E`, `$5866D`, `$5864E`, `$5866D`, `$5A2F7`
    - `rst $28` @ `$5A32D` -> `$5A332`, `$5A339`
- `$5A372` (anims [19], len 1337)
    - `rst $28` @ `$5A573` -> `$5A578`, `$5A58D`, `$598CD`
    - `rst $28` @ `$5A5E0` -> `$5A5E3`
    - `rst $28` @ `$5A606` -> `$5A60F`, `$5A633`, `$5A60F`, `$5A65A`
    - `rst $28` @ `$5A67A` -> `$58226`, `$58253`, `$58271`, `$58282`
    - `rst $28` @ `$5A692` -> `$5A6A5`, `$5A6B6`, `$5A6D2`, `$5A69B`
    - `rst $28` @ `$5A6FA` -> `$5A6FF`, `$5A710`
    - `rst $28` @ `$5A74C` -> `$5A751`, `$5A762`
    - `rst $28` @ `$5A78A` -> `$5A793`, `$5A79D`, `$5A7CB`, `$5A801`
    - `rst $28` @ `$5A835` -> `$5A83E`, `$5A845`, `$5A850`, `$5A856`
    - `rst $28` @ `$5A872` -> `$5A87B`, `$5A89F`, `$5A87B`, `$5A8C6`
- `$5A8AB` (anims [20], len 506)
    - `rst $28` @ `$5A914` -> `$5A939`, `$5A92D`, `$5A945`, `$5A94D`, `$594CD`
    - `rst $28` @ `$5A976` -> `$5A979`
    - `rst $28` @ `$5A9B1` -> `$5A9B6`, `$5A9BD`
    - `rst $28` @ `$5AA15` -> `$5AA20`, `$5AA30`, `$5AA53`, `$5AA5A`, `$5AA0A`
    - `rst $28` @ `$5AA88` -> `$5AAA0`, `$5AAA7`, `$5AA8F`
    - `rst $28` @ `$5AABB` -> `$5AAC6`, `$5AB0F`, `$5AB21`, `$5AAD9`, `$5AAF0`
- `$5AAA5` (anims [21, 24], len 139)
    - `rst $28` @ `$5AABB` -> `$5AAC6`, `$5AB0F`, `$5AB21`, `$5AAD9`, `$5AAF0`
    - `rst $28` @ `$5AB4D` -> `$5AAC6`, `$5AB0F`, `$5AB21`, `$5AAD9`, `$5AB58`
    - `rst $28` @ `$5AB7E` -> `$5AB89`, `$5ABAE`, `$5ABB3`, `$5ABB8`, `$5ABD0`, `$58A3E`
- `$5AB30` (anims [22], len 62)
    - `rst $28` @ `$5AB4D` -> `$5AAC6`, `$5AB0F`, `$5AB21`, `$5AAD9`, `$5AB58`
    - `rst $28` @ `$5AB7E` -> `$5AB89`, `$5ABAE`, `$5ABB3`, `$5ABB8`, `$5ABD0`, `$58A3E`
- `$5AB6E` (anims [25], len 29)
    - `rst $28` @ `$5AB7E` -> `$5AB89`, `$5ABAE`, `$5ABB3`, `$5ABB8`, `$5ABD0`, `$58A3E`
- `$5AB8B` (anims [23], len 520)
    - `rst $28` @ `$5AC03` -> `$5AC08`, `$5AC11`
    - `rst $28` @ `$5AC3E` -> `$5AC47`, `$5AC5F`, `$5AC65`, `$5AC54`
    - `rst $28` @ `$5AC7F` -> `$5AC88`, `$5AC9B`, `$5ACB4`, `$5ACCF`
    - `rst $28` @ `$5ACE6` -> `$5ACED`, `$5AD1C`, `$5ACFA`
    - `rst $28` @ `$5AD2F` -> `$5AD36`, `$5AD3C`, `$5ACFA`
    - `rst $28` @ `$5AD4F` -> `$5AD5A`, `$5AD72`, `$5AD85`, `$5AD72`, `$5ADB1`
    - `rst $28` @ `$5ADE2` -> `$5ADED`, `$5ADF9`, `$5AE0D`, `$5AE2E`, `$5AE42`, `$5883E`
- `$5AD93` (anims [26], len 589)
    - `rst $28` @ `$5ADE2` -> `$5ADED`, `$5ADF9`, `$5AE0D`, `$5AE2E`, `$5AE42`, `$5883E`
    - `rst $28` @ `$5AEF9` -> `$5ADED`, `$5AF04`, `$5AF19`, `$5AF2E`, `$5AF40`, `$59521`
    - `rst $28` @ `$5AFD9` -> `$5AFE4`, `$5AFF4`, `$5AFFB`, `$5B037`, `$5B05E`, `$58D3E`
- `$5AFE0` (anims [27], len 226)
    - `rst $28` @ `$5B06F` -> `$5B078`, `$5B085`, `$5B08C`, `$5B0C6`
    - `rst $28` @ `$5B0DA` -> `$5B0E5`, `$5B0F8`, `$5B11E`, `$5B137`, `$5B15D`, `$58D3E`
- `$5B0C2` (anims [28], len -194)
    - (no inline `rst $28` table: linear routine)

28 entries, 387 state-table slots.
