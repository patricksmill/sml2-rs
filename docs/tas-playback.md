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

### Position comparison and sampling boundaries

The default comparison pauses on the first sampled level difference or world
X/Y difference greater than four pixels. `--diverge-threshold N` changes that
inclusive tolerance; **`--diverge-threshold 0` disables it**. Use
`--exact-position` to pause on any world X/Y difference, including one pixel.
`--no-diverge` disables comparison even with `--exact-position`.

`--trace START:END` prints ROM and native state for an inclusive range of
zero-based movie input rows. Both endpoints must be nonnegative and ordered.
With phase lock, the trace pairs the last completed ROM update with its native
step; with `--no-phase-lock`, it prints the live state after each hardware tick
and native step. Add `--debug-state` to include native object-clock fields.

```bash
python tools/compare_tas.py recordings/<recording>.json --rom ../sml2decomp/baserom.gb --exact-position --debug-state
```

Reports identify the zero-based **movie input index** and the **native update
count**, which starts at zero before any `Session::advance`. Those indices can
differ: phase lock is on by default and may send zero, one, or several native
updates for one movie row. The port takes no step until the ROM's main loop
first dispatches (`$0279`, after the boot ROM and `Init`), so its title starts
on the ROM's title loader. A gameplay update (`$05D5`) uses the buttons the ROM
latched at its last `ReadJoypad` (hooked at `$38B9`). Map frames also carry
the ROM's whole `$FF97` and the `rDIV` values its random-state reads returned
(`$3C13E`, the Space and Turtle zone animations, bell room B's rung picks), as
`f97=` and `div=` fields after the phase; the port's `MapSim::div_reads` takes
them in order, so `sMapRandAccum` and the bonus-game rungs follow the console.
The witch's side and fire-trajectory decisions (`$97FE`/`$979D`) also carry
their divider sample as `boss_div=` on the corresponding gameplay update.
This comparison-only input keeps random choices aligned; ordinary play uses
the native divider. `--debug-state` includes the witch's side, state, attack
phase, platform sprite, tile-animation configuration, and terrain-edit count.
The once-per-frame step
outside a level uses the read that frame's state handler acts on: one taken
before line 144 of that frame, or at the end of the previous tick, since the
tick ends at frame start. Scene steps use the row.
If the native loader finishes before ROM states `$02`/`$03` return, phase
lock holds the native level until the first `$05D5` dispatch. Loading ticks
must not advance its object clock. This is comparison alignment; the native
loader's fixed transition duration is not validated by this mode.
The two agree until a ROM frame overruns the hardware tick: the next read then
happens before the next row is applied, and the ROM acts on the previous row.
The ROM-supplied phases are diagnostic alignment, not proof of native clock
parity. The window's input label identifies the movie row, not a native update.

The sampling contracts come from `home.asm` and the bridge:

- `$05D5` publishes live Mario HRAM `$FFC2/$FFC0` into `$A227/$A229` before
  calling the player routine `$0F71`. With phase lock off, the comparison uses
  this published position and the preceding native row. This is a delayed
  diagnostic approximation, particularly across transitions.
- With phase lock on, each state-`$04` entry (`$05D5`) is one native update,
  whether or not it reaches the player routine `$0F71` (the death arc and the
  screen-transition branch do not); blocking scenes add one per `$2728` return
  (`$273D`). The ROM sample is the memory captured at `$028D`, the main loop's
  point after the state handler returns, paired with the native update for
  the same ROM update. A lagging ROM frame can straddle `PyBoy.tick(1)`'s
  boundary, so live HRAM at the tick can hold half an update; it is used only
  for frames that never reach `$028D`.
- The native row and pixels both follow the corresponding `Session::advance`.
  With no selected update the viewer retains the previous native row and image.
- World camera X/Y (`$FFCA/$FFC8`) are sampled at the current ROM boundary and
  shown with their deltas. They are diagnostic only, especially because the
  non-phase-lock mode's published Mario position is from an earlier update.
  The stop condition compares level and world X/Y only. It does not compare
  cameras, velocities, contacts, or pixels.

Equal screen projections do not imply equal world state: a 512-pixel page
change can preserve `(Mario Y - camera Y) & 255`. For the local recording that
motivated this tool, exact mode distinguishes a one-pixel mismatch at input
7602 from the default tolerance's 512-pixel page difference at input 8134.
These are diagnostic observations, not committed replay acceptance fixtures.
Inspect native-size images separately before making a visual parity claim.

`--debug-state` includes native object counter `fff0`, object update phase,
standing-on-object contact, scene ID, gameplay ownership and script cursor.
The ROM counter (`$FFF0`), phase (`$AF08`), contact (`$A26B`) and scene ID
(`$A28E/$A28F`) shown beside them are current hardware-frame samples. Native
ownership and cursor have no asserted equivalence to arbitrary stopped CPU
registers. Debug fields supplement the position report and do not affect it.

The bridge emits one ASCII `STATE` line followed immediately by exactly
160×144 grayscale bytes. Legacy five-field position rows and nine-field
position/velocity rows remain readable. `--debug-state` implies `--state` and
appends decimal `key=value` fields to the same line; it adds no separate lines
between state and pixels. Outside a level the row remains `STATE - - - - -`.
The comparison viewer tracks native updates even with a legacy bridge, though
opt-in debug fields require rebuilding the current bridge.

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

## Record your own PyBoy reference

```bash
/home/pmill/.venvs/sml2-pyboy/bin/python tools/record_play.py --rom ../sml2decomp/baserom.gb
```

The window records the exact held-button mask for every emulated tick, starting
from fresh cartridge RAM and PyBoy's built-in DMG boot ROM. Arrows move, Z is
A/jump, X is B/run, Enter is Start, and Backspace is Select. P pauses/resumes;
Escape or closing the window finishes and saves. Losing focus pauses emulation
and clears held controls. Paused time contributes no input rows.

**F5 saves a checkpoint; F9 loads it and pauses.** Press P to retry. Loading
rewinds the recorded inputs as well as the game, discarding the failed attempt
from the current recording. F5 replaces your previous checkpoint for this run.
Checkpoints persist beside the recording as `pyboy-<timestamp>.state` files and
include the emulator snapshot and matching input history. To return to one
after closing the window:

```bash
/home/pmill/.venvs/sml2-pyboy/bin/python tools/record_play.py --rom ../sml2decomp/baserom.gb --load-state recordings/pyboy-<timestamp>.state
```

This opens paused, creates a new recording, and preserves the checkpoint you
loaded. F5 then saves to a new checkpoint beside that new recording. The normal
recording JSON still replays from fresh RAM with no snapshot required.

The generated file is `recordings/pyboy-<timestamp>.json`. This local directory
is ignored by Git and survives `cargo clean`. A complete file is published
atomically every 300 frames and at exit. It contains input masks, ROM/boot ROM
hashes, PyBoy version, settings, and the final screen hash. It contains no ROM
bytes, emulator snapshots, or memory trajectories. Existing saves are untouched.
The recorder and comparison viewer currently run silently with sound emulation
disabled, using identical settings.

Continue playing from the end of an existing manual recording:

```bash
/home/pmill/.venvs/sml2-pyboy/bin/python tools/record_play.py --rom ../sml2decomp/baserom.gb --resume recordings/pyboy-<timestamp>.json
```

The recorder replays the existing inputs as fast as possible and verifies the
final screen hash, using the same ROM, PyBoy version and boot ROM. It opens
paused at the last frame; press P to take control. A new timestamped recording
contains the original inputs followed by your new play; the original file
stays intact. Use `--out PATH` to choose the new file. `--frames N` limits new
ticks only, excluding the replayed prefix.

Replay the resulting file using the same Python environment:

```bash
python tools/compare_tas.py recordings/pyboy-<timestamp>.json --rom ../sml2decomp/baserom.gb
```

The viewer verifies the manual recording's ROM, boot ROM, PyBoy version and
settings before playback. If playback reaches the last input, it also checks
that PyBoy's final screen hash matches the recording. That check confirms the
final frame; it does not locate the first divergent frame in the native port.
A 180-frame neutral-input record/replay smoke check matched exactly before the
first manual recording window was opened.

## First manual pipe divergence (2026-10-19)

The first recording exposed two distinct problems:

* At input 1461 (Down at the first pipe), PyBoy was at X=1140, movement phase 0;
  the native session was at X=1138, phase 7. Both had the same horizontal speed
  index `$AC`. The two-pixel difference delayed native pipe entry from the
  phase-aligned frame 1462 until frame 1470. The ROM's `$1189` entry window
  requires X low nibble 5 through 11 with bit 4 set, so a small movement mismatch
  has a much larger timing effect here.
* `$0F71` calls `$13FE` before branching to the travelling arm `$0FBB`. The
  native implementation returned into pipe travel before clearing XSub,
  XPushed, and PipeAlignTarget (`$A202/$A218/$A283`). A vertical pipe could then
  carry the entry frame's horizontal delta into camera follow. This is fixed:
  the reset now precedes the travelling branch. Horizontal pipes still publish
  their own fresh delta. Two ROM-derived regressions failed before the fix and
  pass afterwards.

The speed table matches the ROM. Native `LevelContext` currently starts a new
movement phase at each level, whereas the ROM's `$FF97` counter belongs to the
shared game loop (`$02FF` / `$2728`). Correctly carrying the phase across boot,
loading, and gameplay remains unresolved; native tick count alone is not a
verified console frame origin.

For an **explicit diagnostic experiment**, the comparison viewer can seed the
first native level's phase before its first update:

```bash
python tools/compare_tas.py recordings/<recording>.json --rom ../sml2decomp/baserom.gb --port-initial-phase 1
```

This seeds the pre-update phase, so the first gameplay update increments it to
2. It changes neither input rows nor their timing, applies only once, and labels
the port window `[initial phase 1]`. Without the option, behavior is unchanged.
The pipe reset fix plus this declared alignment let all 18,907 original input
rows run, cleared native level 0 at input 8516, and preserved the exact PyBoy
final-screen hash. That is evidence for the first-level improvement, not a claim
that both levels or all frames now match. Earlier presentation differences and
later map/level transitions still need review.

Do not turn phase 1 into a hardcoded default or call this experiment a general
frame-clock fix. The comparison bridge logs level entry/exit outcomes to stderr;
its stdout remains the raw frame protocol.
