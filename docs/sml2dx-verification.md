# SML2DX verification matrix

Phase-1 inventory, 2026-10-27. Source target: the exact DX SHA-256 documented in
[the audit](sml2dx-audit.md); contracts and exclusive source ranges are in
[the source contracts](sml2dx-contracts.md). ROMs and reference images stay local.

This matrix distinguishes source assertions, native unit/render tests and
independent evidence. A smoke render is not a playthrough or a pixel comparison.
The baseline plan was `43588db`; regional selector implementation `291981a`
landed separately during this audit. Its branch tests are implemented, while
source-order and independent palette evidence below remain open. This phase
adds source-inventory tests only and does not certify that separate change.

## Test and evidence keys

| Key | Existing test/source | What it establishes |
|---|---|---|
| C | `sml2-dx/tests/contracts.rs::audited_dx_content_difference_inventory` | Both ROM identities, all level layouts/classes/metatiles, headers, spawns, edge/scene maps and unchanged collision/script ranges |
| H | `contracts.rs::audited_dx_character_table_inventory` | Exact per-character curve differences, configuration defaults and bounded unchanged source |
| L | `sml2-dx/tests/revision.rs::every_level_derives_from_the_dx_loader_tables` | Importer derives all 32 levels from their own DX sources |
| R | `sml2-runtime/tests/dx_preview.rs::audited_rom_draws_every_level_and_its_arenas` | Both graphics characters render during a 240-tick native run per level; explicitly enters arena $09 only |
| P | `session/dx_palette.rs` unit tests and `dx_preview.rs::audited_rom_regional_palettes_follow_mario` | Implemented regional branch/color checks |
| F | `dx_preview.rs::audited_rom_forms_effects_and_initial_frame` | Imported forms/effects; audit records one independent Mario opening frame |
| T | `dx_preview.rs::audited_rom_title_and_file_select_match_their_pyboy_frames` | Two historical independent Mario RGB hashes |
| M | `dx_preview.rs::audited_rom_draws_the_hub_and_zone_screens` | Both graphics characters, screens 0..8; historical partial hub/Tree comparison |
| B | `sml2-dx/src/bundle_tests.rs::each_boss_arena_reads_its_loader_sources` | Synthetic arena table derivation including Wario 6/7/8, not independent pictures |

Paths without crate prefixes above are relative to the named crate. R/F/M/T are
existing opt-in tests; historical independent results are from the earlier audit,
not a fresh emulator run. All rows require Mario AND Luigi. Level rows require
Small/Super/Bunny/Fire; alternate space graphics additionally apply to
$12/$13/$1C. Physics remains native until the gameplay tickets pass.

## All levels and rooms

IDs are zero-based hexadecimal; display level number is ID+1. Counts are changed
metatile bytes / decompressed layout cells / collision-class cells against base.
All class changes cross solid/non-solid boundaries. Scene IDs are distinct
nonzero entries from that level's 48-byte SceneMap. They are trigger IDs, not a
count of pipe rooms: same-layout pipe travel can occur without a nonzero scene.

Every row has C/L/R source or smoke coverage. Its `dx-room-ID` acceptance case
must enumerate pipe mouths/destinations and scene transitions from layout markers,
scene instructions and live entry state, then test entry, room view, return,
mutation and reload. Those played/visual room cases remain pending for every row;
the audit's opening surface/pipe route regression compares two port renderers,
not an independent DX room reference. Do not mark a room checked from R alone.

| ID | Counts | Scene IDs | Regional source | Independent evidence still needed |
|---|---|---|---|---|
| $00 | 0/0/0 | — | default | Mario opening frame recorded; remaining frames/rooms/forms and Luigi pending |
| $01 | 8/3473/45 | — | default | Both characters, rooms, forms and transitions pending |
| $02 | 0/0/0 | 06,07 | default | Both characters, rooms, forms and transitions pending |
| $03 | 21/39/0 | — | 26:6D1A | Both characters, rooms, forms and transitions pending |
| $04 | 10/301/0 | — | default | Both characters, rooms, forms and transitions pending |
| $05 | 11/128/0 | 0B | default | Both characters, rooms, forms and transitions pending |
| $06 | 4/790/768 | 02 | default | Both characters, rooms, forms and transitions pending |
| $07 | 20/0/0 | 01,02 | 26:6DFD | Both characters, rooms, forms and transitions pending |
| $08 | 10/0/0 | — | default | Both characters, rooms, forms and transitions pending |
| $09 | 22/0/0 | 0C | default | Both characters, rooms, forms and transitions pending |
| $0A | 0/0/0 | 11,12 | default | Both characters, rooms, forms and transitions pending |
| $0B | 79/773/0 | — | default | Both characters, rooms, forms and transitions pending |
| $0C | 0/256/0 | — | 26:6B39 | Both characters, rooms, forms and transitions pending |
| $0D | 118/1739/445 | 10 | 26:6EB3 | Both characters, rooms, forms and transitions pending |
| $0E | 101/184/5 | — | default | Both characters, rooms, forms and transitions pending |
| $0F | 0/0/0 | 17,18 | default | Both characters, rooms, forms and transitions pending |
| $10 | 0/0/0 | 0E | default | Both characters, rooms, forms and transitions pending |
| $11 | 101/372/1 | 02 | default | Both characters, rooms, forms and transitions pending |
| $12 | 8/0/0 | — | default | Both characters, rooms, forms and transitions pending |
| $13 | 8/0/0 | 0D | default | Both characters, rooms, forms and transitions pending |
| $14 | 24/4402/0 | 19,1A | default | Both characters, rooms, forms and transitions pending |
| $15 | 20/470/0 | — | 26:6BC4 | Both characters, rooms, forms and transitions pending |
| $16 | 0/0/0 | — | 26:6E5E | Both characters, rooms, forms and transitions pending |
| $17 | 2/0/0 | 0F | default | Both characters, rooms, forms and transitions pending |
| $18 | 0/0/0 | 03,04,08,09,13,1B | 26:6FBC | Both characters, rooms, forms and transitions pending |
| $19 | 0/0/0 | — | 26:6C5B | Both characters, rooms, forms and transitions pending |
| $1A | 0/0/0 | — | default | Both characters, rooms, forms and transitions pending |
| $1B | 40/347/1 | — | default | Both characters, rooms, forms and transitions pending |
| $1C | 8/0/0 | — | default | Both characters, rooms, forms and transitions pending |
| $1D | 8/1147/0 | — | default | Both characters, rooms, forms and transitions pending |
| $1E | 11/0/0 | — | default | Both characters, rooms, forms and transitions pending |
| $1F | 0/0/0 | — | default | Both characters, rooms, forms and transitions pending |

Geometry tickets `dx-geometry-ID` first cover $01/$06/$0D/$0E/$11/$1B;
remaining changed-layout/metatile rows follow individually. C confirms **21**
changed metatile tables and **14** changed decompressed layouts. The old audit's
15-layout count was incorrect. Header $1B changes only field+$0C (metatile
pointer high byte $42->$78). Level $0B's final spawn moves Y392->400;
all other packed spawn streams, all edge/scene maps and level properties match.

## Maps and minigames

Map palette/attribute source table: audit's Overworld section; all rows require
both characters, native and enlarged views, entry/exit and pause where applicable.
Use `dx-map-SCREEN` for each outstanding comparison/integration case.

| Screen | Source | Current test | Independent evidence / pending work |
|---|---|---|---|
| 0 hub | 26:77F1; 29:6542 | M | Partial historical comparison; both camera palettes, water/decor timing and return fades pending |
| 1 Pumpkin | 26:7969 branch; 29:4B40 | M | Pending both characters and decor animation |
| 2 Tree | 26:7969 branches; 29:4000 | M | Partial historical comparison; bird/decor/water timing pending |
| 3 Macro | 29:4240 | M | Pending |
| 4 Space | 29:46C0/4900 | M | Pending before/after path unlock |
| 5 Turtle | 29:4480 | M | Pending |
| 6 Mario | 29:4D80 | M | Pending |
| 7 Wario castle | 29:5200 | M | Pending including sky bands |
| 8 castle hill | 29:4FC0 | M | Pending |
| 9 coin room | 26:765A; 37:7340 | `both_minigames_draw_in_dx_color` | Each prize/door, white entry and exit pending |
| 10 coin room | 26:765A; 37:7100 | importer table path | Each prize/door and transitions pending |
| 11 coin room | 26:765A; 37:6EC0 | importer table path | Each prize/door and transitions pending |
| 12 coin room | 26:765A; 37:6C80 | importer table path | Each prize/door and transitions pending |
| 13 bell room | 26:78F0/7C91; 37:6800 | `both_minigames_draw_in_dx_color` | Both entry routes, rewards and transitions pending |
| 14 bell room | 26:78F0/7C91; 37:6A40 | importer table path | Both entry routes, rewards and transitions pending |

## Bosses and fixed scenes

All boss rows require Small/Super/Bunny/Fire for both characters, load/blank,
actual arena entry, inherited palette state, direct OBJ effects, defeat and exit.
B tests data shape; R does not prove all boss entries. Use one `dx-boss-DEF`
verification ticket for each definition below, with transition cases as named.

| Scene / definition | Source | Existing evidence | Still required |
|---|---|---|---|
| Tree $05 / def0 | 36:7306/7654; 26:70DA/711A | B | Independent picture and live entry/defeat |
| Pumpkin $09 / def1 | 36:7306/7654; 26:72DA/731A | B/R; historical partial picture/OAM comparison | Independent picture and live entry/defeat |
| Mario $0D / def2 | 36:7306/7654; 26:735A/739A | B | Independent picture; regional selector/arena order |
| Turtle $10 / def3 | 36:7306/7654; 26:725A/729A | B | Same-state picture, both characters and defeat |
| Macro $13 / def4 | 36:7306/7654; 26:715A/719A | B | Alternate theme, picture and live entry/defeat |
| Space $17 / def5 | 36:7306/7654; 26:71DA/721A | B | Independent picture and live entry/defeat |
| Wario $18 / def6 | 36:7443/7554; 26:73DA/741A | B | Independent picture and entry; selector interaction |
| Wario def7 + 6->7 | 36:7435, inherits6 | B synthetic inheritance | Real phase inheritance, picture and defeat |
| Wario def8 + 7->8 | 36:74A4/74FE | B synthetic copies/patches | Real phase inheritance, picture and defeat |
| Title | 20:6A40/6A66 | T, Mario hash | Luigi, white startup and subsequent transitions |
| File select + character door | 25:4234; 0C:7158/7186 | T; session file-select-character tests | Luigi, transition timing, persisted physics choice |
| Castle reveal/door | 27:4480; 0C:469F/706B | `castle_reveal_and_game_over_use_their_own_dx_palettes` | Both characters, exact black fade and coin flash |
| Game Over | 27:4505 | same synthetic palette test | Both characters, live entry and return; distinguish DMG-only effects |
| Congratulations / ending entry | 29:6718; 1A:4656/5096 | table/renderer support | Both characters and same-state reference |
| Ending 7->8 | 1A:43F8/5E00/5E6C | Source only | BG/OBJ changes, $2A0 attributes, exact transition |
| Ending 8->9 | 1A:4187/5E10 | Source only | New BG, OBJ retention, transfer completion gate |
| Ending 9->10 | 1A:4145/5E3E | Source only | New BG, OBJ retention, transfer completion gate |
| Credits scrolling -> terminal | 1A:4016/4267 | renderer support; base terminal tests | Both characters, palette retention, wobble end and uninterrupted completion |

Attract demos: source/renderer support exists; add `dx-attract` for active mode,
character/content selection, timer/scene state and presentation timing. Existing
native demo playback does not establish DX gameplay equivalence.

## Bounded tickets and acceptance mapping

All tickets are pending except existing tests explicitly named above. Allowed
files below are owner directories, not blanket editing permission: before each
ticket, name its exact native module, adapter, tests and registration file as
required by PORTING.md. Each selector, room, geometry level, scene and boss gets
a separate bounded case/ticket. Do not assign this entire table to one worker.

| Ticket | Source contract | Allowed owners | Required independent assertions |
|---|---|---|---|
| dx-rgb-bridge | Current grayscale bridge/compare_tas | runtime host/bridge; compare tool/tests | Known RGB pixel, payload length/truncation, mode identity and state alignment |
| dx-palette-order | 01:54CF; home0618/0636; 02:4100; 24:793C | runtime palette state; level-frame phase/tests | Previous object-pass refs, wrap, pause/freeze, special22C4/2304 calls, real reload reset |
| dx-region-ID | Each of eight exact selector ranges in contracts | focused palette module + render/tests | Thresholds, shared latch, $15 fallthrough, $19 skipped trigger, direct OBJ writes; re-audit existing291981a tests before changes |
| dx-palette-jobs | 24:7800..7912 | native effect state/scene consumers | One dispatch, job chaining/clear, original vs temporary buffers, no render mutation |
| dx-map-brighten | 0F:7700..77A1 | native RGB helper/tests | Saturating +10 on32 BG+OBJ colors; exact first/final bytes |
| dx-map-exit | 0F:69FB..6A53 | map transition owner/tests | Four source stages,36 explicit waits; vanilla48 retained |
| dx-zone-walkout | 0F:62F7..6367 | map transition owner/tests |80 walk waits,20 fade waits, unchanged DMG-only walk-in |
| dx-map-return | 0F:6A9A/77A1..78E5 | map effect state/transition/tests | Backup/restore original; +30,+20,+10,+0 after3 waits each |
| dx-castle-black | 0C:469F/7000..7118 | castle state/effects/tests | Timer low3F/2F/1F/0F, RGB subtraction16/4/4; terminal black and visit reset |
| dx-white-SCENE | 20:6A66;26:765A/78F0;36:7746 | one scene owner + renderer/tests | White D300, BG+OBJ ordering, actual route-specific load behavior |
| dx-credits-7-8 /8-9 /9-10 | Three hooks in scene table above | ending native state + dx_ending/tests | Actual gate/transition, exact palettes, attributes, retention and read-only drawing |
| dx-castle-flash | 0C:7118..7158 | castle color state/render/tests |4-of32 frames; exact OBJ words, restore, both characters |
| dx-gameplay-selection | homeFFF6 + session lifecycle | runtime content/session/frame/host; desktop wiring | Safe level-entry activation, doors/settings, restart, tapes, render fallback retains collision |
| dx-physics-import | 01:4000/4200/42E4/44E4/424C/4298;24:7D00 | DX bundle/presentation/cache/tests | Narrow curves: H256,V76; defaults; reject bad shape; schema/re-import roundtrip |
| dx-character-friction | home16CC/19FF..1A2C | focused player module/wiring/tests | Mario4/Luigi3, neutral snap80..83, gates, held input and ordering |
| dx-character-displacement | home156B/1587/1842/1851 | player movement tables/wiring/tests | H F7/F8 and phases, V59..72, both jump routes; shared water/moon; valid bounds |
| dx-profile-regressions | H plus unchanged source ranges | focused core/runtime tests | Mario/base equality, default autoscroll1, mode/save identity, render-frequency independence |
| dx-spawn-0B | ROM E4F8, same E037 pointers | DX content import/runtime spawn selection/tests | Last Y392->400 only; all other decoded fields and streams preserved |
| dx-geometry-ID / dx-room-ID | C; header/SceneMap/layout markers | one level content/reload/terrain case + DX render/tests | Same geometry for player/actors/draw; doors, mutations, arena/room return |
| dx-parked-priority-58712 /58758 | ROM5872B/5876A:80->00 | named handler boundary or presentation projection/tests | Exact FFDC/slot12 and CGB priority after staging; vanilla behavior retained |
| dx-object-flash | ROM5B693 and B416:10->08 | focused item/boss presentation adapter/tests | CGB VRAM bank toggle, fourth/even frame gates, preserve other attribute bits |
| dx-boss-DEF / dx-map-SCREEN / dx-attract | Tables above | one named scene/arena + tests | Same-state picture, branch/frame bytes, both characters; no smoke-only closure |
| dx-final-routes | Audited active gameplay/content | existing native route harness/docs | Both characters from new file through six coins, Wario and credits; mid-game save/reload |

## Data/import decision and open evidence

Regional selectors, fade arithmetic, white/black boundaries, castle flash and
credits sources already fit schema8's42 banks. They need native behavior and
accessors, not a new raw bank31. The bank1 character movement tables are absent;
extract narrow validated slices and bump schema/importer/cache identity when
that persisted physics contract is implemented. Bank24's96-byte configuration
is already present: forced-character/friction overrides allFF, autoscroll all1.
The $0B spawn patch can be a narrow extracted datum/decoded record; do not add
bank3's artwork/code merely to obtain one changed Y coordinate.

Static audits do not prove arbitrary indirect palette resets, unusual special
pose-call timing, or whole-ROM equivalence. The matrix retains those questions
in dx-palette-order, room/boss and final-route cases. Full-game independent RGB
and hardware comparisons remain open. Existing vanilla route milestones and
accepted presentation-latency differences are separate; no new capture or
trajectory script is authorized by this inventory.
