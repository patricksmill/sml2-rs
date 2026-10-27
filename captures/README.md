# Frozen replay corpus

Run `bash captures/sweep.sh` from the repository root. The gate builds the
release runtime, replays every retained JSON fixture and rejects runtime errors,
notes or unexpected mismatches. `traj19` uses level 19; all others use level 0.
For one fixture: `cargo run -p sml2-runtime -- --level 0 --replay captures/traj15.json`.

| Fixtures | Coverage |
|---|---|
| `traj4_ff0` | Legacy spawner/phase baseline; only the exact `LEGACY` counts in `tools/replay_gate.py` are allowed |
| `traj5`, `traj6`, `traj7` | Spawn scanning, raw object slots, stomp/removal, platform pacing and item poses |
| `traj10`, `traj11`, `traj12` | Pipes/screen transition, long level traversal, ledge launch, hazards and tower climb |
| `traj13`, `traj14` | Item pickup/secret-route state; different sampling and checks, both retained |
| `traj15` | Block bumps, coin slots and live layout mutation |
| `traj16` | Coin touch and BCD counters |
| `traj17` | Exit activation, fade, tally and handoff |
| `traj19` | Level 20 autoscroll and moon/float state |
| `death1`, `death2`, `death3` | Death arc, full fade/lives/reload gap, hazard negative control |
| `crouch_pan1`, `screen_transition1` | Crouch camera pan and transition timing |

`traj8` and `traj9` were exact record prefixes of retained `traj10`/`traj11`.
Unused `traj4` and legacy recording/probe scripts were removed; Git history
retains them. Remaining fixture bytes were preserved. All 18 fixtures are gated.

These are historical ROM observations, not a porting specification or full-game
acceptance. They do not prove pixel/audio parity. Follow [PORTING.md](../PORTING.md):
no new captures, trajectory scripts or emulator steering to obtain captures.
