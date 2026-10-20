# Repair menu and gameplay comparison offsets

## Objective and evidence

Make native title-to-file-select presentation and the conveyor-to-pipe gameplay
sequence follow the ROM's state ownership, reset rules and update ordering.
Follow `PORTING.md`: bounded tickets, source-derived byte tests, focused commits,
and the frozen sweep once at the package boundary.

Verified starting points for the existing local manual recording:

- The ROM performs a separate file-select loader before updating the cursor;
  the native title handler constructs file select immediately. The drop routine
  itself matches the source. The loader's exact native timing still needs a
  source-derived contract; do not choose a delay from this recording alone.
- With exact position comparison, the first reported mismatch is input 7602:
  level 10 ROM `(402,343)`, native `(401,343)`, during conveyor contact.
- The default four-pixel tolerance instead stops at input 8134: ROM
  `(499,676)`, native `(498,164)`. The camera also differs by 512 in Y. The
  native page change occurs at 8135; this is a timing difference, not evidence
  of a permanent 512-pixel displacement.
- Fresh `LevelContext` construction resets `fff0` and seeds the object phase
  differently from the ROM loader. Same-context `reset_level_start` already
  preserves `fff0` and clears the phase correctly.
- Counter initialization has NOT been shown to cause the missing conveyor
  pixel. Both observed counters are even at 7602; parity alone is insufficient.

The recording under `recordings/` is a local diagnostic input, not a committed
acceptance fixture. Keep ROM assets and recordings out of the commits.

## 1. Make comparison claims precise — tooling ticket

Files: `tools/compare_tas.py`, `tools/tests/test_compare_tas.py`,
`crates/sml2-runtime/examples/tas_bridge.rs`, `docs/tas-playback.md`.

- Add an explicit exact-position comparison mode without changing the existing
  meaning of `--diverge-threshold 0` (disabled).
- Report which fields were compared and their tolerance. Include camera
  coordinates and distinguish movie input index from native update index.
- Add opt-in compact state diagnostics for object counter/update phase,
  standing contact, scene ownership and script cursor. Keep pixels and state
  protocol aligned; retain support for existing state rows.
- Document the existing ROM/native sampling boundaries from the source. Do not
  equate an arbitrary hardware-frame sample with a completed gameplay update,
  or claim pixel agreement from a position check.
- Test protocol framing, exact/tolerant comparisons and modulo-256 projection:
  equal screen positions must not hide different world positions or cameras.

Accept when the tool can clearly distinguish the 7602 position mismatch from
the 8134 page-change mismatch. This ticket changes no game behavior.

## 2. Restore the file-select loading transition — front-end ticket

Files: the front-end branch of `crates/sml2-runtime/src/session.rs`, a focused
transition module if needed, `crates/sml2-runtime/src/tests/acceptance.rs`, and
existing front-end tests. Keep new transition behavior out of a growing monolith.

- Derive the loader's dispatch, elapsed work, LCD/presentation state and input
  ownership from ROM title exit, `$30000`, `$2B8B` and their bounded callees.
- Represent loading explicitly before the first cursor update. Reuse a native
  transition/timing abstraction where possible; do not emulate the CPU or add
  a guessed fixed frame count.
- Verify when held/pressed input is sampled as loading completes.
- Preserve the existing drop behavior. Test cursor Y, sprite and priority at
  its thresholds, plus the first update after loading and ordinary menu input.
- Update acceptance helpers to distinguish loading from the 45-update drop;
  preserve assertions that prove menu behavior.

Accept when loading and drop begin according to the derived contract and the
existing visual comparison procedure confirms the early pipe animation timing.
If the source cannot settle exact timing, record that dependency explicitly;
do not declare an arbitrary delay verified.

## 3. Correct object-clock lifetime and level initialization — shared ticket

Files: a focused session clock/lifecycle helper, the relevant entry/exit seams
in `session.rs`, `crates/sml2-runtime/src/tests/session.rs`, and bounded object
initialization helpers/tests if required.

- Audit the ROM writers/readers of `fff0` and the object update phase through
  new game, level entry, exit, death/re-entry and boss handoff. Separate the
  persistent counter from the scratch phase cleared by the level loader.
- Give persistent clock state an owner that survives replacement of a
  `LevelContext`; transfer its current value through the actual lifecycle.
- Initialize a fresh live level with the source-defined phase. Reuse/reset
  through explicit lifecycle helpers; preserve deliberate replay fixture seeds
  and avoid changing every `ObjectSim::new` caller without auditing its contract.
- Test nonzero counter inheritance, first and second engine updates, wrapping
  at 255, and same-context reset. Assert `fff0`, update phase and resulting
  observable actor/player bytes. A targeted revert must fail the new tests.

Accept this as a reset/lifetime correction independently of whether it resolves
the conveyor mismatch. Never seed 92, 174 or a movie-specific phase as a fix.

## 4. Prove and repair the conveyor discrepancy — one behavior ticket

Files: `objects/platforms/conveyor.rs`, its compatibility adapter and category
tests, plus one runtime integration test. Re-scope to a separate shared-helper
prerequisite if the source identifies contact ordering rather than this handler.

- Recheck the existing recording after ticket 3; report whether the first exact
  mismatch moved, disappeared or remained. Do not infer causality from a change
  in a tolerant pass/fail result.
- Build an in-memory producer-to-consumer fixture from `$2D7D`, `$5A236` and
  `$3CC0`: approach, standing contact, handler initialization/update, rider carry
  and application of the object result. Cover both phase parities and the
  contact frame, not only an already-standing rider.
- Derive expected bytes from the source. Test horizontal movement and carry
  together, then walking/jumping off, with camera deltas checked separately.
- Change only the first proven wrong branch/order. If ticket 3 already resolves
  it, retain the integration regression without adding a compensating patch.

Accept when the source-derived fixture proves the carry result and the local
7602 discrepancy is resolved or explicitly traced to a separate bounded defect.

## 5. Verify pipe handoff and integrate — scene ticket/package gate

Files: `scene.rs`, `scene_pipe_tests.rs`, the relevant camera transition helper
and tests only if implicated, and the runtime scene integration seam/tests.

- Recheck the pipe after the preceding corrections. Follow the causal order:
  edge threshold, scene latch, script ownership, `$4x` page write, waits and END.
- Test scene 17 (`40 21 FF`) through its caller, asserting Mario and camera high
  bytes together, preserved low bytes, and suspension/resumption of gameplay.
- If the page write is still late, fix the first source-proven ordering defect.
  Do not force a warp at input 8134 or add a delay to conceal an earlier offset.
- Compare native-size output with cameras included using the documented visual
  procedure. Inspect menu presentation and pipe transition separately.

Run targeted tests while editing and the full core suite before accepting each
behavior ticket. Run relevant runtime and Python tests for their changed seams.
At the completed package boundary:

```bash
cargo test -p sml2-core
cargo test -p sml2-runtime
python3 -m unittest discover -s tools/tests -p test_compare_tas.py
cargo build --release -p sml2-runtime --example tas_bridge
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

Use the existing local recording unchanged as supplemental evidence, first
through the two problem regions and then through its end. Report exact state
comparison separately from visual results and the ROM final-screen hash.
Retain existing phase-lock labeling; success with ROM-supplied phase does not
establish native clock parity. No new captures, trajectory scripts, VBlank
lag-frame debugging, input retiming or injected per-frame ROM state fixes.

Commit each accepted ticket separately. Close only the specific behavior its
tests establish; this package does not claim whole-game synchronization.
