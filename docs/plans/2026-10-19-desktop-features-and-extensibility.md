# Desktop features and future extensibility

Planning proposal, 2026-10-19. This document specifies future work; it does not
claim these features are implemented or change the active port queue. Original
game completion still follows [the completion workflow](2026-10-15-rust-completion-workflow.md).
Implement each architectural move as a bounded, independently verified step.

Package A's runtime boundaries are implemented: a reusable library, separate
CPU rendering and window-buffer presentation, an injectable preloaded content
source, and explicit session inputs/deferred host effects. The CLI adapters
retain their existing behavior. See [Runtime boundaries](../runtime-architecture.md)
for ownership, the public API, compatibility contracts, and verification.

## Assessment from the current code

The three-crate workspace already separates much native simulation from the
desktop executable and extraction tool. Preserve the typed actor work and the
raw-slot compatibility boundary described in [the Rust guide](../idiomatic-rust-porting.md).

The main obstacles are ownership and dependencies around that simulation:

- `sml2-runtime/src/main.rs` contains CLI handling, filesystem loaders, rendering,
  replay verification, and a separate level-play loop. `session.rs` adds the
  overworld/session loop, direct keyboard polling, persistence, and live audio.
- Both window paths already allow resizing and implement integer scaling with
  letterboxing. `--scale` sets initial output size; `--size` selects the logical
  viewport. Runtime controls and persisted preferences remain to be added.
- `SessionRunner::step_frame` is a useful starting seam, but also loads levels,
  writes saves, generates audio, and samples different clocks in live/headless
  modes. Its whole struct cannot serve as a portable simulation snapshot.
- `render_view` resolves tiles and palette indices, then returns grayscale
  `Vec<u8>`. Presentation expands those bytes to RGB. There is no general
  texture-pack resolver or CGB color compositor yet.
- Extraction reads `baserom.gb` **and** assembly/source assets from the sibling
  decompilation. The desktop runtime reads a working-directory `extracted/` tree.
  A ROM picker alone cannot replace that dependency.
- `PlayerSim` holds player movement alongside level timers, scene/progression
  state, and sound requests. `ObjectSim` has a single `MarioContact` and player
  reference. Simultaneous co-op requires separating those responsibilities.
- `audio_output.rs` launches `pacat` or `pw-play`; desktop portability also
  requires an in-process audio backend or equivalent platform implementations.

The standing goals in [DESIGN.md](../../DESIGN.md) already anticipate texture
packs, interpolation, variable viewport sizes, and DX. They are contracts to
preserve, rather than evidence that those systems are finished. In particular,
later camera notes and the current renderer supersede its early viewport-clamp
description.

## Feature feasibility

Effort is relative engineering scope, not a calendar estimate. DX estimates need
an audit of a user-provided target revision; multiplayer estimates need rules.

| Feature | Feasibility / scope | First useful delivery | Main dependency |
|---|---|---|---|
| Resizable window | Already supported; small polish task | Persist size, fullscreen, DPI handling, minimum size | Shared presentation code |
| Output zoom | Straightforward; small | 1x–8x presets, auto-fit, shortcuts | Display settings |
| Wider viewport / world zoom | Feasible; medium and behavior-sensitive | Optional larger view with native gameplay bounds | World-space render data |
| Debug menu | Straightforward; medium | Pause/step, collision overlay, actor inspector, level jump | Inspection and command APIs |
| Filters and palette presets | Straightforward for simple filters; medium for shader chains | Nearest, linear, grayscale/green palettes, optional LCD effect | Color-capable presentation |
| ROM import and settings menu | Feasible; medium UI plus substantial importer work | Import one supported base revision, cache it, launch/resume | ROM-only importer and storage paths |
| Key rebinding | Straightforward; small–medium | Saved gameplay/UI bindings and reset defaults | Action-based input |
| Controllers | Straightforward; medium including device behavior | D-pad/stick, hotplug, remapping, menu navigation | Action-based input and device assignment |
| SML2 DX | Feasible per audited revision; large | One revision's graphics, then character/rules parity | Versioned importer, CGB attributes, rules profiles |
| Ghosts / asynchronous races | Feasible; medium after replay cleanup | Replay another run alongside the player | Versioned input/state recordings |
| Simultaneous local co-op | Feasible; very large | Two players sharing one camera and world | Per-player state and explicit co-op rules |
| Online play | Feasible; very large after local play works | Direct-connect synchronized sessions | Determinism, snapshots, network protocol |
| Level editor | Feasible; large, naturally incremental | Tile/spawn editing and playtest using existing mechanics | Editable level schema and asset injection |

## Target boundaries

Start by extracting modules and a library from the existing runtime; create
additional crates only when there are actual shared consumers. Keep the CLI
renderer, replay verifier, and headless runner available throughout.

```text
Desktop app: window, menus, settings, device events, import progress
    | actions / typed commands                  ^ immutable view / events
    v                                           |
Session simulation: world, players, camera, scenes, deterministic sound driver
    | reads immutable GameContent               | emits FrameEffects
    v                                           v
Content provider                         Host services
  base-ROM import                          save writes
  DX revision import                       audio device
  level-project overlay                    network transport
  art-pack overlay                         file dialogs

Read-only render snapshot -> CPU game renderer -> color frame
                                             -> GPU filters/scaling -> UI
```

Proposed responsibilities:

- `app`: launcher/settings screens, in-game overlay, editor screen, UI focus.
  App screens are separate from the ROM's game states and original pause logic.
- `input`: bindings, device/player assignment, action transitions, input tapes.
- `session`: tick state and transitions, independent of windows, paths and devices.
  Move pure coordination toward `sml2-core` incrementally, preserving ordering.
- `content`: validated immutable levels/tables/art plus explicit content identity.
  Initially adapt the existing extracted directory behind this API.
- `render`: CPU composition and immutable render snapshots; `present`: window,
  GPU upload, postprocessing, scaling and overlay composition.
- `storage`: settings migrations, content cache, progress saves, level projects.
  ROM-style progress records and complete runtime snapshots have separate formats.
- `debug`: typed inspection data and commands shared by GUI and headless tools.

A useful eventual API is conceptually
`advance(state, content, tick_input, commands) -> FrameEffects`, with rendering
reading a snapshot. `tick_input` contains the tick number, per-player held
buttons, and any explicitly recorded external clock samples. Host services apply
effects outside the simulation. Preload/cache the content needed for level
transitions so stepping and restoring do not require filesystem access.

### Desktop libraries

Recommended direction: `winit` for window/events, `wgpu` for presenting the game
texture and filter passes, `egui` for menus/debug/editor panels, and `gilrs` for
controllers. `egui` has official `egui-winit` and `egui-wgpu` integrations; pin a
compatible set of dependencies when implementing. A richer graphics stack adds
initialization and platform build work, so first present the existing CPU output
unchanged. There is no requirement to move tile composition onto the GPU.

Use an in-process audio backend such as `cpal` behind `AudioSink`, with buffering
and resampling outside the simulation. Check chosen target platforms in a small
integration prototype before committing to the dependency set. Preserve a
headless CPU path for tests and PNG output.

Primary references: [winit](https://github.com/rust-windowing/winit),
[wgpu](https://github.com/gfx-rs/wgpu),
[egui integrations](https://github.com/emilk/egui#integrations),
[gilrs](https://docs.rs/gilrs/latest/gilrs/),
[cpal](https://github.com/RustAudio/cpal).

## Window, zoom, rendering and filters

Keep four independent concepts in `DisplaySettings`:

1. Window dimensions in physical pixels, plus DPI-scaled UI dimensions.
2. Logical game viewport, with 160x144 as the compatibility default.
3. Output scale: auto integer fit, selected integer scale, or optional fractional fit.
4. Art/internal resolution multiplier for future higher-resolution packs.

Changing output scale should resize the window or game rectangle, without
changing the camera or visible world. Define the behavior of a manual preset
versus auto-fit explicitly. Enforce a suitable minimum window size in integer
mode; the current scale clamp to 1 can crop a window smaller than the viewport.
Handle minimized/zero-size surfaces and DPI changes without advancing simulation
as a consequence of presentation events.

Changing the logical viewport is a separate feature. Existing `--size` is useful,
but object activation, despawn, camera triggers and source screen-byte projection
are tied to native bounds. A first wider view must preserve those gameplay
bounds, which can expose inactive enemies or abrupt appearance outside them.
Draw extended views from world coordinates; do not stretch wrapped OAM bytes.
Changing activation or AI range belongs to an explicit enhanced rules mode.
Keep the native center slice and HUD placement as the compatibility visual check.

Render to an explicit color frame (for example RGBA8), retaining tile color
indices, palette IDs, transparency and priority until composition is complete.
Represent ordinary four-shade palettes through that same interface. DX can then
supply actual CGB colors and art packs can resolve logical IDs independently of
collision. A grayscale framebuffer cannot carry enough information for DX.

Presentation pipeline: resolve art/palettes, compose game layers, apply selected
filter passes, scale/letterbox, then draw readable menus. LCD/CRT-style effects
may need output-resolution passes; define pass order per effect. Palette choice,
sampling filter, and temporal ghosting should be separate options. Temporal
history must reset on level change, load/restore, and relevant setting changes.

Use a fixed simulation accumulator with the repository's current nominal 60 Hz
contract. Display refresh and interpolation are separate. Interpolate previous
and current world positions, with explicit discontinuities for teleports,
respawns and scene changes. Retiming to hardware cadence is a separate parity
decision, not a side effect of adding a new renderer. Audio-device pacing must
not choose simulation state or game update order.

## Menus, bindings and controllers

Initial app screens: ROM library/import, play/resume, settings, and pause overlay.
Settings tabs can cover display, sound, input, and accessibility. Developer tools
open as an optional panel. Leave original title/file-select game flow distinct
from this desktop launcher.

Store versioned settings in a user configuration directory and content in a user
data/cache directory, independent of the checkout or working directory. Use
atomic settings/save replacement. Resolve defaults, persisted preferences and
explicit CLI overrides in a documented order. Separate local presentation/input
preferences from gameplay rules that must agree between peers.

Map keyboard/gamepad events to actions such as Left, Right, Jump, Run, Start,
Select, MenuConfirm, MenuBack and ToggleDebug. Convert gameplay actions to the
existing `PlayerKeys` at one boundary. Record normalized buttons per tick, so
replays survive binding changes. Preserve held/pressed/released distinctions and
queue short taps between simulation ticks. Define opposite-direction behavior.

Support multiple bindings, rebinding cancellation/conflicts, default reset,
stick dead zones with hysteresis, and persistent device preferences. Handle
disconnect/focus loss without stuck buttons. Route events through active UI/input
contexts so typing or binding capture cannot move Mario. Add explicit controller
focus/navigation for menus; using egui does not provide that complete flow.
Reserve a `PlayerId` in input assignment now without rewriting gameplay yet.

## Debug tools

First increment: pause, exact one-tick step, speed multiplier, collision probes,
camera/native bounds, object slots/typed fields, scene state and RNG/clock input.
Second increment: level/scene jump, teleport, powerup, invulnerability, spawn,
snapshot save/load and deterministic input recording.

Inspect immutable views; apply mutations through validated `DebugCommand`s at
tick boundaries. Keep overlays and free-camera viewing independent of the game
camera. Snapshot restore must restore input-edge latches as well as visible state.
Separate pausing the desktop session from pressing the original game's Start.
Later network modes must define debug authority and synchronized pause/commands.
Do not allow local unsynchronized mutations of a shared world.

## ROM import and DX compatibility

Implement an importer library callable by CLI and GUI. Keep the development
extractor as an oracle while migrating each source-dependent lookup to audited
ROM pointers/tables or verified engine metadata. No decompilation checkout should
be required on an end user's machine.

Import flow: select/drop ROM, validate size and bounds, identify a supported
revision by exact digest, extract to a temporary directory, validate the complete
bundle, then atomically publish its cache entry. Cache identity includes source
digest, importer version and content schema. A failed or cancelled import must
not disturb a working installation. Report unsupported revisions explicitly.
Header flags and file size are hints, not proof of DX identity.

Preserve the existing bring-your-own-ROM policy in DESIGN.md. Extract assets
locally from supplied base/DX ROMs. Supporting arbitrary hacks is a different
scope: native Rust does not execute their patched Game Boy code.

DX should ship in explicit stages:

1. Audit one selected revision: tables, changed graphics, attributes, scenes and
   mechanics. Do not assume the historic version/offset examples in DESIGN.md
   are verified support data.
2. Extract banked tile data, BG/window attributes, BG/OBJ palettes, sprite bank
   and palette selection, flips and priority. Apply them during composition.
3. Extract or verify character graphics/animation and movement profiles. Move
   audited character-specific constants behind `CharacterProfile`; represent
   behavior branches explicitly where a numeric profile is insufficient.
4. Verify the revision's actual gameplay changes through normal game flow before
   calling it full DX compatibility. Color support alone is a narrower feature.

Keep `ContentId`, `RulesetId` and character choice explicit. Namespace saves by
compatible content/rules, with deliberate conversion if progress transfer is
supported. Art-only changes may share simulation identity; changes to physics,
collision, spawns or scripts must affect it.

## Multiplayer and online play

Distinguish shared-control/alternating turns, ghosts/races with separate worlds,
and simultaneous co-op. The first two need much less gameplay change. Plan the
co-op foundation for two players sharing a camera; split-screen and independent
level transitions are later extensions.

Before implementing co-op, define camera tether/catch-up, enemy targeting,
simultaneous pickup/stomp arbitration, pipe and boss entry, death/revival,
lives/progression ownership, and pause. These are new game rules with no original
single-player reference. Keep the original rules as a verified configuration
while sharing native movement and actor behavior across configurations.

Extract per-player movement, powerup, collision/contact and input-edge state from
shared terrain, objects, timer, scene VM, progression and camera. Introduce stable
player/entity IDs. Update the world once per tick, resolve interactions in a
stable order, and supply explicit player/target context to actors. Running the
existing whole `PlayerSim` twice would duplicate shared effects and retain hidden
single-player assumptions. Preserve ROM slot limits/conversion in compatibility
mode; enhanced entity capacity can be a later policy.

For online play, first prove identical initial state plus tick inputs produces
identical state across render rates and supported platforms. Live mode currently
samples `NativeDividerClock` from wall time; headless uses `VirtualDividerClock`.
Expose this as an explicit clock policy: record externally sampled DIV values for
fidelity runs, or use a seeded tick-derived clock for synchronized native games.
Do not silently change the current replay/native clock contract.

Snapshots must include players, mutable terrain, spawn cursors, entities and
allocation state, camera/scene/map transitions, timers, RNG/divider phase and
previous input latches. Immutable content is referenced by identity. Include
sound-driver state where it feeds gameplay: `current_song` already affects the
level timer. Keep `Instant`, windows, device handles and save paths outside.

Prototype local snapshot/restore and forced rollback before transport. Then try
LAN/direct-connect input synchronization. For responsive action, evaluate rollback
using a library such as [GGRS](https://github.com/gschup/ggrs), which requests game
state save/load/advance operations. A simpler delayed lockstep prototype can prove
determinism but adds input latency. Neither approach removes the co-op work.

Use a protocol handshake covering engine/protocol version, gameplay-content
digest, rules, tick rate and seed. Send tick-indexed inputs with recovery for
loss, check state hashes, and test latency/jitter/disconnects. Define confirmed
effects so rollback cannot duplicate saves, achievements or sound playback;
simulation sound sequencing still restores deterministically. Public internet
play adds NAT traversal/relay, lobby discovery, host departure and service
operations. Those are later deliverables beyond a networking library.

## Level editor

Use the game's content provider and renderer for editing and playtest. Define a
versioned `LevelProject` with base-content identity, terrain/metatiles, object
placements, player entry, camera/scene constraints, exits and required assets.
Keep authoring data separate from the mutable playtest world. Use stable logical
behavior IDs with a registry mapping original spawn/handler values at import.

First release stays within existing 256x48 cells and supported metatiles/actors.
Provide paint/fill/select, spawn placement, collision overlay, undo/redo,
save/load and restartable playtest. Compile projects into validated normal game
content, including spawn ordering and scene/camera dependencies. Report original
limits clearly; existing collision is tied to metatile IDs, so changing art alone
does not define a new collision type.

Store edits as native projects or overlays referencing the user's imported
content. Original-ROM changes, new mechanics, arbitrary level dimensions and ROM
patch export each need separate scope. Begin with native runtime loading; ROM
export also needs compression, allocation/bank limits and pointer rewriting.

## Delivery order and acceptance

Preserve the original-game completion roadmap. The following packages order the
enhancement work; they do not authorize one large refactor or mark port items done.
Small diagnostics needed to finish parity can be pulled forward.

| Package | Focused steps | Exit evidence |
|---|---|---|
| A. Boundaries | Expose runtime library; extract render/presentation; inject content provider; move storage/device effects out of stepping | Existing CLI/headless behavior preserved; core/runtime tests and frozen sweep unchanged |
| B. Desktop shell | Present CPU frame through selected backend; settings; action mapping; controllers; zoom/fullscreen; debug inspection/step | Resize/DPI/focus/hotplug tests on target desktops; normalized input equivalence; unchanged game frames |
| C. Standalone import | ROM-only base importer library; atomic cache; library/import menu; save namespaces | Clean user-data directory can import and play without checkout/decomp tree; byte agreement with development extraction |
| D. Presentation | Color frame/resolver; palettes/filters; optional viewport and interpolation controls | Native unfiltered visual parity and center-slice checks; no simulation changes from display settings |
| E. Content tools | One DX revision audit/graphics/rules; independently, native editor MVP | Revision-specific DX behavior/color checks; project round-trip and isolated playtest |
| F. Multiple players | Rules specification; per-player state extraction; local co-op; deterministic snapshots | One-player parity; deterministic two-player interaction cases; restore/resimulate yields matching state |
| G. Online | Local forced rollback; LAN/direct-connect; then internet services | Cross-platform state hashes; delayed/lost/reordered packet scenarios; no duplicated effects on rollback |

Editor and DX work can proceed independently after their content/render
dependencies exist. Ghosts/races can precede co-op. Do not implement online
transport before there is a multiplayer mode worth synchronizing.

Behavior changes retain PORTING.md tests and frozen-corpus rules: meaningful
in-memory producer/consumer assertions, core tests, and one frozen sweep per work
package with pipefail. Add runtime tests for new boundaries; use direct synthetic
inputs and existing fixtures, without adding frozen captures. Visual checks are
separate because the replay sweep does not validate pixels.

This planning-only change requires documentation/diff validation; it makes no
new simulation, visual-parity, DX-support or networking verification claim.
