# Understanding and modifying the game

This is a source-code map for learning the game and making Rust changes.
It describes existing entry points; proposed architecture changes live in the
[native modularity plan](plans/2026-10-23-native-modularity.md).

## Start with the game loop

The desktop supplies input to a `Session`. `Session::advance` advances one
simulation tick and returns effects such as save records. The renderer reads
the resulting state. The host handles devices, clocks, and saving to disk.

```text
Desktop or CLI
  -> FrameInput (held buttons and DIV sample)
  -> Session::advance
       -> current screen / map / level
       -> player, objects, camera, scenes, sound driver
  -> FrameEffects -> host persistence and diagnostics
  -> read-only rendering -> display

ContentSource -> SessionAssets -> Session
                decoded before ticking
```

This is an ownership map, not the exact order of every gameplay operation.
Within a tick, order matters: a collision or timer may deliberately read the
previous tick's value. Read `LevelContext::step_frame` and `PlayerSim::step`
before moving an operation between systems. The
[runtime boundaries guide](runtime-architecture.md) explains session loading,
host integration, and the sound driver in more detail.

## Find the rule you want to understand

Paths below are relative to the repository root.

| Question or change | Start here | Also inspect |
|---|---|---|
| Walking, jumping, swimming, moon physics | `crates/sml2-core/src/player_sim.rs`: `PlayerSim::step` | `player/tests/`, `camera.rs` for camera effects |
| Pipes and level travel | `player_sim.rs`: `pipe_entry`, `pipe_travel` | `player/tests/pipe.rs`, runtime `session.rs` |
| One enemy's movement or animation | `crates/sml2-core/src/objects/enemies/<behavior>.rs` | Matching `objects/compat/enemies/` adapter and `objects/dispatch.rs` |
| Platforms, items, projectiles, bosses | Corresponding directory under `crates/sml2-core/src/objects/` | Category tests and shared actor helpers |
| Tile solidity and terrain | `crates/sml2-core/src/collision.rs` | `level.rs`, player probes and object terrain probes |
| Level layout and placed objects | `crates/sml2-core/src/level.rs`: `LevelLayout`, `LevelManifest`, `SpawnRecord` | Runtime `content.rs`, `sml2-extract` producers |
| Map walking, doors, course entry | `crates/sml2-core/src/map_sim.rs`: `MapSim::tick` | `map.rs`, `map/`, runtime `session/map_transition.rs` |
| Title, file select, save slots | `crates/sml2-core/src/front_end/` | Runtime `Session::advance` and save effects |
| Level transitions and overall game flow | `crates/sml2-runtime/src/session.rs` | `session/`, `game.rs`, runtime tests |
| Sprites, tiles, palettes, viewport | `crates/sml2-runtime/src/render.rs` and `render/` | Core `player.rs` is sprite drawing; `player_sim.rs` is gameplay |
| Music and sound effects | `crates/sml2-core/src/audio/driver.rs` | `audio/`, runtime `host.rs` and `audio_output.rs` |
| Controls, settings, mod switches | `crates/sml2-desktop/src/` | [Desktop guide](desktop.md) |

Core paths in the second and third columns use `crates/sml2-core/src/` unless
marked runtime. Runtime paths use `crates/sml2-runtime/src/`.

## Trace one enemy from engine to game rule

Use the hopping enemy as a small example:

1. `objects/dispatch.rs` routes ROM handler `0x58848` to its adapter.
2. `objects/compat/enemies/hopping.rs` converts the staged slot to an `Actor`
   and supplies world queries such as walls, ground, and the player's position.
3. `objects/enemies/hopping.rs::update` implements initialization, hopping,
   and turning using actor fields and world methods.
4. The adapter maps probe effects back to engine state and converts the actor
   to its slot representation.
5. `objects/enemies/tests/hopping_adapter.rs` checks the engine boundary;
   the behavior file also contains native tests.

The filename describes the behavior, but its current Rust module is called
`handler_0x58848` in `objects/mod.rs`. Search both the name and address when
following callers. The address is a source-reference identity, not a suggested
name for new game logic. `objects/enemies/walker.rs` provides another example
with named states and explicit movement outcomes.

## Know which representation you are editing

| Term | Meaning |
|---|---|
| `Actor` | Named native object fields and shared movement/animation operations |
| Slot | The original 32-byte object representation used by engine compatibility and byte comparisons |
| Adapter | Converts state and world operations between the engine and native behavior |
| Animation/definition ID | A source table selector that may determine behavior as well as appearance |
| Sprite/frame ID | A drawing selection; distinct from the behavior selector |
| World position | Actor `Position` uses wrapping `u16` coordinates |
| Screen position | `ScreenPosition` uses independently wrapping `u8` coordinates |
| DIV / frame phase | Timing inputs used by game rules; see [frame clocks](frame-clock.md) and [randomness](randomness.md) |

Preserve unknown values and reserved slot bytes at the conversion boundary.
Ordinary Rust signed velocity and source speed-table indices are also different
representations: changing an index to a floating-point speed changes movement.

## Make a focused change

For a source mod, choose one behavior and read its native code, adapter, and
tests together. Keep gameplay changes in the behavior; use the adapter for
engine state conversion. Follow the [Rust guide](idiomatic-rust-porting.md)
and [porting policy](../PORTING.md) when changing original-game logic.

For level-tool experiments, the existing
`SessionAssets::insert_level(id, LevelContent)` boundary replaces a decoded
level before session construction. `ContentSource` also supports in-memory
content. These are integration points, not a finished editor or a general
installed mod format. Changes to extracted files can be overwritten by extraction.

For a behavior-preserving refactor, keep the existing tests and exact constants,
wrapping, call order, and effects. For an intentional mod, add tests for the
new rule while retaining original defaults. The frozen replay corpus verifies
sampled simulation state; it does not establish pixel or audio parity.
