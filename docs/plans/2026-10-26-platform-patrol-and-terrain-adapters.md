# Share platform patrol movement and enemy terrain adapters

Implemented and verified, 2026-10-26, in two focused steps. The platform
extraction preceded the terrain adapter extraction. The design and acceptance
contracts below describe the completed work.

## 1. Shared horizontal platform patrol

Keep the native entry functions in
`crates/sml2-core/src/objects/platforms/horizontal_patrol.rs` and
`long_horizontal.rs`. Extract their identical states 1..=4 and private `step`
functions into one sibling `platforms/patrol_motion.rs`, registered explicitly
in `objects/mod.rs`.

The source briefs `tasks/handlers/0x58C41.md` and `0x58C52.md` both show the
nonzero-state branch entering ROM `$58C7B`, with the same four movement arms.
The shared function takes `&mut Actor`, reads the stored span, updates the
actor, and returns the optional rider displacement or invalid-state error.
Keep each entry's existing effect interface; share or map the movement error
without changing the adapters' entry-specific diagnostics.

Preserve these contracts:

- Short legs: state 1 moves left and ends in 4; state 2 moves right and ends
  in 3. Long legs: 3 moves left and 4 moves right, reversing with `state ^= 7`.
- Move first, derive rider displacement from `stood_on`, then increment the
  wrapping phase. Reset phase and reverse only on equality with the cap.
  Double the stored span with byte wrapping for long legs.
- Horizontal patrol retains its four-frame sprite toggle before dispatch,
  including initialization and invalid-state calls; initialization sets
  contact 32, flash 16, facing, and span 36.
- Long horizontal retains its own initialization with span 60 and no flash
  assignment or sprite toggle. Neither initializer gains a phase reset.
- Slot conversion and `carry_mario_dx` stay in the existing compatibility
  entry functions. Unknown states still report the dispatching ROM address.

Retain both existing platform test suites. Add only missing boundary coverage:
16-bit X wrapping in both directions, phase/span wrapping (including a zero
cap), rider displacement on long-leg reversal frames, and animation ordering
on movement/invalid-state calls. Assert all 32 slot bytes and relevant rider,
facing, scratch, and diagnostic outputs. Expected values come from the briefs,
not merely from comparing two calls to the new shared helper.

Allowed production files: the two native platform entries, the new movement
module, and registration lines in `objects/mod.rs`. Change compatibility
platform entries only if a shared type requires an import adjustment. Tests
stay under `platforms/tests/`.

## 2. Shared terrain adapter and narrower patrol contract

The three named enemy adapters duplicate the same position staging, terrain
calls, and flag masks. A fourth copy exists inside
`objects/compat/enemies/dispatch.rs::fallwalker_dispatch`; include that caller
in this refactor.

### Domain contract

In `objects/enemies/fallwalker.rs`, introduce `PatrolWorld` with only
`probe_left_wall`, `probe_right_wall`, and `probe_ground`. Make
`FallWalkerWorld: PatrolWorld` add `step_fall`, and leave the full fallwalker
update bound to `FallWalkerWorld`. Narrow `patrol` to `PatrolWorld`.

Update projectile walker's update bound, striking patroller's `World`
supertrait, and landing walker's `World` supertrait to `PatrolWorld`. Landing
walker already advances its fall profile directly and needs no `step_fall`
method. Split the existing fallwalker test mock implementations accordingly;
add a patrol test world implementing only `PatrolWorld` so compilation checks
that patrol does not require falling.

### Compatibility adapter

Add `objects/compat/terrain.rs`, registered in `objects/mod.rs`, containing
one borrowed `TerrainWorld` over `ObjectSim`, `Staging`, and `LevelLayout`.
Implement the three patrol probes once:

1. Stage the actor's current X/Y immediately before each probe.
2. Call the existing `m_left_wall`, `m_right_wall`, or `m_ground_own` helper.
3. Decode the existing masks `0x02`, `0x01`, and `0x08` respectively.

Implement the real fallwalker fall operation using the existing
`falling_dropper::step_fall_speed`. Use this world directly in the projectile
adapter and generic fallwalker dispatch. Implement landing walker's facing
accessors and striking patroller's distance accessor for this shared type in
their existing adapter modules. Delete the three local `Inputs` probe copies,
the nested `FallWalkerSimWorld`, and both dummy fall implementations.

Preserve initialization, animation, landing profiles, spawn effects, timers,
facing updates, distance calculations, error attribution, and final actor slot
writeback in the owning entries. In particular, the even-frame patrol probes
see the position after its one-pixel movement. Keep raw staging and flag
decoding inside the compatibility layer.

### Observable probe state

Use the current legacy probe helpers for this extraction. Do not replace them
with `objects/terrain_probes.rs::TerrainProbes` in this work step: the current
implementations differ on misaligned ground probes. `m_ground_own` assigns
the full actor Y to `st.py` before returning, whereas `TerrainProbes::grounded`
retains the previous high Y byte. Preserve the existing caller behavior and
handle any source correction in a separate ticket.

The shared adapter must retain the exact final `st.px`, `st.py`, and full
`sim.move_flags` byte. Preserve wall retry coordinates, stale X on ground
alignment early returns, and untouched scratch/flags when no probe occurs.
Do not clear, restore, or normalize these outputs around a probe.

Before extraction, add characterization tests that exercise the actual
adapters with seeded scratch and flag values. Cover:

- Both wall directions, first-sample hit, retry hit, and clear wall, including
  tile threshold and coordinate-wrap boundaries.
- Aligned ground hit/miss and misaligned early return with actor Y high byte
  different from the seeded scratch high byte.
- No-probe initialization, pause/spawn, and strike-animation paths.
- Even-frame movement before probing, odd-frame ledge reversal, and generic
  fallwalker falling/landing with its real fall operation.

Check exact scratch coordinates, full flag bytes, and full slot bytes, plus
entry-specific spawn/facing/diagnostic outputs where relevant. Keep existing
landing walker (including anim 66), projectile walker, striking patroller,
and generic fallwalker coverage. A small shared adapter test matrix can cover
probe geometry; retain caller regressions to verify wiring and ordering.

Allowed production files: `enemies/fallwalker.rs`, the three named native
enemy modules and compatibility adapters, the bounded fallwalker dispatch
section, the new shared compatibility module, and module registrations.
Tests stay in the existing enemy suites or a focused terrain-adapter suite
under `enemies/tests/`. The separate `SimWalkerWorld` used by walker/crawler
dispatch and other terrain consumers are outside this ticket.

## Verification and commits

For each work step, run its focused tests before and after extraction, then
`cargo test -p sml2-core`. New boundary tests must detect a deliberate change
to the transition, mask, staging order, or scratch behavior they protect.
Keep source-derived expected bytes explicit. Commit each completed, verified
step with only its files staged; preserve the user's existing `README.md`
change.

Treat both steps as one work package. After both core gates pass, run once:

```bash
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

The frozen sweep supplements the unit tests; it does not establish coverage
for these branches. Add no captures, dependencies, queue status claims, or
unrelated handler changes. Record any baseline failures separately rather
than expanding either refactor to fix them.

Planning limitation: the two 2026-10-15 workflow plans linked from `AGENTS.md`
are absent in this checkout. This plan uses the available canonical
`PORTING.md`, orchestration rules, native porting guide, source briefs, and
current code.

## Verification results

- Platform characterization tests passed before and after extraction; all
  1,689 core tests passed at that step. A deliberate `>=` reversal mutation
  failed the wrapping/reversal regression test.
- Terrain characterization tests passed against the original adapters and
  the shared adapter. The patrol-only test world compiles without a fall
  operation. All 1,694 core tests passed after both refactors.
- Deliberate missing-X-staging, wrong-ground-mask, and discarded-probe-Y
  mutations each failed the targeted terrain regressions; all mutations
  were restored before the final core run.
- Formatting and diff whitespace checks passed. The frozen replay gate ran
  once after both steps and passed, retaining only its documented
  `traj4_ff0` legacy mismatch allowance. No capture files changed.
- The unrelated local `README.md` edit remains outside these commits.
