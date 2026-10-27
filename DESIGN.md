# Architecture and design contracts

Native Rust port of Super Mario Land 2. The source reference is the byte-identical
sibling `../sml2decomp/` and its `baserom.gb` (`SML2_DECOMP_ROOT` overrides it).
Implementation rules are in [PORTING.md](PORTING.md); source navigation is in
[the code guide](docs/game-code-guide.md). Pending work lives in `PORT_QUEUE.json`.

## Layout

| Crate | Owns |
|---|---|
| `sml2-core` | Native player/actor/camera/map/scene/audio rules, data types and CPU display helpers |
| `sml2-extract` | Original-ROM verification, extraction and atomic publication; developer disassembly extraction |
| `sml2-dx` | Exact DX revision recognition, local import/cache and derived presentation tables |
| `sml2-runtime` | Decoded content, sessions, shared gameplay phases, read-only rendering, CLI/play/replay hosts |
| `sml2-desktop` | Window/GPU presentation, settings, controls/controllers, import UI and debug tools |
| `sml2-scenery-editor` | Cosmetic scenery authoring and isolated playtests |

No ROMs or extracted game art ship in the repository. Runtime ticks consume
loaded content, not ROM reads. `cargo run -p sml2-extract` regenerates developer
assets under `extracted/`; the desktop also supports direct original-ROM import.

## Runtime boundaries

`ContentSource -> SessionAssets -> Session` decodes content before ticking.
All 32 original levels are loaded eagerly; unusable levels retain a cached error
reported when requested. `SessionAssets::insert_level` supports injected content.

```rust
let assets = sml2_runtime::content::SessionAssets::load_from(&source)?;
let mut session = sml2_runtime::Session::new(assets);
let effects = session.advance(sml2_runtime::FrameInput {
    keys: Default::default(),
    div: 0x5a,
});
let pixels = session.render_frame((160, 144));
// The host persists effects.saves exactly as emitted.
```

`Session::advance` advances one tick and returns effects. It does not read files,
sample clocks, write saves, open devices or generate audio samples. Input edges
stay in the session; hosts submit held buttons and explicit DIV samples. The
sound driver stays in simulation because its acknowledged song affects timers;
the host ingests it into the APU after every tick, even without rendering.

`SessionRunner` owns CLI devices, pacing, audio output and persistence. Live DIV
uses its native clock; headless DIV uses a virtual clock. Desktop hosts the same
session directly. Save effects contain 0x50-byte progress records captured at
the transition, not full simulation snapshots or records regenerated later.

Live sessions and diagnostic play use `game/level_frame.rs` and its `live.rs`
driver. Frozen replay uses those shared phases around its observation points:
it installs the captured camera edge between player and camera phases, while
crouch-look reads the preceding edge; its contact snapshot precedes mutating pose
selection. Keep these adapter contracts explicit rather than copying game loops.

## Progression ownership

| Boundary | Owner / contract |
|---|---|
| Map -> level | `MapSim::enter_check` emits entry; `Session::advance` stages `LevelContext::from_content`, then seeds carried state when it becomes live |
| Live clear / handoff | `PlayerSim::exit_fade_step`, object results and `map_handoff` produce progress; level context retains the first handoff; `map_handoff_done` prevents duplicates |
| Goal bell | Early return differs from ordinary clear/save; do not merge the branches |
| Ordinary death | Return through `MapSim::from_player`, preserve progress/coins, reset session powerup; do not copy clear's broader stats unconditionally |
| Clear | Carry lives, both coin bytes, tally and powerup; boss reveal retains castle progress and the node observed at save time |
| Game over | Separate tally penalty, money-bag/powerup reset and save emission; retain the game-over player image |
| Local respawn | Reset movement/death/exit/pipe/projectile state; preserve source-retained fields; this is not the full map-return protocol |
| Save/load | Persist emitted bytes; loading validates and restores progress/stats but zeros unsaved flutter timer/latch/charge |

The session's flutter copy survives ordinary clear/death, map travel and castle
entry; save loading clears it. Money bag has an object-engine producer. Session
`MapRam` also carries level/metatile writes into later map behavior. `MapSim`
conversion helpers copy selected fields, not complete snapshots. Tests live in
runtime `tests/session_boundary.rs`, `src/tests/session.rs`, `castle_flow.rs` and
core `player/tests/simulation.rs`.

## Standing goals

### Texture packs

Art resolves through logical tile/sprite/metatile IDs. Collision and behavior
follow IDs, not pixels. Keep palettes and 2bpp art separate; extractor PNGs are
reference renders, not the runtime format. Preserve a resolver seam for ordered
ROM-default/pack overrides and higher-resolution art without changing IDs.
An externally loaded pack/mod format remains future work; do not imply the
source/content injection APIs already provide one.

### Simulation and presentation

Simulation stays at fixed 60 Hz; display refresh is independent. Interpolation
reads previous/current positions and never changes collisions, spawns, timers,
animation frames or tile writes. Rendering is read-only at every frequency.

Presentation latency is not a parity target: the console shows previous VBlank
shadows, while native rendering shows the producing tick. Pixel comparisons
normally pair ROM frame N with native N-1. Console CPU lag and skipped load time
are not simulated; see [frame clocks](docs/frame-clock.md) and
[accepted visual differences](docs/visual-parity.md).

### Viewports and composition

160x144 is the parity anchor. Logical viewport, output scale and future texture
resolution are separate settings. Preserve source scroll/window addressing,
OAM order, raw color-index transparency/priority and the leading camera; do not
replace it with player-centered following or separately clamp terrain/sprites.

In enlarged levels, the native rect stays horizontally centered with three
quarters of added height above and one quarter below; the classic HUD sits
bottom-center and the Modern style floats in the view's top corners.
Fixed screens retain their authored frame. Test native pixels within enlarged
views (`sml2-runtime/src/tests/widescreen.rs`). Cosmetic scenery must not modify
collision, actors, spawns or progression. Default output uses nearest sampling
and integer scaling; optional fractional scaling remains a presentation choice.

### DX compatibility

Bring your own exact supported ROM; never redistribute patched code or graphics.
Extract validated data into a local cache; implement behavior in native Rust.
Palettes/attributes feed rendering; DX layouts, spawn positions, object patches
and Luigi's movement feed simulation through a typed mode selected at safe
level-entry boundaries. Current
support and source findings are in the [DX audit](docs/sml2dx-audit.md), with
remaining work in the [DX plan](docs/plans/2026-10-26-dx-completion.md).

## Verification

Follow PORTING.md for source-derived byte tests and package regression gates.
A dispatch/closure count proves reachability, a smoke run proves execution, and
a played route proves its inputs/outcomes. Pixel/audio claims need their own
comparisons. Current route evidence and gaps are in
[the route matrix](docs/route-matrix.md); tool contracts are in
[tools/README.md](tools/README.md). Keep historical debugging journals in Git
history rather than appending them to design or queue files.
