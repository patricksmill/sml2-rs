# Hardware display palettes

The desktop preserves the winning pixel's source register (`BGP`, `OBP0`, or
`OBP1`) and its shade after the register's two-bit mapping. Colour is applied
after composition. Tile colour zero remains transparent for sprites, and
background priority still uses the original tile colour. The existing grayscale
rendering API remains available to the replay runner and image tools.

## Game Boy Color

`GBC auto` uses the cartridge header to select three colour palettes.
The boot algorithm accepts Nintendo's old licensee `$01`, or old licensee `$33`
with new licensee ASCII `01`. It adds the 16 title bytes at `$0134..$0143` modulo
256, searches the boot table, and checks the fourth title byte for the table's
duplicate checksums. An unrecognised title or licensee selects combination zero.

The supported ROM's title is `MARIOLAND2`, padded with six zero bytes. Its title
sum is `$C9`, its old licensee is `$01`, and its selected combination is 37:

| Register | RGB555 colours, in shade order 0–3 |
|---|---|
| BGP | `67FF 77AC 1A13 2D6B` |
| OBP0 | `7FFF 01DF 0112 0000` |
| OBP1 | `7FFF 7E8C 7C00 0000` |

The extractor writes the 24 bytes at `$0134..$014B` to `cartridge_header.bin`.
The runtime loads this metadata before play. Older extracted directories without
the file use the supported original ROM's header; malformed or unreadable files
are reported instead of treated as missing. No ROM is opened during rendering.

## Super Game Boy

The desktop offers all 32 built-in presets, labelled `SGB 1-A` through `SGB 4-H`.
The `sgb` command-line alias and the migrated old `SGB color` setting select
1-A. That preset's RGB555 words are `67BF 265B 10B5 2866`. A preset applies the
same four colours to all three register sources.

These are the SGB's selectable presets. This option does not emulate its border,
menu, cartridge-specific automatic selection, or SGB command protocol.

Both hardware modes expand five-bit channels by bit replication
(`(channel << 3) | (channel >> 2)`). They apply no LCD colour correction or
analogue television model. Grayscale, DMG green, Pocket, and Amber remain
presentation choices alongside the hardware palettes.

## Sources and verification

The reference tables are pinned to SameBoy revision
`26a56bd6a122a6318a456abd80558408742d5adf`:

- [CGB bootstrap tables and selection algorithm](https://github.com/LIJI32/SameBoy/blob/26a56bd6a122a6318a456abd80558408742d5adf/BootROMs/cgb_boot.asm):
  `TitleChecksums`, `PalettePerChecksum`, `Dups4thLetterArray`,
  `PaletteCombinations`, `Palettes`, and `GetPaletteIndex`.
- [SGB built-in RGB555 table](https://github.com/LIJI32/SameBoy/blob/26a56bd6a122a6318a456abd80558408742d5adf/Core/sgb.c):
  `built_in_palettes`, in group/letter order.
- [Original CGB boot ROM disassembly](https://gist.github.com/drhelius/6063265):
  header selection and palette dictionary cross-check.

Unit tests check the header selection branches, literal hardware colour words,
source retention through composition, settings migration, and native frame
colourisation. Grayscale parity and the frozen replay sweep guard the existing
presentation and simulation paths. These checks establish palette selection and
composition; they do not establish full-game pixel parity with physical displays.

Acceptance checks for `g-gbc-sgb-palettes`:

- `cargo test --workspace`: 1,595 core, 139 runtime, 31 desktop, and 14 extractor
  tests passed. The desktop also passed its 11 tests without default features.
- Twelve temporary mutations were rejected: register-source selection, palette
  remapping, HUD provenance, title checksum length, both licensee checks,
  duplicate-title selection, SGB table data, settings migration, desktop source
  mapping, preset selection, and the submitted screenshot snapshot.
- `cargo build --workspace --release` completed without warnings.
- `set -o pipefail; bash captures/sweep.sh 2>&1 | tail -n 25` retained the
  established baseline, including the documented `traj4_ff0` exception.

Normal play reaches the new output through the desktop's existing filter control
or `--filter`. Handler coverage is unchanged; this work only changes display
composition and palette selection.
