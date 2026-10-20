# Watching a Game Boy TAS in the native port

The player imports held buttons from a BizHawk `.bk2` movie, then feeds one
unchanged input row to each native session tick. This is an attempt to replay
the inputs, not a claim of emulator synchronization or full-game completion.
It starts at the native title screen with empty slots and never reads or writes
your save files. It does not run the Game Boy BIOS, reproduce emulator lag, skip
neutral frames, retime inputs, or inject level/progress state.

## Get and import the movie

The initial attempt uses [dadinfinitum's 20:49.86 TAS, publication 4042](https://tasvideos.org/4042M),
recorded with BizHawk 2.3.1 / Gambatte. Download the emulator replay from that page
and put it in `target/tas/4042.bk2` (create the directory first). The importer also
accepts TASVideos' outer ZIP wrapper. The movie is not included in the repository.
It requires the original UE v1.0 ROM; the importer verifies its SHA1 against the
movie header. It refuses SRAM/savestate starts, reset/power events, unsupported
controllers, and malformed input rows.

```bash
python3 tools/import_bk2.py target/tas/4042.bk2 target/tas/4042.json --rom ../sml2decomp/baserom.gb
cargo run --release -p sml2-runtime --example tas_playback -- target/tas/4042.json
```

The window shows the input frame number and current native state in its title.
Space pauses/resumes; Right advances one frame while paused; Escape exits.
Game buttons come exclusively from the movie. Playback closes at the last row;
it does not hand control to the keyboard. On Windows the same commands work
with Python and a native Rust toolchain; this example is a separate executable
from `sml2-desktop` and must be built with `--example tas_playback`.

## Export a video or run quickly

```bash
cargo run --release -p sml2-runtime --example tas_playback -- target/tas/4042.json --video target/tas/4042-native.mp4
cargo run --release -p sml2-runtime --example tas_playback -- target/tas/4042.json --headless
```

Video export requires `ffmpeg` on PATH with the `drawtext` filter and `libx264`.
For Fedora's FFmpeg use `--encoder libopenh264` instead. Add `--frames 3600`
for a one-minute preview. Video is silent, nearest-neighbor scaled, and labeled
as a native port attempt with the input author and frame counter. A neighboring
`.events.jsonl` file records state transitions and level entry/exit outcomes.
The native clock runs at 60 Hz; the original movie plays at about 59.7275 Hz, so
wall-clock durations differ even though every input row is preserved.

## First result (2026-10-19)

All **74,651 input rows** ran without a crash. Native transitions (zero-based
input frame indices):

| Frame | Native result |
| --- | --- |
| 416 | File select |
| 535 | Overworld |
| 609 | Enter level 0 |
| 14,414 | Return to map without clearing level 0 |
| 18,974 | Re-enter level 0 |
| 65,014 | Game over |
| 74,650 | End of input, still in level 0 |

**The TAS did not beat the port or clear its first level.** This run establishes
that the original inputs get through the native menus into gameplay. It does
not locate the first divergent frame: startup/transition timing, physics, and
lag-sensitive glitches still need comparison to the original. The chosen TAS
uses pipe carryover and other glitches, so it is also not an all-level coverage
gate. Manual play remains useful; this tool provides a repeatable visual probe
alongside it. The [original TAS publication](https://tasvideos.org/4042M) links
the reference video and author notes. The generated video here is new native
port output, not the author's emulator recording.
