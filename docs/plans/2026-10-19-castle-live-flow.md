# Castle controls, presentation and live-session routes

This package repairs three independently checked seams. An empty mechanics
queue or handler coverage report does not establish a full-game playthrough.

| Step | Source | Implementation | Acceptance |
|---|---|---|---|
| CW-1: castle controls and transitions | home $3573; bank 12 $446F/$4697/$4748/$4777; bank 15 $4169 | `castle_sequence.rs`, `castle_sequence/door.rs`, `session/castle.rs` | The real $FF map entry starts a visit. Arrival/departure take 64 ticks. With six coins, A/Start/Up starts the 64-tick entry fade into level 24; Down alone leaves; B is ignored. Held map-entry input cannot also confirm the door. Boss deposits use their own state machine and wrapping vertical flight. |
| CW-2: castle presentation | bank 12 $45B6/$49CD/$4A2A, $477B; home $2A63 | `map/castle_render.rs`, extracted castle blobs, runtime render seam | Five overlapping 20-byte row windows match the ROM. VBlank row requests become persistent presentation state and rendering does not consume them. Both object palettes, zero palettes, all six icons, coin sparkle and congratulations assets are displayed. |
| CW-3: continuous session seams | map $3C169; home $1CBA/$3547; map $3EEF0/$3D573 | `Session::advance`, `MapSim::from_player`, `tests/played_routes/continuous.rs` | All 64 ordinary map-origin/header-or-secret-entry cases load the expected level and source coordinates. Six fixed approaches across levels 19/28/29 run from map selection through completion, map return, actual CLI-host disk save and reload. |

The integration tests found and guard against three data-transfer defects:

- Ordinary level entry discarded every other level's progress and reset the tally.
- Reload selected the hub regardless of the saved node. Reload now reuses the
  native map handoff and zone loaders, preserving the shared arrival latch.
- `$3D573` sets the shared dead byte after a successful clear. The session now
  distinguishes that map latch from the completed level's death outcome.

The boss reward adapter already credits its tally. Its castle deposit preserves
that credit and delays the new coin's visual commit until the source timer ends.
The castle return persists progress into the active slot and emits a save.

## Validation

```sh
cargo test --workspace
cargo test -p sml2-runtime played_routes
cargo test -p sml2-runtime castle_flow
cargo test -p sml2-core castle_sequence
cargo test -p sml2-core castle_render
set -o pipefail
bash captures/sweep.sh 2>&1 | tail -n 25
```

Verified: 1,434 core tests, 82 runtime tests plus three runtime integration
tests, 22 desktop tests and nine extractor tests. The release replay gate
passes with the established `traj4_ff0` exception unchanged. Seven temporary
regressions in input dispatch, timer borrowing, row presentation, progress
transfer, clear classification, arrival state and castle map dispatch were
rejected by their targeted tests; all mutations were restored.

`cargo run -p sml2-runtime --example castle_frames` writes native previews to
`/tmp/castle_locked.png`, `castle_open.png`, `castle_faded.png` and
`castle_rows_1.png` through `castle_rows_5.png`. These are native renderer
previews; no console capture corpus was added or modified.

Re-extract assets with the updated extractor. `castle_scripts` now includes the
door-row window table and congratulations tilemap/tiles. Windows executables
and their extracted assets must be updated together.

## Remaining scope

The ordinary entry matrix covers 31 level IDs at 32 map origins, with Wario's
node-free level 24 covered by the actual door transition. Complete played
approaches are covered for levels 19, 28 and 29 (manifest numbers); level 19
has both header and secret starts to both bells. Level 28 also exercises the
equivalent ASCII and typed tapes. The fixtures start at source map nodes and
never rewrite Mario, objects or collision state after input begins.

Full played approaches for the other 29 levels, continuous title/hub navigation
between courses, and an uninterrupted all-bosses-to-ending run remain pending.
Existing arena and ending tests still establish their narrower boundaries.
No full-game pixel or audio parity claim follows from these tests.
