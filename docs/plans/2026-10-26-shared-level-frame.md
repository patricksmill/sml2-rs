# Share gameplay frame orchestration

Implemented and verified, 2026-10-26. Scope: finding 1 from the maintainability audit.

## Outcome

A change to the shared player, camera, or object update sequence has one
implementation to edit. Interactive/headless diagnostic play and live sessions
use one live level-frame driver. Replay uses those same gameplay phases through
a small adapter that preserves its historical input and observation contract.

This is a behavior-preserving runtime refactor. Platform handlers, terrain
adapters, actor types, save encoding, asset extraction, and progression ownership
are outside this work package. Preserve public session APIs, CLI options,
`PlayOutcome`, tape termination, and existing route timing assertions.

## Source and callers

All paths below are relative to `crates/sml2-runtime/` unless otherwise stated.

| Entry | Current responsibility | Callers/consumers to retain |
|---|---|---|
| `src/session/level.rs::LevelContext::step_frame` | Live level update, scene effects, sprite preparation, pause tail, counters | `Session::advance` in `src/session.rs`, including the attract demo; desktop and runtime hosts consume the session |
| `src/play.rs::run_play` | A second live level update plus window/tape handling, diagnostics and rendering | `src/cli.rs`; `src/tests/played_routes/mod.rs::RouteAssets::play` |
| `src/replay.rs::run_replay` | Capture-seeded update, intermediate observations, comparisons, death gaps, optional filmstrip | Replay CLI and the frozen gate |
| `src/game.rs` | Existing shared contact construction, frame head, scene transfer, input sampling and timer helpers | All three paths and focused integration tests |

Confirm the caller inventory again before implementation. Existing local edits
to `README.md` belong to the user and must stay outside these commits.

## Differences that must be pinned before moving code

These are observed implementation differences, not claims that either side is
correct. Record their current outputs first. A source discrepancy gets a
separate correction and source-derived test, rather than being silently fixed
inside an extraction.

| Boundary | Live session / diagnostic play | Replay | Refactoring rule |
|---|---|---|---|
| Clock and input preparation | Advance phase/counter; mask joypad during fade; accept START/SELECT edges | Correct/seed phase from captured samples; no recorded START/SELECT; captured input goes directly to frame head | Hosts prepare inputs and timing. Preserve each preparation contract. |
| Scene-owned frames | Shared `scene_frame_head` suspends gameplay and ticks fade | Same existing helper | Reuse it; never tick the timer a second time in a new driver. |
| Player update | Crouch pan precedes physics; transitions substitute a fade tick | Same basic body, with optional captured camera-edge override | One shared player/camera phase; keep capture injection in replay. |
| Player/object events | Reset stomp chain, apply player bump, apply fireball bump, resolve fire scans, then advance camera | Same sequence | First extraction; preserve old-scroll coordinates and one-shot consumption. |
| Object update | Snapshot player contact; sample inputs/DIV; handle boss load; step objects; apply results | Same sequence | Share the common sequence and effects; explicitly retain timing differences below. |
| Boss-load timing | Subtract `BOSS_LOAD_VBLANKS` after a successful load; session also remembers the boss definition | No matching subtraction in current replay | Characterize independently. Keep compensation and session bookkeeping at the adapter boundary until their contracts are established. |
| Pose / block update | `frame_index` and `block_update` run after objects and scene processing; frozen frames preserve pending SFX | Pose and block check are sampled before objects; block update has a different freeze gate | These calls mutate state. Preserve placement and SFX, not just the returned pose. Do not treat them as read-only rendering. |
| Death and exit | Session advances death fade/map return; diagnostic play stops/records according to its existing contract; exit guards differ | Capture gap handling fast-forwards death fade/reload; exit-tail checks also run outside the frame loop | Keep replay gap accounting and final assertions outside shared gameplay. Pin native differences before sharing dispatch. |
| Scene result | Both native paths apply DMA, tileset/metatile/room reloads; session additionally tracks arena blanking and DX state | Applies scene DMA/pipe presentation, with a different surrounding content lifecycle | Share matching scene operations only; preserve room and layout-write effects and their order. |
| Host tail | Pause edges, live sprite/palette data, animation and telemetry; session later drains sound requests and changes screens | Assertions, capture summaries and filmstrip | Shared native mutations happen before host effects. Replay reporting remains outside the driver. |

## Intended shape

Extend the existing private `game` module with `src/game/level_frame.rs` and
small named functions for the established gameplay phases. Reuse the existing
`scene_frame_head`, `build_contact`, `sample_level_player_inputs`, and
`apply_*_result` helpers.

After those phases are verified, extract one `step_live_level_frame` driver
from the matching parts of `LevelContext::step_frame` and `run_play`. Keep the
current state owners. If grouping references is necessary, use one private
borrowed view of the existing state; do not introduce a second owned level
model or reconstruct a level through `from_content` every frame. In particular,
diagnostic callers supply customized `PlayerSim`, `CameraSim`, and `ObjectSim`
values that must not be reseeded.

Replay calls the same shared gameplay phases around its named observation
points. Its adapter owns phase/edge injection, capture gaps, and assertions.
It may retain an outer sequence where the table above proves different
observation timing is required. It must not retain a copied player-event body,
object-result writeback list, or independent implementation of a common phase.

Do not introduce an engine trait hierarchy, event bus, arbitrary callbacks, or
a collection of `is_replay`/`skip_*` switches. Return only observations that
existing callers consume: for example, the player-step float-budget sample,
successful boss definition, scene result, and entry-state freeze/death flags.
Snapshots must be taken at their original phase, not reconstructed from final
state after a camera or object update.

## Bounded implementation steps

Each step is a focused commit after its checks pass. Keep behavior corrections
separate from mechanical consolidation.

1. **Pin the contracts and establish the baseline.** Allowed files:
   `src/tests/game.rs`, a new `src/tests/level_frame.rs`,
   `src/tests/mod.rs`, and the relevant existing session/route test modules.
   Construct small in-memory cases for old-scroll block bumps, fire hits,
   trigger-frame versus next-frame freeze, transition fade ticks, boss-load
   timing, pose/block/SFX order, and death/exit handoffs. Reuse existing pause,
   pipe, boss, ending and played-route fixtures. Pin each entry's actual
   externally meaningful bytes before extraction; equality between two paths
   alone is insufficient. Record baseline failures/ignored tests without
   weakening existing assertions. Use targeted source slices if deciding
   whether a difference is intentional; do not invent a new compatibility flag
   merely to avoid investigating it.

2. **Share the player-event and camera phase.** Allowed files:
   `src/game.rs`, new `src/game/level_frame.rs`, `src/play.rs`,
   `src/replay.rs`, `src/session/level.rs`, and `src/tests/level_frame.rs`.
   First move stomp reset, bumps, and fire scans verbatim into one function and
   switch all three callers. Then consolidate the matching crouch-pan,
   physics/transition and camera body around it. Preserve the dead-arc and
   freeze contracts; feed captured edge input from replay. Return the precise
   player-step observations needed by timed-float telemetry. Acceptance:
   all three call the shared phase, and tests fail if event resolution moves
   after the camera or one-shot effects are applied twice.

3. **Share object preparation, stepping and writeback.** Same file scope as
   step 2, plus `src/tests/boss_bounce.rs` and only the boss-entry test files
   needed for the selected cases. Consolidate contact construction, sampled
   player/camera inputs, DIV, object stepping, and both result applications.
   Replay currently builds contact before its pose/block operations, then
   steps objects afterward. Keep that contact snapshot across the checkpoint;
   building it inside a later object-step helper would change the inputs.
   Snapshot contact at its existing point relative to boss loading. Extract
   the common boss-load operation once; apply any proven adapter-specific
   compensation before the object pass, at the same point as today. Preserve
   active-object telemetry and blocked-handler reporting. Acceptance: one
   implementation of the common object pass; seeded slot bytes, contact
   results, boss clocks and sound requests stay unchanged.

4. **Collapse the two native frame drivers.** Allowed files: step 2's runtime
   files, `src/session.rs` for import wiring only if needed, and the existing
   pause/audio, scene/pipe, ending and played-route tests. Move matching native
   frame-head dispatch, shared phases, scene effects, pose/block operations,
   pause tail and animation into `step_live_level_frame`. Preserve the
   characterized diagnostic/session differences at explicit boundaries;
   keep session-only death progression and presentation bookkeeping in their
   owning wrapper where necessary. `run_play` keeps its input/window/tape loop,
   rendering, printed diagnostics and outcome aggregation. `LevelContext`
   retains ownership and its public method. Do not move session sound draining,
   save emission, overworld transitions or the sound driver into this helper.
   Acceptance: the native hosts no longer independently implement the matching
   gameplay dispatch; route frame numbers, saves, pause/death audio requests
   and scene layout writes remain pinned.

5. **Finish the replay adapter and audit remaining duplication.** Allowed
   files: `src/replay.rs`, `src/game.rs`, `src/game/level_frame.rs`, and focused
   frame tests. Remove leftover copies of the phases migrated in steps 2/3.
   Keep capture comparisons at their existing checkpoints and death-gap/
   exit-tail accounting outside normal ticking. Add a short source comment
   explaining each remaining ordering difference. Reuse the native driver
   only for cases demonstrated equivalent; otherwise use its shared phases.
   Acceptance: every remaining replay-only mutation has a capture-contract
   reason and a test; a common gameplay fix reaches all three paths by editing
   one phase. Keeping three complete loops under a new filename does not pass.

If an uncovered behavior difference prevents a particular extraction, document
the exact conflicting outputs and the bounded prerequisite correction. Other
independent phases can proceed, but the package is not complete while an
unexplained duplicate gameplay body remains.

## Verification and completion

For each implementation step run its focused tests, then:

```bash
cargo test -p sml2-core
cargo test -p sml2-runtime
```

The runtime suite includes session-boundary integration tests and played routes
through the actual `run_play` caller. Explicitly check that the intended tests
ran; list ignored or asset-dependent cases separately. Existing manually
assembled boss harnesses are useful branch tests, but do not replace exercising
the production driver. New ordering tests use in-memory state and byte/SFX/
slot assertions; deliberately reorder a relevant phase once to establish test
sensitivity, then restore it. Do not add a test-only copy of the full old loop.

After unit/integration tests pass, run the frozen sweep once at the work-package
boundary:

```bash
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

The checked-in gate accepts only its exact existing `traj4_ff0` exception.
Do not edit captures, expected mismatch counts, or notes to make the refactor
pass. This gate supplements the unit tests; it does not prove pixel/audio
parity. If sprite selection, palettes or audio outputs change unexpectedly,
investigate before accepting the structural move.

Completion requires one native driver, one implementation of each common
gameplay phase used by all three paths, explicit tested replay boundaries,
unchanged public callers and route outcomes, passing applicable checks, and
focused commits containing only this package's files. No LOC target or
full-game completion claim is attached to this work.

## Implementation and verification

All five steps are complete. The shared phases live in
`crates/sml2-runtime/src/game/level_frame.rs`; the borrowed native state view
and driver live in its `live.rs` submodule. Diagnostic play and live sessions
use that driver. Replay calls the same player, camera, object, ending-sequence
and scene phases around its capture checkpoints.

The player and camera phases remain separate because replay installs the
current captured edge between them: crouch-look must still read the preceding
edge latch. Its pose checkpoint also retains the contact snapshot taken before
pose selection mutates invincibility. Both boundaries have direct tests.
Native frozen departure policy and session presentation latches remain in
their existing hosts; public callers and state ownership are unchanged.

Validation passed:

- `cargo test -p sml2-core`: 1,687 tests.
- `cargo test -p sml2-runtime`: 263 tests, including 11 new frame-boundary tests.
  The existing four route-authoring tools and five audited DX ROM/export cases
  remain ignored; none is counted as passing validation.
- Ordering mutations fail the relevant tests: events after camera movement,
  contact rebuilt after pose selection, and captured edges installed before
  crouch-look. All mutations were restored before the final checks.
- `cargo fmt -p sml2-runtime -- --check` and scoped Git whitespace checks.
- Frozen replay sweep: `gate done`, with the exact existing `traj4_ff0`
  exception and no new notes. No captures or gate expectations changed.

These checks establish the refactor's tested state/timing contracts, not full
pixel or audio parity. The unrelated local `README.md` edit was preserved.

## Planning validation

At planning time this document was checked against runtime source, callers,
test inventory, and `tools/replay_gate.py`; that documentation-only step ran
no implementation tests or replay sweep. The older completion-workflow and port-reset
plan paths linked by `AGENTS.md`/`PORTING.md` are absent in this checkout; the
current porting policy and existing modularity plan remain the available
workflow references.
