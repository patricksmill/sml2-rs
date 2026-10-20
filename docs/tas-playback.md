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

The window shows the input frame number, held buttons, native state, and Mario's
position/speed-table index in its title. `R+B` and `L+B` distinguish alternating
directions; `R+L+B` would explicitly indicate both. The speed index is the ROM's
encoded velocity-table selector, not pixels per frame.
Space pauses/resumes; Right advances one frame while paused; Tab toggles between
normal and quarter-speed playback; Escape exits. Slow playback preserves every
input row and changes only the viewing rate.
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

The imported 4042 movie contains **zero simultaneous Left+Right frames**. Its
rapid direction changes are intentional TAS input, not an importer conflict.
Once gameplay diverges, later inputs address different obstacles/positions and
cannot be interpreted as a successful movement test on their own.

## Live PyBoy and port comparison

Build the native pipe host, then use a Python environment containing PyBoy,
NumPy, and PySDL2 (including SDL2 binaries on Windows):

```bash
cargo build --release -p sml2-runtime --example tas_bridge
python tools/compare_tas.py target/tas/4042.json --rom ../sml2decomp/baserom.gb
```

On this WSL installation the Python executable is
`/home/pmill/.venvs/sml2-pyboy/bin/python`. On Windows pass
`--bridge target/release/examples/tas_bridge.exe`.

Two SDL windows display actual PyBoy ROM output and actual native session
output. One controller sends exactly the same input row to both, waits for both
to finish the tick, then draws both frames. Space pauses/resumes both, Right
steps both, Tab toggles normal/quarter speed, and Escape or closing either window
closes both. Use `--paused` to begin paused. At EOF both frames remain displayed.
The titles show the same zero-based input row and held buttons. Window position
requests place PyBoy on the left and the port on the right; the desktop may
choose different positions. This viewer is silent.

The ROM is supplied from memory with fresh cartridge RAM; neither your `.ram`
file nor native save is read or written. Only the imported movie controls the
games. No savestates, memory edits, or input retiming are used. PyBoy normally
uses its own replacement boot ROM; `--bootrom PATH` accepts a local DMG boot ROM.
The source movie used Gambatte in BizHawk, so equal input indices do **not** imply
PyBoy reproduces that movie correctly. Boot timing and core differences may
already desynchronize the reference. Use the original published video to check
PyBoy's progress before attributing a mismatch to native physics.

`--headless --frames 1200` checks that the emulator and bridge can consume a
shared input prefix without opening windows. This is a live inspection tool,
not an automated trajectory-capture or acceptance-test harness.

## PyBoy compatibility search (2026-10-19)

The following published movies were tried with PyBoy 2.8.1, original Game Boy
mode, fresh cartridge RAM, the project's SHA1-matching UE v1.0 ROM, PyBoy's
built-in boot ROM, and unchanged input rows:

| Publication | Author / run | Result in this setup |
| --- | --- | --- |
| [4042](https://tasvideos.org/4042M) | dadinfinitum, 20:49.86 | Did not reach the ending |
| [1564](https://tasvideos.org/1564M) | andymac, 21:43.52 | Did not reach the ending; first Start precedes PyBoy's title readiness |
| [1721](https://tasvideos.org/1721M) | andymac & MUGG, game end glitch | Did not trigger the ending |
| [1886](https://tasvideos.org/1886M) | andymac & MUGG, game end glitch | Did not trigger the ending |
| [2651](https://tasvideos.org/2651M) | MUGG & Masterjun, game end glitch | Did not trigger the ending |
| [3746](https://tasvideos.org/3746M) | MUGG, Masterjun & Alyosha, game end glitch | Did not trigger the ending |

The BK2 importer now accepts GBHawk's `P1 ` control-name prefix while continuing
to reject other players. Older VBM inputs were decoded for this exploratory
check using the [documented bit layout](https://tasvideos.org/EmulatorResources/VBA/VBM);
they are not a new supported importer format. Their unused motion-sensor flags
were ignored; no reset, shoulder, SRAM or snapshot starts were accepted.

A separate experiment prepended 0, 1, 2, 4, 8, 12, 16, 24, 32, 48, or 64 neutral
PyBoy ticks and tried up to the first 5,000 input rows of each candidate. None
reached another playable level or triggered credits within that prefix. These
were explicitly modified startup experiments, not successful original movies.
No automatic input steering or per-level retiming was used, and the comparison
viewer still defaults to unchanged inputs.

The two older Soulrivers full-game movies use the Japanese ROM, as documented
in [submission 2093](https://tasvideos.org/2093S) and
[submission 1975](https://tasvideos.org/1975S), so they were not tested against the
UE ROM. An older forum-linked Gambatte conversion could not be retrieved (404).

**No published TAS has been verified to complete in this PyBoy configuration.**
This does not establish incompatibility with every PyBoy/boot-ROM configuration.
Use the movie's original emulator/core for a dependable reference, or explicitly
resynchronize and independently verify a PyBoy adaptation before using it to
judge native-port divergence. Do not describe the live PyBoy viewer as a
verified TAS baseline.
