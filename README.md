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

## Desktop app

The desktop shell adds settings, rebinding, controllers, zoom/fullscreen and
debug inspection/stepping around the native session:

```sh
cargo run -p sml2-desktop
```

Home also offers **Import SML2DX…** for a local-ROM opening-level color preview.
See [DX preview setup and limits](docs/desktop.md#sml2dx-opening-level-preview).

See [Desktop shell](docs/desktop.md) for Linux build dependencies, controls,
storage paths and validation commands. Original CLI commands remain available.

GitHub Actions builds downloadable Windows x64 executables on pushes and pull
requests. See [Windows builds](docs/windows-builds.md) for download and setup
instructions. Game data is supplied separately.

## Replay gate

From this repo root, after extract (`cargo run --release -p sml2-extract` — it
also writes the per-level `levels/levelNN.edge.bin` camera edge maps, the
parallel `levels/levelNN.scene.bin` scene maps, the ROM-wide
`scenes/script_table.bin` scene scripts and the `map/nodes.bin` overworld node
tables):

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
