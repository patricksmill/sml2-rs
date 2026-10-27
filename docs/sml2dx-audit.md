# SML2DX compatibility audit

Source audit: 2026-10-27. Supported input: 1,048,576-byte DX ROM, SHA-256
`1dd604a91b9ee99ef381cc795da3ae28c72d04ba9de78b355df829e56ce49c46`.
Base ROM SHA-256:
`5450dce1bd0c073964c374b5b5b5729dce8d00f2e807892c34af32b8bce1392e`.
Compatibility is restricted to this digest; the audit does not authenticate the
patch publisher/version. ROMs and imported graphics remain local.

[User setup](desktop.md#sml2dx-mode) · [Remaining work](plans/2026-10-26-dx-completion.md)

## Implemented presentation

Schema 10 imports 42 banks plus loader tables/headers, movement curves and
packed spawn streams. `sml2-dx::Presentation`
derives level graphics, CGB attributes/palettes, both characters and space themes,
object mappings, arenas, title/file select, maps/minigames, castle, Game Over,
ending and credits. Saves are shared with the original game; DX sessions
simulate DX layouts, spawn positions, object patches and Luigi's movement
(see the gameplay sections below).

Key source locations (bank:CPU; range ends exclusive):

| Data / loader | Source |
|---|---|
| Level header pointers / graphics configuration | ROM `$55CB` / `$360A`; home `$0669` |
| Metatiles / compressed layouts | Header+$0B in bank8 / header+$0D bank, `$42F0[header+$12 & 15]` |
| Tile-ID attributes | `$21:$4000 + level*$100`; loader `$21:$732C` |
| BG / OBJ palettes | `$26:$4000 + level*$40`, OBJ+$1000; loader `$26:$7CD6` |
| HUD / extra VRAM1 graphics | `$24:$7912`; common tiles `$20:$5D00` |
| Mario / Luigi graphics | Banks `$2A/$2B`; loader `$24:$7880` |
| Mario / Luigi mappings | Banks `$2C/$2D`, `$4000` normal / `$6000` space |
| Object mappings | Banks `$23/$28/$3A`; selection home `$07EA` |
| Title / file select | `$20:$6A40` / `$25:$4234`; character door `$0C:$7158/$7186` |
| Hub / zones / minigames | `$26:$77F1/$7969/$765A/$78F0`; attributes banks `$29/$37` |
| Boss arenas / OAM | `$36:$7654/$7306`; sprite lists `$19:$6092` |
| Castle / Game Over | `$27:$4480/$4505` |
| Ending | `$29:$6718`; font/map `$1A:$4656/$5096` |

Pause subtracts BG RGB `(8,4,4)` or `(18,8,4)` according to frame bit 4; OBJ is
unchanged. Star adds `(star_low >> 1) & 3` to each player's mapping attribute.
Rendering must remain read-only. Mid-level tile copies use their native source
addresses in the imported DX banks; repeated base animation frames are resolved
using the copied row and animation phase.

## Regional palettes

All eight branches are implemented in `session/dx_palette.rs` (`291981a`),
timed from the pose pass's staged references (`451b824`). Below, XH/XL and YH/YL are
bytes of the **staged** references `(player_x+8, player_y+32) mod 65536`;
S is the shared `$FFAD` latch. Unmatched positions retain state/colors, and
ordinary actions skip when S already equals their target.

| Level / bank26 range | Branch contract |
|---|---|
| `$0C`, `$6B39..$6BA7` | XH01/0E -> S0/BG6100; 02/04/0A/0D -> S1/6140; 05/07/09 -> S2/6180 |
| `$15`, `$6BC4..$6C5B` | XH09 -> S1/BG60C0; XH0B,YH02,YL<15 -> S0/6080. XH05 requires YH02: YL>=5D -> S1; otherwise fallthrough compares **YL**, so YL09 -> S1 and YL0B -> S0 |
| `$19`, `$6C5B..$6D1A` | Q=swap(XH)+swap(XL&80), byte wrapping. Q>S updates S even off-trigger; exact Q18..70 step8 selects 12 BG5A80/OBJ5D80 sets, stride40. Reversal/skipped triggers do not restore/interpolate |
| `$03`, `$6D1A..$6DFD` | XH00/01 -> S0/1 BG5A00/5A40; XH05/06 -> S0/1 OBJ palette5 colors1/2 =0995,2E9C /7FFF,11C8. XH0A combines BG5A40 with XH06's patch |
| `$07`, `$6DFD..$6E5E` | XH7/8 -> S0/1; OBJ palette5 colors1/2 =7BD9,6B45 /56B5,294A |
| `$16`, `$6E5E..$6EB3` | XH08/0C -> S0, OBJ palette5 color2=267D; XH09/0B -> S1, color2=5294 |
| `$0D`, `$6EB3..$6FBC` | With YH!=0: XH00 -> S0/BG61C0, XH01/05 -> S10/6200, XH06 -> S1/6240. XH08/0B -> S2/6280 without YH gate. XH07 is not a trigger |
| `$18`, `$6FBC..$703A` | XH8,YH2 -> S1/OBJ62C0; XH0,YH2 -> S2/6300; XH5,YH1 -> S3/6340 |

Addresses/constants in the table are hexadecimal. Whole palette sets are 64
bytes; direct OBJ patches leave neighboring colors and BG unchanged.

Source order: home `$0618` pose setup -> bank1 `$54CF` -> `$26:$6B00/$6B0B`,
then home `$0636` object engine stages the next references at `$02:$4100..$4120`.
The native selector runs at `$0618` on the previous object pass's `$AF30..$AF33`
bytes and skips paused, frozen and dead frames. `$22C4` pumps pose/objects only
when `$A28C` is nonzero; `$2304` sits in `$22DF`, which nothing calls in this
ROM (`regional_timing.rs`). Level HUD initialization `$24:$793C` clears S; boot
and hub palette branches also write it. No unconditional pipe/arena reset was
found. Boss scene loads (`$36:$7306`, except Wario 7) replace the colors while
S keeps the section, so a later regional trigger can override fight palettes.

## Fades and scene effects

Implemented in `session/dx_effects.rs` and the DX renderer (`5a10a26`,
`f5cc37f`, `47169a2`, `6467e18`). The job queue's one-upload-per-VBlank
ordering below is not yet modeled; effects are applied when rendering.

Palette jobs `$24:$7800..$7912` consume `$FFF8` once per eligible VBlank, then
queue `$FFF7` for the next dispatch. Jobs 1/2/3 upload pointer BG/OBJ/both; 4
uploads canonical BG `$D200` plus OBJ; 5 restores BG; 6 uploads temporary BG
`$D240`. Keep original buffers, live palette RAM and queued jobs distinct.
RGB555 arithmetic saturates each five-bit channel independently.

| Effect / source | Required behavior |
|---|---|
| White blanking, `$20:$6A66..$6A7E` | `$D300` holds 32 `$7FFF` words, uploaded to BG+OBJ by: boss reload `$25AF -> $36:$7746` (white until `$0EC0 -> $7306` at scroll end; native blank window); coin rooms `$3F307 -> $26:$765A` (job 7 then job 4: first room frame white); map return `$3C528 -> $0F:$78AD` (step 0 of `$77A1`); level->map `$3EF2B -> $0F:$7873` and bell B `$3F5A4 -> $26:$78F0` (overwritten before display). Bell A `$26:$7C91` uploads directly |
| Castle entry, `$0C:$469F..$4700`, `$7000..$7118` | Subtract RGB 16/4/4 from BG+OBJ at timer low 3F/2F/1F/0F; zero forces black. Initializer `$27:$4480` supplies canonical buffers |
| Map exit, `$0F:$69FB..$6A53`, `$7700..$77A1` | Four in-place saturating RGB+10 steps; 36 explicit VBlank waits vs native 48 |
| Zone walkout, `$0F:$62F7..$6367` | 80 walking waits followed by 20 fade waits |
| Map return, `$0F:$6A9A..$6AD7`, `$77A1..$78E5` | Save 128 original BG+OBJ bytes to D180; restore before derived RGB +30/+20/+10/+0; 3 waits per step |
| Credits 7->8, `$1A:$43F8 -> $5E00/$5E6C` | BG5EC1, OBJ5F81; copy $2A0 attribute bytes from $5FC1 to VRAM1 $9800 |
| Credits 8->9, `$1A:$4187 -> $5E10` | BG5F01, OBJ5F81; retain attributes; gate on transfer completion |
| Credits 9->10, `$1A:$4145 -> $5E3E` | BG5F41, OBJ5F81; retain attributes; gate on transfer completion |
| Castle coin flash, `$0C:$7118..$7158` | OBJ flashes 4 frames out of 32 at index $9C: normal words $1A7A,$0085; flash $0B3F,$06B8 |

Bank `$31` is pause-only. All data above already exists in the
cache; no new raw bank/schema is needed for these effects. Ordinary bank1
`$6442` level fades, home `$03AF` ending-entry fade, map walk-in `$0F:$63BA`
and legacy fade-in `$6A68` retain DMG-only writes. Do not invent CGB fades there.

## Gameplay and object differences

Implemented: movement curves and Luigi friction (`ff04d84`, `beecfec`), DX
layouts and spawn positions (`875e815`) and object patches (`6467e18`). Attract
demos always use DX Mario (`6137fc8`).

Mario's curves match base. Luigi friction is 3 instead of 4 (home `$16E9/$16F2`
via `$19FF`), applied at the existing phase even with held input. Preserve
neutral snapping for hvel `$80..$83` and the grounded/swim/moon gates.

| Curve | Base home | DX Mario bank1 | DX Luigi bank1 | Difference |
|---|---|---|---|---|
| Horizontal, 256 bytes |19FF |4000 |42E4 | Luigi indices F7/F8:1->2,2->3 |
| Normal vertical, 76 bytes |18AF |4200 |44E4 | Luigi indices 59..72 decimal: FD->FC |
| Swim vertical, 76 bytes |18FB |424C |4530 | Both match base |
| Moon vertical, 76 bytes |1947 |4298 |457C | Both match base |

Vertical selection hooks: home `$156B/$1574/$157D/$1587`; horizontal:
`$1842/$1851`. Moon wins over swim over normal. H index wraps hvel+(frame&7).
Native vertical arrays include six adjacent overread bytes; import only the 76
actual curve bytes and verify reachable bounds. Shared water-H remains home
`$1AFF..$1BFF`; DX changes only index 0 FF->00, outside ordinary clamps.
Jump/release/acceleration/death control paths match within the ranges asserted
by `audited_dx_character_table_inventory`; this is not whole-ROM equivalence.

Bank `$24:$7D00..$7D60` already contains 32 forced-character FF values, 32 friction
override FF values (fallback 4-character), and 32 autoscroll values 1. The new
`$0DD3 -> $1A2C` scroll hook therefore preserves supported-ROM behavior.
Schema 9 added the narrow H256/V76 curves from bank 1; no DX code is imported.

`audited_dx_content_difference_inventory` confirms 21 changed metatile tables,
14 changed layouts, and collision changes only in levels 01/06/0D/0E/11/1B
(45/768/445/5/1/1 cells). The old 15-layout count was incorrect. Header 1B changes
only metatile-pointer high byte 42->78. All edge/scene maps, properties, secret
entries, collision consumers 1C82..1F51 and script blob 14125..141E5 match base.
Level 0B's last spawn moves Y392->400 (ROM E4F8); all other packed streams match.
Schema 10 imports every packed stream (bank 3 `$6037` pointers). DX sessions
enter each level on DX's layout and spawn positions, so simulation, actors,
arena reloads (level 18's unchanged blob) and rendering share one geometry;
a level entered before DX was enabled keeps native cells until re-entry.

Additional object patches: ROM 5872B/5876A change the parked pose's FFDC
(slot 12) 80->00, and the only two flash routines, 5B690 and boss B413, change
`xor $10` (DMG palette) to `xor $08` (CGB VRAM bank) on FFDE. `ObjectRevision`
carries both through `ObjectSim` and `FrameTick`; DX level entry selects it, and
the draw composes slot 12 ^ flip ^ flash into CGB attributes.

## Verification and limits

Historical independent evidence: one Mario opening frame and title/file-select
frames matched all 23,040 pixels; hub/Tree comparisons have water/decor timing
differences; boss 09 has a partial picture/OAM comparison. Port-to-port surface/
pipe-route regression and 240-tick level render smoke tests do not establish
independent room or full-game parity. Real hardware has not been checked.

Remaining coverage: both characters and Small/Super/Bunny/Fire (space themes on
12/13/1C), all 32 levels and pipe destinations/returns, all 15 map/minigame
screens, attract demos, all boss definitions 0..8, and title through credits.
Boss level/definition pairs: 05/0,09/1,0D/2,10/3,13/4,17/5,18/6..8. Wario 7
inherits 6; 8 patches its VRAM/palettes over that state. Check actual 6->7->8 entry,
not just isolated fixtures. Full RGB comparison, special palette call/reset
paths, uninterrupted progression and mid-game save/reload remain open.

Source assertions live in [contracts.rs](../crates/sml2-dx/tests/contracts.rs);
loader assertions in [revision.rs](../crates/sml2-dx/tests/revision.rs);
render checks/historical RGB hashes in
[dx_preview.rs](../crates/sml2-runtime/tests/dx_preview.rs). Reproduce with
`SML2_DX_ROM` pointing to the supported ROM; `SML2_BASE_ROM` optionally overrides
the sibling base ROM for source-inventory tests:

```sh
cargo test -p sml2-dx --test contracts -- --ignored
cargo test -p sml2-dx --test revision -- --ignored
cargo test -p sml2-dx --test regional_timing -- --ignored
cargo test -p sml2-runtime --test dx_preview -- --ignored
cargo test -p sml2-desktop real_rom_import_restart -- --ignored
```

Phase 1 passed 1,708 core tests, 22 DX unit tests, 2 source-inventory tests and 3
revision tests. The frozen sweep passed with its documented traj4_ff0 exception.
These are regression/source results; they do not close visual or hardware gaps.
