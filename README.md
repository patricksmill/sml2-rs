# sml2-rs

Rust port of Super Mario Land 2. Reads assets from the sibling byte-identical
disassembly [`sml2decomp`](../sml2decomp); see [`DESIGN.md`](DESIGN.md).

## Setup

```sh
# default: ../sml2decomp next to this repo
export SML2_DECOMP_ROOT=/path/to/sml2decomp   # optional override

cargo build --release
cargo run -p sml2-extract                      # writes ./extracted/
```

## Replay gate

From this repo root, after extract (`cargo run --release -p sml2-extract` — it
also writes the per-level `levels/levelNN.edge.bin` camera edge maps and the
parallel `levels/levelNN.scene.bin` scene maps):

```sh
for f in traj4_ff0 traj5 traj6 traj7 traj8 traj9 traj10 traj11 traj12 traj13 \
         traj14 traj15 traj16 death1 death2 death3 crouch_pan1 screen_transition1; do
  ./target/release/sml2-runtime --level 0 --replay captures/$f.json | tail -1
done
```

Every field on that line must read `0` (`traj4_ff0` keeps its documented legacy
`$FFF0` divergence) and no `note:` line may appear. A `note: level edge table
missing` means the extract step was skipped or stale — the replay then falls
back to injecting the capture's `$FF98` and the run no longer proves the port's
own edge map. The `edge` field is that proof: it derives `$FF98` from
`levelNN.edge.bin` at the camera's cell and compares it with the captured byte.

(`traj4.json` is legacy; prefer `traj4_ff0`. `traj13_full` is retired and not tracked.)

## Captures

PyBoy scripts under `captures/` use `SML2_DECOMP_ROOT` (same default). See
`captures/README.md`.
