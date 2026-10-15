# Boss handler region map (bank 2), measured 2026-10-14

The 29 `$92E8` boss anim -> handler entries (28 distinct routines).

**Bank correction (verified, and the port's first extraction was wrong here).** The
table stores **bank-2 CPU addresses**: `$92DF`'s `rst $28` jumps inside the bank that
`$3C94` mapped, which is 2, and the whole `$8D9C`/`$8DA9` chain runs there. So the file
offset is `value + $4000` — *not* `+ $54000` (that is `BANK16_CPU_ADD`, correct for the
level profile tables `$58040`/`$59C07`, which really are bank 22). Checks: entry 0
(`$5322`) is a lone `C9` at `$9322`, the same "inert anim = `ret` right after the table"
idiom as the level table's `$58100`; anim 16 (`$60B9`) is `$A0B9`, a clean
`call $5285` / `sObjectDone` / save-state body; and with `+ $54000` 13 of the 29 entries
land mid-instruction. The decomp's `main.asm` bank-02 section **does** label this family
(`UnknownCall_0x9285` at line 3802, `UnknownCall_0x92DF` at 3856, `UnknownCall_0x9443`
at 4038) — one earlier note in this repo said bank 22 was undisassembled, which was true
only of the wrong addresses.

Method: for each entry, scan to the next entry (plus 80 bytes) for `rst $28` (`$EF`) and
read the inline `dw` run that follows while the targets stay in the bank-2 window. That
is the state machine each entry drives; bodies are shared across entries (e.g. `$9CAC`
is anims 10/11/16/19/23/26's hit-resume arm), which is why the porting unit is the region
plus the helper library, not the entry.

| entry | anims | states |
| --- | --- | ---: |
- `$09322` (anims [0], len 1)
    - `rst $28` @ `$0934D` -> `$09354`, `$09366`, `$09385`
- `$09323` (anims [3], len 132)
    - `rst $28` @ `$0934D` -> `$09354`, `$09366`, `$09385`
    - `rst $28` @ `$093EF` -> `$093FE`, `$09443`, `$09463`, `$0947B`, `$094B6`, `$09508`, `$09543`
- `$093A7` (anims [1], len 607)
    - `rst $28` @ `$093EF` -> `$093FE`, `$09443`, `$09463`, `$0947B`, `$094B6`, `$09508`, `$09543`
- `$09606` (anims [2], len 60)
    - `rst $28` @ `$09669` -> `$09629`, `$09670`, `$09688`
- `$09642` (anims [4], len 86)
    - `rst $28` @ `$09669` -> `$09629`, `$09670`, `$09688`
    - `rst $28` @ `$096BD` -> `$096C4`, `$096E1`, `$09688`
- `$09698` (anims [5], len 102)
    - `rst $28` @ `$096BD` -> `$096C4`, `$096E1`, `$09688`
- `$096FE` (anims [6], len 294)
    - `rst $28` @ `$09753` -> `$09762`, `$09443`, `$09786`, `$097BD`, `$097CF`, `$097E9`, `$097F6`
    - `rst $28` @ `$09863` -> `$0986C`, `$09883`, `$098A4`, `$098B3`
- `$09824` (anims [7], len 164)
    - `rst $28` @ `$09863` -> `$0986C`, `$09883`, `$098A4`, `$098B3`
    - `rst $28` @ `$09909` -> `$0986C`, `$09883`, `$098A4`, `$09912`
- `$098C8` (anims [8], len 95)
    - `rst $28` @ `$09909` -> `$0986C`, `$09883`, `$098A4`, `$09912`
    - `rst $28` @ `$0994B` -> `$09954`, `$0995E`, `$099C3`, `$099D9`, `$0BDCD`
- `$09927` (anims [9], len 783)
    - `rst $28` @ `$0994B` -> `$09954`, `$0995E`, `$099C3`, `$099D9`, `$0BDCD`
- `$09C36` (anims [10], len 281)
    - `rst $28` @ `$09CA1` -> `$09CAC`, `$09CD4`, `$09CDA`, `$09D13`, `$09D29`
    - `rst $28` @ `$09D78` -> `$09CAC`, `$09CD4`, `$09D83`, `$09D8F`, `$09DDF`
- `$09D4F` (anims [11], len 184)
    - `rst $28` @ `$09D78` -> `$09CAC`, `$09CD4`, `$09D83`, `$09D8F`, `$09DDF`
- `$09E07` (anims [12], len 220)
    - `rst $28` @ `$09E5C` -> `$09CAC`, `$09CD4`, `$09E67`, `$09E6E`, `$09EB5`
- `$09EE3` (anims [13], len 17)
    - `rst $28` @ `$09F37` -> `$09CAC`, `$09443`, `$09F46`, `$09F65`, `$09F82`, `$09F65`, `$09FB1`
- `$09EF4` (anims [14], len 309)
    - `rst $28` @ `$09F37` -> `$09CAC`, `$09443`, `$09F46`, `$09F65`, `$09F82`, `$09F65`, `$09FB1`
    - `rst $28` @ `$0A05D` -> `$0A07D`, `$0A08C`, `$0A064`
- `$0A029` (anims [15], len 144)
    - `rst $28` @ `$0A05D` -> `$0A07D`, `$0A08C`, `$0A064`
    - `rst $28` @ `$0A104` -> `$09CAC`, `$0A12C`, `$0A132`, `$0A1D4`, `$0A1F7`, `$0A18A`, `$0A229`, `$0A24C`, `$0A117`
- `$0A0B9` (anims [16], len 453)
    - `rst $28` @ `$0A104` -> `$09CAC`, `$0A12C`, `$0A132`, `$0A1D4`, `$0A1F7`, `$0A18A`, `$0A229`, `$0A24C`, `$0A117`
    - `rst $28` @ `$0A296` -> `$0A29B`, `$0A2A7`
- `$0A27E` (anims [17], len 64)
    - `rst $28` @ `$0A296` -> `$0A29B`, `$0A2A7`
    - `rst $28` @ `$0A2E4` -> `$0A300`, `$0A30C`, `$0A2F1`, `$0A321`, `$0A337`, `$0A353`
- `$0A2BE` (anims [18], len 180)
    - `rst $28` @ `$0A2E4` -> `$0A300`, `$0A30C`, `$0A2F1`, `$0A321`, `$0A337`, `$0A353`
    - `rst $28` @ `$0A3C0` -> `$09CAC`, `$0A420`, `$0A42B`, `$0A448`, `$0A4A5`, `$0A4C0`, `$0A504`, `$0A527`, `$0A548`, `$0A565`, `$0A5B0`, `$0A5CC`, `$0A5F6`, `$0A611`, `$0A63B`, `$0A65A`, `$0A679`, `$0A689`, `$0A6CE`, `$0A6E9`, `$0A706`, `$0A747`, `$0A763`, `$0A78D`, `$0A7A9`, `$0A7D3`, `$0A7F7`, `$0A816`, `$0A83A`, `$0A864`, `$0A88C`
- `$0A372` (anims [19], len 1337)
    - `rst $28` @ `$0A3C0` -> `$09CAC`, `$0A420`, `$0A42B`, `$0A448`, `$0A4A5`, `$0A4C0`, `$0A504`, `$0A527`, `$0A548`, `$0A565`, `$0A5B0`, `$0A5CC`, `$0A5F6`, `$0A611`, `$0A63B`, `$0A65A`, `$0A679`, `$0A689`, `$0A6CE`, `$0A6E9`, `$0A706`, `$0A747`, `$0A763`, `$0A78D`, `$0A7A9`, `$0A7D3`, `$0A7F7`, `$0A816`, `$0A83A`, `$0A864`, `$0A88C`
- `$0A8AB` (anims [20], len 506)
    - `rst $28` @ `$0A90D` -> `$09CAC`, `$0A920`, `$0A9BD`, `$0AA05`, `$0AA39`, `$0A937`, `$0A952`, `$0A96D`, `$0AA70`
    - `rst $28` @ `$0AAA7` -> `$0AAB0`, `$0AAB5`, `$0AAEA`, `$0AB01`
- `$0AAA5` (anims [21, 24], len 139)
    - `rst $28` @ `$0AAA7` -> `$0AAB0`, `$0AAB5`, `$0AAEA`, `$0AB01`
    - `rst $28` @ `$0AB32` -> `$0AB37`, `$0AB4D`, `$0B03E`
    - `rst $28` @ `$0AB70` -> `$0AB75`, `$0AB4D`, `$0B13E`
- `$0AB30` (anims [22], len 62)
    - `rst $28` @ `$0AB32` -> `$0AB37`, `$0AB4D`, `$0B03E`
    - `rst $28` @ `$0AB70` -> `$0AB75`, `$0AB4D`, `$0B13E`
- `$0AB6E` (anims [25], len 29)
    - `rst $28` @ `$0AB70` -> `$0AB75`, `$0AB4D`, `$0B13E`
- `$0AB8B` (anims [23], len 520)
    - `rst $28` @ `$0AC03` -> `$09CAC`, `$0A920`, `$0ACB2`, `$0ACCE`, `$0ACF0`, `$0AC1A`, `$0A952`, `$0AC55`, `$0AD60`, `$0AD31`, `$0ABD1`
- `$0AD93` (anims [26], len 589)
    - `rst $28` @ `$0ADFC` -> `$09CAC`, `$0AE17`, `$0AEDB`, `$0AF07`, `$0AF32`, `$0AF79`, `$0AF86`, `$0AE58`, `$0AE91`, `$0AEAE`, `$0ADD9`, `$0AC3A`, `$0AE3C`
    - `rst $28` @ `$0AFE5` -> `$0AFEC`, `$0B000`, `$0B015`, `$0803E`
- `$0AFE0` (anims [27], len 226)
    - `rst $28` @ `$0AFE5` -> `$0AFEC`, `$0B000`, `$0B015`, `$0803E`
    - `rst $28` @ `$0B0C4` -> `$0B0CF`, `$0B0D3`, `$0B0DA`, `$0B11E`, `$0B14D`
- `$0B0C2` (anims [28], len 318)
    - `rst $28` @ `$0B0C4` -> `$0B0CF`, `$0B0D3`, `$0B0DA`, `$0B11E`, `$0B14D`

28 entries, 275 state-table slots, region `$9322`-`$B0C2` (~7.4 KB).

The helper library these call is bank-2 too (`$9285` hit/vulnerability pass, `$B14E` faderead, `$B157`/`$B15B` finish+blank, the `$B16B`-`$B1EB` step pickers over profile tables`$71A3`/`$71FF`-`$7217`, the `$B224` def-record spawner, `$B264`/`$B26A`/`$B270`/`$B276`screen-X/Y movers on `B_SCRX`/`B_SCRY`, `$B27C`/`$B296`/`$B2C2` face-Mario, `$B2DB`|marioX - X|, `$B2E8`-`$B40E` blink/wobble toggles, `$9435`/`$9443`/`$9725` shared tails)and is a *parallel* set to the bank-22 `$5B4xx`/`$5B5xx` helpers the port already has —byte-compare the two regions and 2020 of 2048 bytes differ, so the same local addressesdo not mean the same routines. Movement in these handlers is 8-bit on `B_SCRY`/`B_SCRX`,never the world X/Y the port's `mv_x_add`/`mv_y_add` operate on.

ROM0 sanity: `$3D22` = X -= b (`ld hl,$FFD1 / sub b`), `$3D32` = X += b, `$3D2D` = Y -= b,`$3D3D` = Y += b.
