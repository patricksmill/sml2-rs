# TAS playback and comparison

Literal movie inputs drive one native tick per row. No BIOS/lag emulation, input
retiming, level-state injection or user-save access is implied. A published TAS
working in its original emulator may desynchronize in PyBoy or the native port.
This is a diagnostic tool, not full-game acceptance.

## Import and play

Example source: [dadinfinitum's TAS, publication 4042](https://tasvideos.org/4042M),
BizHawk 2.3.1/Gambatte. Download the `.bk2` (or outer ZIP) locally; it is not shipped.
Import verifies the original UE v1.0 ROM SHA-1 from the movie header and rejects
SRAM/savestate starts, reset/power events, unsupported controllers and malformed rows.

```sh
python3 tools/import_bk2.py target/tas/4042.bk2 target/tas/4042.json --rom ../sml2decomp/baserom.gb
cargo run --release -p sml2-runtime --example tas_playback -- target/tas/4042.json
```

Space pauses, Right steps when paused, Tab selects normal/quarter speed, Escape
exits. Gameplay buttons come only from the movie; playback ends at EOF.
`--headless` runs quickly; `--video target/tas/native.mp4` exports silent video
and neighboring `.events.jsonl`; `--frames 3600` limits a preview. Video requires
ffmpeg with drawtext/libx264; `--encoder libopenh264` is an alternative. Native
60 Hz and movie 59.7275 Hz differ in wall time even with unchanged input rows.

The original 74,651-row attempt ran without crashing but never cleared level 0.
No published TAS was verified to complete in the tested PyBoy setup. Do not use
a later mismatch, after earlier divergence, as an isolated movement test.

## Compare the live ROM and port

Requires PyBoy, NumPy, PySDL2 and SDL2 binaries, plus the native bridge:

```sh
cargo build --release -p sml2-runtime --example tas_bridge
python tools/compare_tas.py target/tas/4042.json --rom ../sml2decomp/baserom.gb
```

On Windows use `--bridge target/release/examples/tas_bridge.exe`. Space/Right/
Tab/Escape control both windows; `--paused` starts paused, EOF leaves both shown.
`--headless --frames 1200` checks a prefix without windows. The viewer is silent
and starts from fresh RAM; `--bootrom PATH` selects a local boot ROM. Check the
reference emulator's progress before blaming the native port.

| Option | Meaning |
|---|---|
| `--mode dx --character mario` | Audited DX ROM, CGB reference and full RGB comparison; `luigi` expects selection through movie inputs |
| `--exact-position` | Stop on any sampled world-X/Y mismatch |
| `--diverge-threshold N` | Default tolerance 4 pixels; 0 disables this threshold |
| `--no-diverge` | Disable comparison stops, including exact mode |
| `--trace START:END` | Inclusive zero-based movie-index trace |
| `--debug-state` | State plus object clocks/contact/scene, camera and movement diagnostics |
| `--pixel-report` | Compare pictures using documented ROM-N/native-N-1 pairing |
| `--no-phase-lock` | Raw hardware-tick/native-step pairing instead of dispatch alignment |

Default phase lock aligns ROM dispatches, not identical movie/update counts:
zero/one/multiple native updates may correspond to a movie row. Title starts on
ROM dispatch $0279; gameplay input uses the $38B9 joypad latch and $05D5 entry; blocking
scenes use $2728 returns. Native loaders wait for the matching ROM entry. Completed
ROM state at $028D avoids half-updates. Map FF97/DIV and witch DIV samples are
comparison-only injected timing inputs; normal play retains native clock policy.
This alignment does not verify native loader durations or hardware CPU timing.

Stop conditions compare level and world coordinates, not camera/speed/contact
or pixels. Equal wrapped screen projection can conceal a world-page difference.
Debug fields do not prove equivalence to arbitrary stopped CPU registers. See
[visual parity](visual-parity.md), [frame clocks](frame-clock.md) and
[randomness](randomness.md) for accepted differences and remaining gaps.

DX comparison uses the supported local DX ROM for both PyBoy and native import:

```sh
python tools/compare_tas.py recordings/dx.json --rom /path/to/sml2dx.gbc --mode dx --character luigi --pixel-report target/dx.jsonl
```

The movie must match that ROM's SHA-1. Both sides boot as Mario; a Luigi movie
must use the file-select character door. `--character` checks the expected
in-level identity and rejects a ROM/native mismatch instead of changing console
RAM. DX reports count pixels with any RGB channel difference; dumps are PPM.
PyBoy RGB555 channels use a left shift (white 248); the comparison expands
the same five bits by replication (white 255) to match native RGB output.
This conversion applies to reference display/dumps, preserving raw EOF hashes.
Their `alignment` field compares the completed ROM logic snapshot with its paired
native level, full world position, camera and character. Outside a level,
`level_state_match` is null: scene equivalence is not established by this check.
Framebuffer `diff` still uses the documented previous native frame; `diff_same`
uses the current frame. Neither score establishes parity after state divergence.
No arbitrary frame offset is applied.

The bridge emits an ASCII `STATE` line then exactly 160x144 grayscale bytes.
`--mode dx --dx-rom PATH` changes the payload to 160x144 packed RGB triples and
adds decimal `mode=1 character=0|1` state fields. Standalone `--character luigi`
sets native initial identity; the movie comparer starts both sides as Mario.
Legacy 5-field and 9-field state rows remain supported; debug adds decimal
`key=value` fields on the same line. Outside a level: `STATE - - - - -`.

## Manual reference inputs

These are user-driven input recordings, not new trajectory fixtures. Agent
work remains subject to [PORTING.md](../PORTING.md)'s capture restrictions.

```sh
python tools/record_play.py --rom ../sml2decomp/baserom.gb
```

Arrows move; Z/X are A/B; Enter/Backspace Start/Select. P pauses; Escape/window
close saves. Focus loss pauses and clears controls; paused time adds no rows.
F5 replaces the current checkpoint; F9 rewinds game AND input history, then
pauses. Resume a checkpoint or append to existing recorded inputs with:

```sh
python tools/record_play.py --rom ../sml2decomp/baserom.gb --load-state recordings/pyboy-TIMESTAMP.state
python tools/record_play.py --rom ../sml2decomp/baserom.gb --resume recordings/pyboy-TIMESTAMP.json
```

Both preserve the original and create a new recording paused at the endpoint.
Resume verifies ROM/boot hashes, PyBoy/settings and final screen hash. `--out`
chooses the new path; `--frames N` limits new ticks only. JSON is published
atomically every 300 frames and at exit under ignored `recordings/`; it contains
input masks and provenance, not ROM bytes, snapshots or memory trajectories.
Checkpoint `.state` files separately contain emulator state/history. Existing
saves are untouched. Record/compare use matching silent-emulation settings.

```sh
python tools/compare_tas.py recordings/pyboy-TIMESTAMP.json --rom ../sml2decomp/baserom.gb --exact-position --debug-state
```

EOF verification confirms the final reference image, not the first native
divergence. Keep local recordings out of the frozen capture corpus.
