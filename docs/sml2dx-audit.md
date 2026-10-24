# SML2DX presentation audit

Audited input: 1,048,576-byte patched ROM with SHA-256
`1dd604a91b9ee99ef381cc795da3ae28c72d04ba9de78b355df829e56ce49c46`.
The user identified it as the ROM Claude produced by applying their IPS file.
That provenance does not independently authenticate the IPS publisher/version;
compatibility is restricted to this exact digest. ROM and extracted graphics
remain local and must not be committed. Table auditing used static bounded ROM
slices. The controlled pixel comparison below is separate presentation evidence.

## Opening-level identity and base overlay

`../sml2decomp/levels/levelheaders.asm`, `Level_Header_Pointers` at ROM `$55CB`,
entry zero points to `Level_Header_01` at `$560B`. Its level number is zero,
metatile pointer is bank8 `$4000`, and layout bank is 9. The native opening level
is therefore ID 0 / `level01`. The original loader is documented in
`../sml2decomp/main.asm`, `UnknownJump_0x5579`, and `../sml2decomp/home.asm`,
`UnknownJump_0x03F1`.

The following slices are identical in DX and `../sml2decomp/baserom.gb`:

| Base slice | ROM range (end exclusive) | SHA-256 |
|---|---|---|
| Opening metatiles | `$20000..$20200` | `356423db18111680c1614e439450cd14101932b91062baef84684d060f6d1a94` |
| Opening compressed layout | `$24500..$24E5B` | `1722ea731dddc49fbe77fabdf9b022a0914f70d5e6344ff13d1866561c4d3a6e` |

These establish the opening presentation overlay's geometry compatibility;
they do not establish full-game geometry or gameplay equivalence.

## Graphics and attributes

All ranges below were checked within the audited ROM; destinations fit the
indicated VRAM bank. Sizes are hexadecimal. Banked addresses resolve as
`bank * $4000 + cpu_address - $4000`.

| Data | ROM start | Size | Destination/use |
|---|---:|---:|---|
| Opening tile attributes | `$84000` | `$100` | Attribute indexed by tile ID |
| Shared bank0 graphics | `$6F600` | `$300` | VRAM0 `$8800..$8B00` |
| Opening enemy graphics | `$19800` | `$380` | VRAM0 `$8B00..$8E80` |
| Additional shared graphics | `$1F100` | `$380` | VRAM0 `$8E80..$9200` |
| Opening scenery graphics | `$1C000` | `$600` | VRAM0 `$9200..$9800` |
| Common HUD graphics | `$81D00` | `$300` | VRAM1 `$8800..$8B00` |
| Opening extra graphics | `$90000` | `$D00` | VRAM1 `$8B00..$9800` |
| HUD row tile IDs | `$93C24` | `$14` | Window first row |
| HUD row attributes | `$92040` | `$14` | Window first row attributes |
| Tile animation phases | `$77380` | `$100` | Four phases of `$40` bytes at VRAM0 `$9200` |

Home `$0698` switches to bank `$21`, calls `$732C` (ROM `$8732C`), and copies
`$100` bytes from `$4000 + level * $100` to WRAM bank2 `$D000`. This is a
**tile-ID attribute table**, not a table indexed by metatile. Bank `$24:$79B5`
(ROM `$939B5`) draws the metatile tile IDs, switches VRAM bank1, and looks up
each quadrant's attribute using `$D000 + tile_id`.

Opening attributes select VRAM0, palettes 0–7, without tile flips or BG
priority. HUD attributes are `$8F` except the sixth `$89`: VRAM1, BG priority,
palette7 or palette1 respectively. Bank `$24:$7912` (ROM `$93912`) copies the
HUD IDs, resolves its attributes through pointer table `$6000 + 2*level`
(entry0 `$6040`), and copies the opening extra graphics. The common HUD range
was previously initialized at bank `$20:$6AA3` (ROM `$82AA3`).

Home `LoadMarioGFX` and `$06C0..$0750` in `../sml2decomp/home.asm` document the
bank0 transfer structure. DX config byte ROM `$360A` is `$20`, selecting shared
pointer `$7100` from home table `$0763`; enemy entry `$362A` is
`06 00 00 58`; scenery entry `$36AA` is `07 00 00 40`. Graphics differ from the
base ROM and must be imported, not recolored from original art.

ROM `$35EA=$4D` is incremented to animation config `$4E`. Unchanged home
`UnknownCall_0x3873` selects bank29 `$6680 + $D00 = $7380`, four 64-byte phases,
with an eight-tick period. Imported phases differ from base graphics.

## Palettes and effects boundary

Home `$1974` calls bank `$21:$7300` for attribute setup and bank `$26:$7CD6`
(ROM `$9BCD6`) for palettes. The latter selects BG `$4000 + level*$40`, copies
64 bytes to WRAM2 `$D200`, and selects OBJ at the same offset plus `$1000`.
Opening BG palettes are ROM `$98000..$98040`; OBJ palettes are
`$99000..$99040`. Home `$18AF/$18DA` select bank `$26` and write exactly 64
bytes through CGB BG/OBJ palette ports.

Literal BG RGB555 values, four per palette:

```
0: 7fff 7a89 72e6 0064
1: 7fff 077f 0e7b 0064
2: 7fff 27ac 0300 0180
3: 0300 0180 2a38 1151
4: 36fc 2a38 1151 0064
5: 7a89 077f 0e7b 0064
6: 7fff 2a9a 05b4 6ea3
7: 7fff 2a9a 05b4 0064
```

The scene-color dispatcher ROM `$9AB0B` has special branches for levels
`$19,$03,$07,$0D,$15,$16,$0C,$18`; level0 takes its default path. No opening
specific scenery-color transition was found there. Generic star, powerup,
damage, fade, OBJ remapping, and their frame timing require the separate OBJ
and effects audit. Pixel parity against an independent emulator remains an
acceptance gap and must not be claimed from this static audit.

## Mario, object mappings, and generic effects

Home `$5297` now jumps to `$465A`, which selects Mario bank `$2C:$5D62`
(ROM `$B1D62`). The original bank1 `$4000` mapping table is repurposed.
Mario's opening mapping table is ROM `$B0000`, 242 little-endian CPU pointers
resolved within bank `$2C`. Records retain `[y,x,tile,attr]` cells terminated
by a leading `$80`; the maximum observed list has 15 cells. Home `$3C82`
calls `$07EA`, selecting object drawing bank `$23` for level0. Its mode-A
mapping table is ROM `$8C0B1`, 168 pointers; the maximum list has 12 cells.
Pointers must be resolved individually: some records moved beyond their
original bank3 positions. The mode-B table `$8CF11` is outside this preview.

ROM `$93880..$938BB` selects Mario bank `$2A` normal graphics `$A8200` or
Fire graphics `$A9200`, `$800` bytes to VRAM0 `$8000`. Home `$1952` then
loads `$A8A00`, `$800` bytes to VRAM1 `$8000`. Small, Super, and Bunny share
normal graphics; their mapping frame IDs choose different tiles. Config
`$A21A` is zero normally, one for Fire. Its high nibble selects alternate
visual themes, not Bunny. Those themes are outside opening-level scope.

Literal mapping assertions: Mario frame0 pointer `$B0000 = E4 41` resolves
`$B01E4`, 11 cells, first `[F4,F9,01,20]`, fourth `[FE,F9,11,2C]`, last
`[06,FE,21,29]`. Small frame32 is `$B0608`, Bunny69 `$B09F2`, Fire112
`$B0F58`. Object sprite0 pointer `$8C0B1 = 02 42` resolves `$8C202`:
`[F0,F8,B0,06], [F0,00,B0,26], [F8,F8,B4,07], [F8,00,B2,27], 80`.
OBJ palette0 is `7777 7FFF 107C 0000`; palettes6/7 are
`7777 7FFF 0137 0000` / `7777 46DA 0137 0000`.

Star presentation: home `$54D8` sets `$FFC7` when `$A271 != 0` and `$A272`
bit2 is set. Mario expander `$B1DC2` then replaces the tentative attr with
`original_attr.wrapping_add(($A272 >> 1) & 3)`. It cycles each cell's palette;
it does not replace palette memory. Mapping bank and flip bits are retained.

Pause: home `$2756` calls bank `$31:$4000` (ROM `$C4000..$C40EA`). Each
paused frame reads original BG palettes from WRAM2 `$D200`, subtracts RGB555
channels with saturation, and writes the result to `$D240` for palette job6.
When global frame `$FF97` bit4 is clear, subtract `(8,4,4)`; when set,
subtract `(18,8,4)`. OBJ palettes are unchanged. Unpause requests job5,
restoring original BG palettes.

Legacy fade `$6442` remains unchanged and only writes DMG palette shadows;
DX VBlank `$0154` removed their hardware uploads. These DMG values must not
recolor the DX frame. Game-over color routine `$45C8` is outside the preview,
which returns Home on death. Static audit does not authenticate the IPS
publisher's release identity.

## Controlled presentation check

A local PyBoy run of this exact ROM reached the opening level using menu inputs
only; Mario remained stationary at `(34, 468)`, with native camera `(80, 448)`.
The reference used animation phase 1, HUD Y 136, five lives and timer 400.
After the initial transfers settled, the native eight-tick opening frame matched
all 23,040 RGB pixels, including Mario, scenery and HUD. PyBoy expands RGB555
channels with `channel << 3`; reference pixels were normalized with
`value | (value >> 5)` to match the native renderer's bit replication.

Reference PNGs and the single emulator state remain under `/tmp`, outside the
frozen replay corpus and Git. This comparison establishes one controlled frame;
it does not establish full-level or full-game pixel parity. Unit tests separately
cover bank selection, raw-index priority, palette effects and imported tables.

## Acceptance commands

The workspace suite passed: 1,636 core, 37 desktop, 18 DX importer/cache,
14 extractor and 174 runtime/unit-integration tests. The desktop's 14 tests
without default features also passed. The frozen sweep retained its documented
`traj4_ff0` exception and completed successfully.

Real-ROM tests are explicitly opt-in so the repository carries no ROM fixtures:

```sh
export SML2_DX_ROM=/path/to/the/audited/sml2dx.gbc
cargo test -p sml2-dx --test revision -- --ignored
cargo test -p sml2-runtime --test dx_preview -- --ignored
cargo test -p sml2-desktop real_rom_import_restart -- --ignored
```

All three passed against the audited local input. These exercise table extraction,
bundle round trips, character graphics, palette effects, import, restart restore,
cancellation and save isolation. Native file-dialog interaction itself was not
automated; desktop navigation and preview transitions use the egui test harness.

## Luigi opening-theme graphics

The same audited ROM supports a second presentation character without changing
opening BG/OBJ palette RAM. Character identity is HRAM `$FFF6` (Mario 0,
Luigi 1), not `$A125`. Home `$465A` masks its low nibble and selects Luigi
bank `$2D:$5E16` (ROM `$B5E16`). Graphics job7 ROM `$93880..$938BB` selects
bank `$2B` when `$FFF6 == 1`; home `$1952` performs both VRAM bank transfers.

| Luigi data | ROM start | Size/use |
|---|---:|---|
| Normal opening graphics | `$AC200` | `$800` bytes to VRAM0 `$8000` |
| Fire opening graphics | `$AD200` | `$800` bytes to VRAM0 `$8000` |
| Shared overlay graphics | `$ACA00` | `$800` bytes to VRAM1 `$8000` |
| Player mapping pointers | `$B4000` | 242 little-endian pointers within bank `$2D` |

Luigi mapping lists use the same four-byte cells and `$80` terminator, with a
maximum 15 cells. The opening uses the normal-theme table. Alternate-theme
mapping table `$B6000` and alternate-theme graphics remain outside this import.
Luigi selects green colors using mapping palette attributes; level0 initial
BG/OBJ palettes at `$98000`/`$99000` are shared. Frame selection
`$5447..$54A7` is common: Small `+32`, Bunny `+69` (flutter `+26`), Fire
`+112` (throw `+32`). Luigi star helper `$B5E73` adds
`((star_low >> 1) & 3)` to original attributes, matching Mario. Priority and
coordinate expansion also match. This graphics audit establishes no physics
claim.

Literal fixtures: `$B4000 = E4 41` resolves `$B41E4`, 11 cells. Its first
cell is `[F4,F9,01,22]`, fifth `[F7,FD,10,2A]`. Small32 pointer `48 46`
resolves `$B4648`, 10 cells; Bunny69 pointer `32 4A` resolves `$B4A32`, 11;
Fire112 pointer `9C 4F` resolves `$B4F9C`, 13. Graphics SHA-256 values:

- Normal: `12ca97a4c770daab7b1bfee26554adb3d002f4021ecc8bc1bb61a2480472ddf7`
- Fire: `4acc7c89c4e997143403deba64d55f4e4788fe5e5d72b1503a2e3a36a5e0b517`
- Bank1: `3dc821c8d8e1f92d98f61c2f9923b920eb071d074d9dd9f28cbe3dfac5c9e909`

Importer/schema version2 includes validated Luigi graphics. Schema1 bundles
remain readable as Mario-only bundles; adding Luigi requires re-import from
the same exact audited ROM. No ROM or extracted graphics are committed.

## All levels

Importer/schema version 3 replaces the opening-level tables with the DX ROM's
bank-0 level loader tables and 18 raw banks; `sml2_dx::Presentation` derives
every level from them. Schema 1 and 2 bundles are rejected with a re-import
message. Banks `$06`, `$07` and `$1B`-`$1F` hold base-game graphics DX patched
in place (level tilesets, enemy sets, arena tilesets, the `$Bx` transfer source
and animation rows). The rest are DX additions: `$20` common HUD tiles, `$21`
attributes, `$23`/`$28`/`$3A` object mappings, `$24` VRAM1 graphics and HUD
tables, `$26` palettes, `$2A`/`$2B` player graphics and `$2C`/`$2D` mappings.

`LoadMarioGFX` (`$0669`) is unchanged from the base game after its config read:
`$0763`, `$362A` and `$36AA` are byte-identical, and only `$360A`
(`sLevelGfxConfig`) differs. With set `s = config >> 4`, VRAM0 is bank 7
`$0763[s]` (`$380` to `$8E80`), bank `$1B` `$7000 + s*$300` (`$300` to `$8800`),
the level's enemy set (`$380` to `$8B00`) and tileset (`$600` to `$9200`).
Config nibble 0 or 1 clears `$A21A`'s theme; any other nibble (5, levels `$12`,
`$13`, `$1C`) sets theme `$10`.

| Per level | Source |
|---|---|
| Tile attributes | `$21:$4000 + level*$100` (`$21:$732C`) |
| BG / OBJ palettes | `$26:$4000 + level*$40`, OBJ `+$1000` (`$26:$7CD6`) |
| HUD attributes | pointer at `$24:$6000 + level*2` (`$24:$7912`) |
| VRAM1 `$8800..$8B00` | `$20:$5D00`, all levels |
| VRAM1 `$8B00..$9800` | `$24:$4D00` level `$09`, `$5A00` `$0C`, `$6700` `$0D`, else `$4000` |
| Object mappings | `$07EA`: bank `$28` for `$0B`/`$0F`, `$3A` for `$0D`, else `$23`; table `$40B1` (168) for levels below `$0A`, `$4F11` (228) from `$0A` |
| HUD tile IDs | `$24:$7C24`, all levels |
| Animation | bank `$1D` `$6680 + row*$100`, as the base game |

The character table `$24:$7D00` is all `$FF`: no level forces a character.
`$24:$7880` selects player graphics by `$A21A`: `$4200` normal, `$5200` Fire,
`$5A00`/`$6200` for theme `$10`, from bank `$2A` (Mario) or `$2B` (Luigi), with
`$4A00` to VRAM1 `$8000`. `$2C:$5D62` and `$2D:$5E16` read mappings from `$4000`,
or `$6000` under theme `$10`.

DX also rewrote 21 levels' `Level_TileMap` metatile tables (bank 8, at the
header's `+$0B` pointer) and 15 levels' compressed layouts (`$42F0[+$12]` in the
header's `+$0D` bank). Schema 4 carries both. A PyBoy capture of level `$01`
matched every streamed BG map cell against DX's layout and metatiles, against
608 of 1,024 for the base layout. Most layout edits are decoration; levels `$01`,
`$06`, `$0D`, `$0E`, `$11` and `$1B` also move solid cells (768 in `$06`). DX
gameplay is not ported, so the renderer draws DX's cell only where it keeps the
native cell's collision class, and the native cell otherwise.

## Title and file select

`$2ABE` now calls `$20:$6A40`: VRAM0 `$8800..$97FF` from `$20:$4000`, VRAM1
`$8800` from `$20:$5D00` and `$8B00` from `$20:$5000`, an 18-row VRAM1 attribute
map from `$20:$6800`, and palettes `$26:$7870` (BG) and `$26:$78B0` (OBJ). The
title map stays at `$05:$59E5`, edited in place. The file select keeps `$2B8B`'s
VRAM0 copies and the `$0C:$4A52` map (both edited in place); `$28D9` calls
`$25:$4234`, which loads the attribute map `$25:$4000` (`$234` bytes) and BG
palettes `$26:$7D17`. VRAM1 and OBJ palettes carry over from the title, with the
player's VRAM1 overlay at `$8000`. Rendered title and file-select frames match
PyBoy's on all 23,040 pixels.

The character door uses two hooks in bank `$0C`: `$4326` calls `$7158`
(ROM `$33158`) before reading menu input, and the right limit at `$4398`
jumps to `$7186` (ROM `$33186`) when cursor X is at least `$88` (Clear).
That extension starts a 64-frame right walk to X `$C8`; the next input
frame toggles the character, requests graphics reload 7 and SFX 0 `$0E`,
then starts a 64-frame left walk back to Clear. `$7186` rejects another
right move at X `$C8`. The native file select enables this extension only
for installed DX presentation, and reports a typed character switch to the
session and desktop Character setting. Disabling DX during the trip returns
the cursor to Clear without an unrequested character switch or slot write.
The graphics reload handler at `$24:$7880` (ROM `$93880`) selects Mario/Luigi
graphics from banks `$2A`/`$2B`; it does not change the door's tilemap.

## Overworld

DX edits the overworld's tiles, maps, route patches and cursor poses in place,
so the importer runs the native extractor's map builders (`sml2_extract::map`)
over the DX banks `$06`, `$0D`-`$11`, `$18` and `$19`. Colors are DX's own:

- Hub: `$26:$77F1` loads BG palettes `$26:$6380`, or `$63C0` once the camera X
  (`$FFB9`) reaches `$E7`, and OBJ `$6400` (Mario) / `$6440` (Luigi, `$FFF6`).
  `$29:$67E9` copies the per-tile attribute table `$29:$6542` to `$D000`; every
  hub cell's attribute is `$D000[tile]` (verified on all 1,024 cells of a
  capture). The status bar's backdrop draws in color 3.
- Zones: branches `$26:$7969`.. set BG and OBJ palettes and call `$29:$6642`
  with an index for the 32x18 attribute map. Pumpkin `$6780`/`$67C0`/`$4B40`,
  Tree `$6480`/`$64C0`/`$4000`, Macro `$6600`/`$6640`/`$4240`, Space
  `$66C0`/`$6700`/`$46C0` (`$4900` once `sMapSpacePath0` bit 0 is set), Turtle
  `$6540`/`$6580`/`$4480`, Mario `$6840`/`$6880`/`$4D80`, Wario `$69C0`/`$6A40`/
  `$5200`, castle hill `$6900`/`$6940`/`$4FC0`; Luigi's OBJ set is `+$40`.
- Sprites: the decor tables `$0F:$4C4D` (hub) and `$18:$62E6` (zones), and the
  status bar `$0F:$7251`, keep their layout; DX adds CGB palette bits to their
  attribute bytes. The route-patch table `$18:$63EA` is the zone decor table's
  last 58 entries.

The hub's wave shimmer is replayed over DX's tiles from the native wave's start
phase and tick count (its state repeats every 128 ticks). Captures of the hub and
Tree Zone matched PyBoy except where water and decor animation timing differ.
Both minigames (coin rooms 9–12 and bell rooms 13–14) use these DX screens:
coin rooms take BG `$75DA`/`$759A`/`$755A`/`$751A` and shared OBJ `$761A`, with
attribute maps bank `$37` `$7340`/`$7100`/`$6EC0`/`$6C80`, selected by the door
cursor's X. Bell rooms take BG `$745A`/`$749A`, OBJ `$74DA`, and attribute maps
`$6800`/`$6A40`. Tiles stay the in-place bank 25 and bank 6 copies. Map palette
fades stay native.

Mid-level copies keep their base addresses: `$25AF`'s tables `$268B`/`$2699`,
the bank-7 `$7800` block and `$216C`'s transfer code are byte-identical, so the
renderer finds the native source of every rewritten tile and reads the same
address from the DX banks. Animation phases are identified by the copied row
and the clock's phase, since some rows repeat a phase in the base art but not
in DX's. Rendering level 1's surface route (3,374 frames) and a pipe-room
route (6,000 frames), for both characters, matched the previous opening-level
renderer frame for frame, including its PyBoy-checked first frame.

Boss arenas: every `$25AF` reload calls `$02:$7500`, which keys `$A1CC` by
the level (the six zone bosses' ids from `$02:$4E03`.., else `sCurLevel`), then
`$36:$7654` copies that arena's VRAM1 tiles (`$8B00`, or `$8900` for `$18`) and
adds `$10` to `$A21A` for `$13`, moving Mario's mappings to the `$6000` table.
The reload swaps in `$26B5`'s metatile table, which DX edits in place, so DX
arenas read DX's bytes at that pointer with the native arena layout. When the
fight's scene load ends, `$0ECA` calls `$36:$7306`: a 32x18 VRAM1 attribute
map at `$9840`/`$9860` (`$77BA`), BG palettes from `$26` into `$D200`
(`$1905`), and the OBJ set at `$A1DE:$A1DF` (`$18DA`). Cells streamed later
take the per-tile table. `$18` keys its fights by `$AFC0`: def 6 also fills
`$9940..$99BF` with attribute 3 and swaps `$D000` for `$36:$7554`. The boss
render pass reads DX's `$19:$6092` lists, where DX moved 30 of the 118
records and recolored most cells, so the port records each draw's sprite id,
base and flip flags and replays the walk through DX's lists. Level `$09`'s
fight matched a PyBoy capture except where Mario stood elsewhere; the
witch's OAM attributes matched list 16 under its flip.

Known gaps, verified against the ROM and left native or approximate:

- Boss arenas, `$18`'s later fights: Wario's defs 7 and 8 (`$AFC0`) are not
  checked on hardware. Def 7 loads nothing new and keeps def 6's look; def 8's
  `$74A4` VRAM0 copies and palette-entry patches are applied over def 6's sets.
  The other boss levels went through `$36:$7306`'s table path, and only `$09`
  was compared with PyBoy.
- `$26:$6B0B` switches palette sets from `$26:$6100 + n*$40` for levels `$19`,
  `$03`, `$07`, `$0D`, `$15`, `$16`, `$0C`, `$18` by Mario's position (`$AF30`-
  `$AF33` are Mario X+8 and Y+32 from `$02:$40FD`), remembering the current set
  in `$FFAD`. Those levels keep their load-time palette.
- `$D300` is the blank palette buffer `$25AF`'s hook and the fades upload; DX
  palette fades are not reproduced.

