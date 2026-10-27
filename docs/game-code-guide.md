# Finding and changing game code

The host supplies held buttons and DIV through `FrameInput`; `Session::advance`
updates simulation and returns `FrameEffects`; the host persists saves and
renders read-only state. See [DESIGN.md](../DESIGN.md#runtime-boundaries) for
ownership, progression and display contracts, and [PORTING.md](../PORTING.md)
for change/verification rules.

## Source map

Paths below are relative to `crates/`.

| Change | Start here |
|---|---|
| Walking, jumps, water, moon physics | `sml2-core/src/player_sim.rs`, `player/motion_tables.rs`, `player/tests/` |
| Pipes | `PlayerSim::pipe_entry`, `pipe_travel`; runtime session/scene tests |
| Enemy/platform/item/projectile | Named behavior under `sml2-core/src/objects/`, matching `compat/` adapter, category tests |
| Bosses | `sml2-core/src/objects/bosses/<family>/` |
| Tile solidity / layout | `sml2-core/src/collision.rs`, `level.rs`; runtime `content.rs` |
| Map navigation / minigames | `sml2-core/src/map_sim.rs`, `map/`; runtime `session/map_transition.rs` |
| Title / file select | `sml2-core/src/front_end/` |
| Tick order / screen transitions | `sml2-runtime/src/session.rs`, `session/level.rs`, `game/level_frame/` |
| Tiles, sprites, palettes, viewport | `sml2-runtime/src/render/`; core `player.rs` draws sprites, `player_sim.rs` moves them |
| Music / SFX | `sml2-core/src/audio/driver.rs`; runtime host/audio output |
| Desktop controls/settings/import | `sml2-desktop/src/`; [desktop guide](desktop.md) |
| Scenery | `sml2-core/src/scenery.rs`, runtime render/content, `sml2-scenery-editor` |

## Trace one behavior

`objects/dispatch.rs` selects a ROM handler. Its compatibility adapter decodes
the slot through `Actor`, supplies world operations, invokes the named behavior
and writes back converted state. For example, follow handler `0x58848` through
`compat/enemies/hopping.rs`, `enemies/hopping.rs` and their tests.

Game rules use named fields, states and shared movement helpers. Raw slot bytes
belong at the conversion boundary. Preserve unknown values/reserved bytes,
wrapping u16 world coordinates and independently wrapping u8 screen projection.
A behavior animation ID, draw frame and encoded speed-table index are different
things; do not substitute pixels-per-frame for the source index.

Before moving code, trace all callers and the shared frame order. Test existing
behavior before refactoring; a ROM discrepancy gets a separate correction.
Keep public paths with re-exports during moves. Add typed tuning only for a
concrete mod, preserving original defaults; avoid generic hooks or a plugin ABI
without a demonstrated consumer. Content injection via `insert_level` does not
implement a public mod format.
