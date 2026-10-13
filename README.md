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

From this repo root, after extract:

```sh
for f in traj4_ff0 traj5 traj6 traj7 traj8 traj9 traj10 traj11 traj12 traj13 \
         traj14 traj15 traj16 death1 death2 death3; do
  ./target/release/sml2-runtime --level 0 --replay captures/$f.json | tail -1
done
```

(`traj4.json` is legacy; prefer `traj4_ff0`. `traj13_full` is retired and not tracked.)

## Captures

PyBoy scripts under `captures/` use `SML2_DECOMP_ROOT` (same default). See
`captures/README.md`.
