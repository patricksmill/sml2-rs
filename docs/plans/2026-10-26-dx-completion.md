# Complete SML2DX compatibility

Proposed implementation plan, 2026-10-26. This document does not claim that any
of the work below has been implemented or verified.

## Outcome and scope

DX Mode reproduces the supported DX ROM's regional colors, palette fades,
character movement and level geometry, with explicit evidence for boss phases
and the rest of the game's presentation. Both Mario and Luigi run through the
native Rust simulation. The exact currently supported ROM digest remains the
compatibility target; supporting additional DX revisions is separate work.

This closes all gaps identified in [DX compatibility](../desktop.md#sml2dx-mode)
and the [DX audit](../sml2dx-audit.md), including limitations currently described
as unverified. The first task must check for further DX gameplay differences so
that completing a shortlist does not become a false full-compatibility claim.

Source authority and implementation rules are [PORTING.md](../../PORTING.md),
the [native Rust guide](../idiomatic-rust-porting.md), and bounded comparisons
between the audited DX ROM and `../sml2decomp/` plus its base ROM. The two
2026-10-15 workflow plans referenced by `AGENTS.md` are absent from this checkout;
this plan does not assume their contents. Existing vanilla route and visual
milestones remain separate and must not be closed by DX evidence alone.

## Current implementation boundaries

| Area | Starting point | Remaining work |
|---|---|---|
| Regional palettes | `session/level.rs` retains a selector for `$0C`; `render/dx.rs` reads its BG set | Add `$03`, `$07`, `$0D`, `$15`, `$16`, `$18`, `$19`, preserving `$0C` |
| Fades | DX uses CGB palettes; native fade timers and scene transitions already exist | Audit DX's palette jobs and reproduce their colors, order and cadence |
| Physics | Character choice selects graphics; `PlayerSim` uses native rules | Audit and port every character-dependent movement difference |
| Geometry | `sml2-dx::LevelGraphics` imports DX layouts and metatiles | Feed DX geometry into simulation and rendering together; remove the collision-class substitution |
| Bosses | Arena tables and Wario definitions 6/7/8 are implemented | Independently verify every arena and each Wario form/transition; fix demonstrated differences |
| Pixel comparison | `tools/compare_tas.py::read_frame` reads one grayscale byte per pixel | Add a DX RGB path with explicit mode, character and state alignment |

Runtime paths in this table are relative to `crates/sml2-runtime/src/`.
The audit reports changed metatiles in 21 levels and layouts in 15; six levels
also move solid cells: `$01`, `$06`, `$0D`, `$0E`, `$11`, `$1B`. Recompute these
inventories against the supported ROM before using them as acceptance counts.

## Work order

Execute one bounded ticket at a time, with a focused commit after verification.
Each numbered phase below is a work package, not a single large worker task.
Before dispatch, give each ticket exact allowed files, bounded source offsets,
branch cases, prerequisites and its acceptance command. Split native modules
before adding logic to files over the project's size limit. Supervisors use the
worker and acceptance loop in [ORCHESTRATION.md](../../ORCHESTRATION.md).

### 1. Establish the DX contract and evidence inventory

1. Confirm access to the audited local DX ROM and base ROM by digest. Inspect
   bounded changed ranges and their callers; identify palette jobs, character
   physics hooks/tables, geometry/collision tables, level headers, spawns and
   scene scripts. Classify each relevant difference as implemented, missing,
   unchanged or not yet understood. Do not infer Luigi's constants from how
   Luigi behaves in other Mario games.
2. Expand `docs/sml2dx-audit.md` with source ranges and branch contracts for the
   seven missing selectors and every fade entry point. Record coordinate
   offsets, byte wrapping, comparisons, retained state and reset conditions.
   Distinguish screen loads, palette effects and simulation changes.
3. Create a compact DX verification matrix in `docs/` covering all 32 level
   IDs, pipe rooms, changed geometry, all map/minigame screens, each boss
   definition and transition, title/file select/character door, castle reveal,
   Game Over, ending and credits. Track both characters and relevant forms.
   Each row names source evidence, a test and any independent comparison still
   needed. Include overworld water/decor animation timing already called out
   in the DX audit.
4. Resolve the data requirements before changing the importer. Existing bundles
   already contain bank `$26` and DX layouts. Fade code in bank `$31` and any
   physics data require an explicit audit: extract only needed data, never
   execute patched cartridge code. Bump importer/schema versions only when
   the persisted contract changes, with cache invalidation and re-import tests.

Acceptance: every known gap has source references and an independently testable
ticket. Newly discovered behavior differences become prerequisites in this plan;
unresolved source questions remain open, rather than becoming guessed behavior.

### 2. Make DX comparisons useful

Extend the existing host/frame bridge and `tools/compare_tas.py` with a declared
RGB format and explicit DX/character selection. Keep the existing grayscale
protocol working. Test payload sizes, truncated reads and a known color pixel.
Support the new data in the existing comparison/reporting path rather than
creating a second replay engine.

Comparison reports must identify ROM digest, mode, character, level/scene,
player and camera coordinates, animation phase, palette selector and transition
state. Normalize RGB555 expansion consistently. Align by the state that authored
the frame; respect documented native presentation latency and load differences
without shifting away a wrong fade or movement update. Report mismatching pixels
and the first differing state. Never silently ignore a screen region.

Use synthetic fixtures and existing permitted reference material. `PORTING.md`
forbids new trajectory capture scripts and steering PyBoy to obtain captures;
this phase does not authorize either. Bounded static ROM analysis and in-memory
tests are the primary gates. Any unresolved instruction may use only the policy's
allowed once-only, under-ten-frame PyBoy memory probe. Missing independent
reference evidence stays explicitly pending; hardware/emulator claims require
the corresponding evidence, not just internally consistent port screenshots.

Acceptance: the tool distinguishes a deliberately wrong RGB palette from a
correct frame, and existing vanilla comparison callers still work.

### 3. Finish regional palettes

Extract the existing `$0C` selector into a small native palette-state module,
then implement one missing level per ticket: `$03`, `$07`, `$0D`, `$15`, `$16`,
`$18`, `$19`. Use the audited `$26:$6B0B` dispatcher and each level's bounded
branch. Preserve Mario X+8/Y+32 semantics and `$FFAD` retention. Update state once
at the source-defined simulation point; repeated rendering must not advance it.

Replace the conveyor-only render special case with the selected palette source.
Preserve the source's BG/OBJ distinctions and boss-arena palette precedence.
Test entry, each threshold just below/at/above, movement in both directions,
unmatched corridors, coordinate wrapping, pause/unpause, room changes, death and
reload. Retain the existing `$0C` boundary regression. Renderer tests assert
actual colors for both characters, plus unchanged sprite colors when only BG
changes.

Primary files: a new focused module beside `session/level.rs`, its tests,
`render/dx.rs` wiring and `tests/dx_preview.rs`. Use existing imported palette
bytes unless phase 1 proves more data is needed.

Acceptance: all eight selectors match source-derived state and palette-byte
expectations; the seven missing levels no longer keep their starting colors.

### 4. Reproduce DX fades and effect ordering

Audit palette job selection, the `$D300` blank buffer, target buffers and RGB555
channel arithmetic. Reuse native transition signals where their contracts match.
Represent only additional persistent state the DX source requires. Advance it
in the simulation/scene update, and pass the resolved palettes to rendering.
Do not approximate DX fades by applying DMG palette shadows or an arbitrary
opacity curve to the completed frame.

Split implementation into tickets by palette operation, then scene family:
level/pipe/arena loads and exits; death/Game Over; title/file select/map and
minigames; castle reveal/ending/credits. Verify the source actually uses a fade
at each entry before adding one. Specify regional palette, boss palette, pause,
star and fade ordering from source, including interrupted/restarted transitions.

Test exact palette bytes at the first, intermediate and terminal steps, cadence,
saturation, restoration and interrupted fades. Cover both BG and OBJ behavior,
both characters, paused/scene-owned frames, enlarged views, and zero/one/multiple
render calls per tick. Update separate scene renderers such as `dx_ending.rs`
through the same palette resolver where the source shares behavior.

Acceptance: every audited fade path has byte and rendered-color assertions;
no DMG filter accidentally recolors the DX picture.

### 5. Add DX gameplay selection and character physics

First establish a typed gameplay selection at the session/content boundary.
Core movement accepts native rules/data without depending on the desktop app,
ROM reader or renderer. Extend the existing shared level-frame path so sessions
and headless diagnostics use the same behavior. Frozen vanilla replay keeps its
existing mode and observation contract.

Proposed user behavior: enabling DX selects its gameplay as well as its art;
the existing Character setting and file-select door select the corresponding
physics. Apply a change of geometry/gameplay at the next safe level entry or
reload, with clear pending-setting feedback. Never replace collision terrain
under a running player. Latch the active selection for that level; save/restart
and death/room reloads must retain it consistently. A rendering fallback changes
only presentation, retaining the active gameplay and collision geometry.

Use the phase-1 audit to choose the smallest physics implementation: shared
movement with audited character parameters where possible, explicit native
branches only where control flow differs. Split tickets by the actual differing
movement operation. Cover acceleration, braking/skidding, speed limits, jumping
and early release, falling/landing, swimming, low gravity, Bunny/Fire forms,
platform contact and pipe/scene movement wherever the audit finds a difference.
Prove unchanged cases too; selecting Mario must not imply a DX movement change
unless the supported ROM contains one.

Likely owners: `sml2-core/src/player_sim.rs`, focused modules under
`sml2-core/src/player/`, runtime `session.rs`, `session/level.rs`, shared
`game/level_frame/`, host/CLI and desktop startup/settings consumers. Give each
ticket a smaller explicit file list after tracing callers. Verify settings,
file-select switching, attract demos, diagnostic play and tape mode identity;
do not silently reinterpret an existing vanilla tape as DX.

Acceptance: ROM-derived byte assertions cover each changed movement branch and
its frame boundaries; vanilla tests/replays retain their existing results.

### 6. Use DX geometry throughout the native game

Build a DX level-content view from the already imported layout, metatiles and
the audited headers/collision data. Rendering, player probes, actor terrain
queries, spawning, camera/scene boundaries and room/arena reloads must consume
the same active content. Audit object placement, scripts and collision-class
tables rather than assuming that changed graphics are the only changed data.

Start with one of the six levels with moved solid cells. Verify a complete
vertical slice: load, collision probe, player and enemy contact, draw, block/coin
mutation and reload. Then cover the other five, followed by every other imported
layout/metatile difference. Preserve live mutations, pipe destinations,
checkpoints/secret entries, breakable blocks, collected coins and arena swaps.
Remove `render/dx.rs::dx_cell`'s native collision-class substitution only after
the active simulation uses that same DX content; then draw the live DX layout.

Primary owners: DX extraction/validation only if required; runtime content and
level initialization, shared scene reloads, terrain adapters and DX render
wiring. Add small source-derived tests per affected level and shared operation.

Acceptance: every changed cell is inventoried, the six collision-changing levels
have explicit probe/mutation tests, and displayed ground matches player and
object collision in both modes and both characters. Schema/cache changes, if
needed, pass import, restart and failed-publication tests.

### 7. Close boss and full-game verification gaps

Verify every boss arena's tiles, attributes, palettes, sprite lists and theme
changes. Give Wario definitions 6, 7 and 8 separate cases, plus both transitions:
7 inherits 6's appearance, and 8 applies its VRAM0/palette patches over that
state. Test entry from the actual previous phase as well as isolated state, so
inheritance cannot accidentally depend on a test fixture's defaults. Include
Luigi, Fire/Bunny/space themes where applicable, pause, fade and death/reload.

Complete the phase-1 matrix across all levels, rooms, maps, minigames and fixed
scenes. Check animation period/order and document independently aligned phases;
resolve unexplained water/decor differences rather than automatically treating
them as acceptable. Use native 160x144 comparisons first, then enlarged-view
regressions. Distinguish source/unit verification, independent emulator
comparison and real-hardware comparison in the evidence record.

Finish with played progression for each character: new file, zone navigation,
all six coins, Wario's three forms and credits, including save/reload and mode
changes at the agreed boundary. Existing vanilla full-playthrough prerequisites
still apply. Every demonstrated discrepancy becomes a bounded correction ticket;
every missing reference remains an open verification row.

Acceptance: every matrix row has its required evidence, both character runs
finish, and no known DX behavior or presentation gap is left undocumented.
Do not claim full-game pixel parity from representative frames alone.

## Gates and closeout

For each behavior ticket, run focused tests and `cargo test -p sml2-core`.
Require the source-derived branch tests to fail when the relevant implementation
line is reverted. At each work-package boundary, run affected crate suites, then
the frozen sweep once after tests pass:

```sh
cargo test -p sml2-core -p sml2-dx -p sml2-runtime -p sml2-desktop
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

At importer, integration and final closeout boundaries, run the opt-in ROM tests
with `SML2_DX_ROM` pointing to the audited local ROM:

```sh
cargo test -p sml2-dx --test revision -- --ignored
cargo test -p sml2-runtime --test dx_preview -- --ignored
cargo test -p sml2-desktop real_rom_import_restart -- --ignored
```

Capture sweeps establish regression only; byte assertions establish behavior;
independent RGB comparisons establish the visual claims they actually cover.
Do not write ROMs, extracted assets or new captures into Git. Keep evidence
artifacts local and commit only permitted tests, tooling and concise results.

Closeout updates `docs/desktop.md`, `docs/sml2dx-audit.md`, README compatibility
wording, and stale presentation-only API comments to match verified behavior.
Preserve unrelated local edits. Keep this DX plan separate from the live vanilla
queue; only add an individual pending queue ticket if execution uses that queue,
and mark it done after its tests and evidence pass. A missing local ROM or
independent reference blocks the associated acceptance claim, not unrelated
implementation and unit testing.
