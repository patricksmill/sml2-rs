# Native modularity and understandable game rules

Initial source audit, 2026-10-23. Goal: make the Rust code easier to learn and
modify while retaining the current implementation's observable behavior.
The first five slices are implemented; their results are recorded below.
Further architecture proposals do not replace the active original-game completion queue. Follow
[PORTING.md](../../PORTING.md) and the
[completion workflow](2026-10-15-rust-completion-workflow.md) for bounded work.

## What is already native

The workspace separates core simulation, content extraction, runtime hosting,
desktop UI, and DX import. `Session::advance(FrameInput)` and `FrameEffects`
already separate ticking from host I/O. `ContentSource`, preloaded
`SessionAssets`, and `insert_level` provide useful content seams.

Object behavior already uses `Actor`, small world interfaces, shared motion
helpers, and compatibility adapters. Preserve these boundaries. The next work
should make them consistent and easier to find rather than replace them with
a new engine framework. [The code guide](../game-code-guide.md) maps existing
entry points for readers and source modders.

## Findings and implications

File sizes are an audit snapshot including tests and tables, not quality scores.

| Observed source | Difficulty for readers/modders | Proposed bounded improvement |
|---|---|---|
| `player_sim.rs`, 3,156 lines | Tables, collision probes, projectiles, pipes, progression, saves, and the player tick share one module | Extract one cohesive responsibility at a time, keeping `PlayerSim` and its public API stable initially |
| `map_sim.rs`, 3,951 lines, with a substantial inline test section | Navigation, decorations, room logic, and compatibility state require substantial searching | Separate tests first where useful, then one map responsibility per ticket; preserve hub-versus-zone ordering |
| Runtime `session.rs`, 2,132 lines | Level context, loading, progression, and screen dispatch share an entry point | Move level context into `session/level.rs` behind a re-export before changing state ownership |
| `objects/mod.rs`, 1,510 lines | Descriptive paths often register as `handler_0x...`; discovery requires address knowledge | Give one domain module a descriptive Rust name and update its adapter references; keep ROM dispatch identities |
| `Actor.state` remains a `u8`; some behaviors match numeric states directly | A reader needs source knowledge to interpret each branch | Introduce local behavior-specific state names in one handler while preserving all raw-value semantics |
| `PlayerSim` and `MapSim` carry progression and source-format state | Adding a movement mod can require understanding unrelated timers and saves | Document ownership and writers/readers before moving a single group of fields |
| Content supports injection, but behavior configuration is distributed across code and source tables | Replacing a level is easier than expressing a reusable gameplay mod | Prove one typed configuration seam with unchanged defaults before defining a general mod format |

This is a structural audit of selected entry points and examples, not a
behavior-completeness audit of every handler. Existing plans contain historical
snapshots; verify claims against current source before scheduling work.

## Preserve behavior explicitly

The first refactoring target is equivalence to the current code. ROM parity is
a separate claim: a discrepancy found during refactoring gets its own fix and
source-derived evidence. Avoid combining a correction with a structural move.

Keep exact lookup tables, wrapping widths, signed comparisons, update ordering,
slot allocation order, timers, unknown values, reserved bytes, and effects.
Preserve source IDs at import/dispatch/export boundaries. A named state must
retain the current behavior for unrecognized bytes; do not silently map them
to a convenient default or start rejecting formerly accepted inputs.

Keep public import paths through re-exports during internal moves. A pure
extraction should not broaden visibility beyond what callers need. Avoid
changing representations and ownership in the same ticket as a file move.
Rendering remains a reader of simulation state; clock and sound-driver state
that affects gameplay remains under the existing tick contract.

## First work packages

Each numbered item defines one bounded ticket. Its exact file list and existing
test inventory are checked before implementation. No pending queue item is
closed by this plan.

1. **Name one native enemy module.** Scope: the `hopping` module registration,
   its adapter imports/call, and any references found by a bounded search.
   Change `handler_0x58848` as the domain module name to `hopping`, retaining
   the address-based engine method, dispatch arm, source comments, and test
   identifiers. Acceptance: unchanged behavior bodies, existing hopping tests,
   core suite, frozen sweep. Review tools that recognize handler names before
   accepting the rename. This improves discoverability with little semantic risk.

2. **Extract player motion tables.** Scope: only the exact speed-table constants
   and their import/re-export wiring in `player_sim.rs` and a new private
   `player/motion_tables.rs`. Inventory all affected constants first. Preserve
   every byte and existing `player_sim::SPEED_*` public path. Acceptance: compare
   the moved arrays against the pre-change source, run existing player tests,
   core suite, and frozen sweep. Keep movement calculations and tick order intact.

3. **Extract the level context.** Scope: `LevelContext` and its implementation
   from runtime `session.rs` into `session/level.rs`, with the old path re-exported.
   Review private-field access from session siblings and renderer before moving.
   Acceptance: unchanged method bodies; core and runtime tests, including public
   session-boundary tests; frozen sweep. Preserve the exact `step_frame` order.

4. **Name hopping states.** After ticket 1, scope only its domain handler and
   category tests. First pin the current handling of initialization, each state,
   invalid bytes, turning cadence, and wrapping. Then introduce local state names
   and named constants whose meanings are established. Acceptance: native branch
   tests plus existing adapter byte assertions and full gates. No generic global
   state enum: different enemies reuse the same byte for different rules.

5. **Map progression ownership.** Produce a small ownership table for level entry,
   clear, death, map return, and save: authoritative writer, consumers, reset
   point, and carry-over behavior. Use it to select one later field-group move.
   Acceptance is a reviewed source map with actual call sites; no state move is
   authorized merely by grouping similarly named fields.

## A practical modding boundary

Start with source mods and content injection; they already have working entry
points. A later mod API should distinguish level content, presentation assets,
and gameplay rules so an art override cannot accidentally change collision.

For the first configurable gameplay rule, choose one well-tested native behavior
and introduce a small typed definition. Preserve original constants as the default
and demonstrate a deliberately different configuration in an isolated test. Keep
the original behavior's serialized bytes and frame cadence identical under defaults.
Only after a second consumer demonstrates the shared need should the API expand.

Stable content IDs, validated overlays, override precedence, compatibility/version
rules, and user-facing load errors need a separate design before externally loaded
mods. `insert_level` is a useful seam but does not implement those contracts.
Avoid committing to a plugin ABI, scripting runtime, ECS conversion, or arbitrary
global hooks before a concrete mod requires them.

## Verification and progress

At this audit, `cargo test -p sml2-core` passed all 1,655 tests, and
`bash captures/sweep.sh` passed the frozen corpus with the established
`traj4_ff0` exception. These runs include the working tree's existing rendering
edits. They establish a starting point for future refactors, not full-game parity.
The initial audit changed documentation only.

### Implemented vertical slices

The five initial tickets above are complete. The configuration/mod-format
discussion remains future design work.

| Slice | Result | Verification |
|---|---|---|
| 1 | Native module is `hopping`; address-based adapter discovery remains supported by the prompt generator | Core suite, 46 tooling tests, frozen sweep; commit `6c21b90` |
| 2 | Six exact speed tables live in private `player/motion_tables.rs`, re-exported at their existing paths | Verbatim extraction comparison, core suite, frozen sweep; commit `ee5e433` |
| 3 | `LevelContext` lives in `session/level.rs`, re-exported by `session` | Verbatim type/method comparison, core suite, runtime suite including public boundary tests, frozen sweep; commit `b9d0269` |
| 4 | Named hopping states and timing constants; unknown state handling retained | 1,656 core tests, frozen sweep, all 251 unknown state bytes checked; deliberate unknown-to-init mutation fails the new test; commit `c009912` |
| 5 | [Progression ownership map](../progression-ownership.md) with producer/consumer/reset contracts and a bounded follow-up candidate | Reviewed against transition, conversion, reset and save call sites; documentation only |

Runtime verification passed 198 unit tests (4 ignored) and 11 integration tests
(4 ignored). Ignored tests were not treated as passing evidence. Each frozen
sweep retained the established `traj4_ff0` exception. No original-game completion
queue item was closed by these refactors.

For behavior changes or new coverage, construct in-memory cases with independently
derived expected bytes and sensitivity checks as required by `PORTING.md`.
For a mechanical move, reuse existing behavioral tests and compare moved content;
add tests only for a real uncovered risk. Run `cargo test -p sml2-core` for each
implementation ticket and relevant runtime checks when that layer changes. Run
the frozen sweep once per work package with failure propagation intact.

Pixel comparisons and audio traces remain separate gates when those outputs
change. A passing core suite or replay does not prove either one.

Track practical outcomes: how many files a one-enemy edit touches, whether its
states and tuning values are discoverable by name, and whether a newcomer can
trace input through update to effect and locate the verifying test. Lines moved
or module counts alone do not establish improved modularity.
