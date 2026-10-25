# Desktop shell

The new `sml2-desktop` executable uses the runtime library through its explicit
session input/effect boundary. The original `sml2-runtime` CLI, frozen replay
adapters, and headless commands retain their behavior.

## Build and run

From the repository root, with the existing extracted game data:

```sh
cargo run -p sml2-desktop
```

Or launch the built executable:

```sh
./target/debug/sml2-desktop
./target/debug/sml2-desktop --play
```

A normal launch opens the in-game title screen, with its wall and grass
filling the window, and a floating bar in the grass below the title sign:
**Play**, **Settings**, **Quit**.
Left/right or the controller D-pad selects an action. Hitting **Play** (or pressing
Enter, Space, or controller South / Start while Play is selected) triggers Start
so Mario walks to a pipe and ducks into file select. **Settings** opens the
settings screen without advancing Mario, and **Quit** exits the app.
Clicking the picture above the grass also starts the game.
**Settings → Game** contains the original ROM importer, ROM replacement,
and optional DX setup. If game data is not yet installed on first launch,
Settings → Game opens so you can add the original **Super Mario Land 2 UE v1.0**
ROM (`.gb` or `.gbc`). Once the ROM is verified, the in-game title starts
with the selected DX Mode setting.

F10 shows or hides the toolbar. Its **Display**, **Audio**, **Controls**,
**Mods**, **Game**, and **Debug** tabs open the matching settings page.
Esc, or the toolbar Menu button, opens the
compact pause menu over the current game frame, with Resume,
Settings and Quit. In-game menus and settings darken the exact last displayed
frame, preserving its scale, colors, overlays, and surroundings until you resume.
Quit to title is on
that menu only after you have left the title. `--play` skips the
title and opens the overworld. `--title` opens the title directly, the same as a
normal launch; it can also be combined with `--smoke-frames` and `--screenshot`
to validate startup rendering. `--file-select` opens the pipes directly.

Linux builds require a C toolchain, pkg-config, and libudev development files
for gilrs. X11 sessions also need libxkbcommon-x11 at runtime. For Fedora:

```sh
sudo dnf install gcc gcc-c++ pkgconf-pkg-config systemd-devel libxkbcommon-x11
```

For Debian/Ubuntu, install `build-essential`, `pkg-config`, `libudev-dev`, and
`libxkbcommon-x11-0`. A working Vulkan or OpenGL driver is needed for the wgpu
presenter. Both Wayland and X11 support are enabled; wgpu's Vulkan, GLES, DX12
and Metal backends are selected by target platform.

On WSL systems with unreliable graphics-driver startup or resize behavior,
`WGPU_BACKEND=vulkan WGPU_ADAPTER_NAME=llvmpipe` selects Mesa's software Vulkan
adapter when available. Native previews were checked with both automatic and
software-adapter selection.

If startup prints `libEGL`, `dzn`, `ZINK`, or
`ID3D12DeviceFactory::CreateDevice failed` messages, Mesa may be probing a
driver that cannot initialize. These messages can appear even when the app
successfully renders through another adapter. Selecting `llvmpipe` alone
still lets the Vulkan loader probe the failing drivers. To bypass those
probes on Fedora WSL, restrict the loader to Mesa's software Vulkan driver:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
WGPU_BACKEND=vulkan WGPU_ADAPTER_NAME=llvmpipe \
cargo run --release -p sml2-desktop
```

The `lvp_icd` filename/location varies by distribution; use the installed JSON
under `/usr/share/vulkan/icd.d/`. This renders on the CPU and applies only to
that launch. A bounded WSLg smoke run with this driver completed without the
Mesa messages and matched the submitted CPU frame pixel for pixel.

The original three crates remain the workspace's default build targets.
Select `-p sml2-desktop` to build the desktop app; `cargo test --workspace`
includes it. Its settings/input/timing policies can also be tested without the
native GUI/controller dependencies:

```sh
cargo test -p sml2-desktop --no-default-features
```

The shell consumes existing extracted base content, which can be produced with
`cargo run -p sml2-extract`.
Use `--assets /path/to/extracted` to run from another directory.

## SML2DX Mode

In **Settings → Game**, **Import ROM…** (or **Replace ROM…** after a ROM is
installed) adds the original game. **Add DX ROM / IPS…** accepts either a
patched ROM or an IPS patch after the original game is verified.
Once SML2DX graphics are installed, **DX Mode** chooses whether Play uses them,
and **Replace DX ROM / IPS…** imports another one. Widescreen, camera zoom,
and character are in **Settings → Mods**.
Select your patched v1.8.1 ROM built against the original UE v1.0 release.
For IPS input, the app applies the patch to a locally retained copy of the
original ROM without modifying your file. The resulting ROM must pass the same
exact digest check as a patched ROM. For original-game installs made before IPS
support, use **Replace ROM…** once to retain the original for patching.
The importer supports the exact ROM digest documented in the
[DX audit](sml2dx-audit.md); other revisions and modified builds are rejected.

If the system dialog returns no file (including when a Linux desktop portal is
unavailable), an in-app file browser opens. Navigate folders or paste the full
file path and choose **Open file**. The browser shows the file types for the
current step. **Cancel** or Escape closes the browser.

Import runs in the background and can be cancelled. The DX ROM's level loader
tables and the 18 banks every level's presentation reads are stored locally in
the application's `dx` cache directory (about 600 KB). A complete bundle is validated before atomic publication. Its identity
includes the ROM digest, base assets, importer version and schema. Failed imports
preserve the selected installation. Settings remember the installation, DX Mode
and the character; startup checks the installation's integrity and base assets.

DX Mode is a presentation layer over the normal game, not a separate session:
saves, progress and the debug panel work as usual. Every level, including its
pipe rooms and the attract demos, uses DX graphics and CGB colors, with sprite
overlays, tile animation, star colors and pause colors; the display filter does
not apply there. **Character** picks Mario or Luigi graphics; both play with the
native rules. The space levels use DX's alternate (space) Mario and Luigi.
On the DX file-select screen, walk right past Clear to use the character door.
The returning character becomes the saved **Character** choice in Settings.

The title, file select, overworld map (hub, zones and both minigames), Game Over,
castle reveal, ending and credits use DX graphics and colors. Boss arenas and
fights use their own DX tiles, palettes and sprite lists too. Seven levels (`$03`, `$07`, `$0D`, `$15`, `$16`, `$18`, `$19`) also
switch palette sets as Mario crosses regions; they currently keep their starting
DX palette throughout. DX's palette fades are not reproduced. A DX rendering
failure falls back to the filter with the error shown in Settings → Game.

Installations from earlier versions, including ones from before the minigames
and scene screens were colored, must be re-imported from **Settings → Game** (**Replace DX ROM / IPS…**).
DX physics remain future work. No ROM or imported graphics are
distributed with the application.

## Controls and settings

| Action | Keyboard | Controller |
|---|---|---|
| Movement | Arrows | D-pad / left stick |
| Jump / A | Z or Space | South |
| Run / B | X or Shift | West or East |
| Start / confirm; pause popup in levels | Enter | Left stick click |
| Original game Select | Backspace | Select / Back |
| Desktop menu / resume | Esc | Start |
| Menu navigation | Arrows, Enter / Space | D-pad / stick, South |
| Menu back | Esc | East / Start |
| Fullscreen | F11 | Settings menu |
| Output zoom | + / − | Settings menu |
| Debug inspector | F1 | Toolbar (mouse) |
| Debug pause / resume | F2 | Debug panel (mouse) |
| Advance one debug tick | F3 | Debug panel (mouse) |

The desktop menu suspends ticking independently of the original game's pause
logic. Losing focus, minimizing, or disconnecting the selected controller also
suspends play. Confirmation/binding-capture buttons wait for release before
they can affect gameplay. Buttons tapped between ticks survive until the next
tick; opposite directions cancel consistently across devices.

Settings include **Fit to window**, **Integer scaling**, 1x–8x output scale, fullscreen, volume/mute,
controller selection, stick dead zone, and gameplay rebinding. Selecting a
choice with the left or right key (or controller direction) moves backward or
forward. The visible arrow controls do the same when clicked. Enter or clicking
the row advances one choice; plain On/Off rows and the controller selector are
buttons. Selecting a binding row captures a single keyboard key or controller
button. Esc cancels;
conflicts and reserved desktop shortcuts are reported. Rebinding one device
keeps the other device's bindings; Reset controls restores the defaults.
Keyboard bindings use physical key positions, matching ordinary game controls.

The game view is 160x144 unless **Settings → Mods → View** enlarges it:
**Widescreen** (Off / 16:9) and **Camera zoom** (100% / 125% / 150% / 200%) give
160x144 through 320x288, or 256x144 through 512x288 at 16:9.
Zoomed-out levels place three quarters of the additional height above the original
view and one quarter below it, reducing the space shown beneath the level.
The gameplay camera is unchanged; terrain, sprites, overlays and object visibility
windows share the upward view bias. Native zoom and overworld/menu framing keep
their existing positions. On the overworld hub, navigation and camera-pan mode
both show the expanded map, clamped to its 512x512 bounds; pan arrows stay at
the view edges. Views larger than the hub show the whole map with bars. Zone
screens keep their original 160x144 picture centered in larger views; unused
tilemap columns stay hidden. Each minigame's original picture stays centered
inside a solid fill matched to the room color, keeping unused tilemap data hidden.
Enlarged level views
spawn and update objects across the wider window, so gameplay differs from the
original (attract demos keep native spawning); the status bar sits bottom
centre, and other fixed screens are letterboxed. Output scaling uses **physical** pixels, with nearest sampling and centered
letterboxing, independent of UI DPI.
**Fit to window** is available under **Settings → Display**, including through
the toolbar's **Display** tab. It automatically adjusts the output scale as you resize
the window, keeping the same game resolution and visible area. **Integer scaling**
is on by default and restricts scaling to whole-number steps. Turn it off to
use fractional scaling and fill more of the window while preserving the aspect ratio.
Turning **Fit to window** off selects a fixed scale without resizing the window and reveals the **Window scale**
presets. Manual presets resize the window when windowed and fit within the
current window if the platform limits its size. Both scaling preferences are saved.
The castle entrance and Game Over screen continue their brick wall and ground
into unused window space at that same scale, keeping the castle centered at its
original size. These surroundings follow the display filter or DX colors and
do not enlarge gameplay.
Debug inspection shows map/player/camera/scene state, object slots, sound-driver
state and diagnostics. Debug stepping advances exactly one outer session tick.

F4 cycles display palettes: Grayscale, DMG green, Pocket, GBC auto, the 32 SGB
presets (1-A through 4-H), and Amber. **Settings → Display** provides the same
choices with a preview: an opening-level frame (background, Mario and the HUD)
recolored live as you step through filters. The paused game behind the settings
popup updates to the selected filter too.
GBC auto uses separate background and sprite palettes selected from the ROM's
title checksum. The old `SGB color` setting migrates to the real SGB 1-A preset.
See [hardware palettes](hardware-palettes.md) for exact colours and source tables.

Select a palette at startup with `--filter gbc`, `--filter sgb` (1-A), or
`--filter sgb-1a` through `--filter sgb-4h`. The existing `gray`, `dmg`, `pocket`,
and `amber` values are also supported.

Settings are JSON in the platform user configuration directory; saves are in the
platform user data directory. On Linux these normally resolve to
`~/.config/sml2-rs/settings.json` and
`~/.local/share/sml2-rs/saves/default.sav`, honoring XDG overrides.
Writes replace files atomically. Unknown-version or unreadable settings are
reported and retained until the user explicitly resets them.

The desktop save location is independent of the old CLI's working-directory
`sml2.sav`. To use that record deliberately:

```sh
cargo run -p sml2-desktop -- --save ./sml2.sav
```

`--config FILE` and `--save FILE` override storage paths. `--scale 1..8` and
`--fullscreen` override startup display preferences. The live audio output
currently uses the runtime's existing PulseAudio/PipeWire backend; `--no-audio`
disables it. Portable in-process audio remains separate follow-up work.

## Scenery

Approved scenery extensions are embedded in port releases and shared by all
three save pipes. The desktop game and CLI use the same bundled content.
Scenery changes rendering only; collision, actors, and routes are unchanged.

Authoring is available in the separate, optional
[Scenery Editor](scenery-editor.md). The normal game has no editor menus and
ignores local `scenery-NN.json` drafts beside the settings file. Existing drafts
remain editable in the companion app. Only exported, reviewed files registered
in the repository ship with a release.

## Validation

Automated tests cover settings persistence/recovery, key/controller input,
rebinding, short taps, dead-zone hysteresis, focus loss, menu confirmation,
fullscreen requests, exact debug stepping, tick budgets at 60/90/120/144 Hz,
and physical-pixel viewport geometry. The existing core/runtime suite and
frozen replay gate remain the parity checks.

Editor tests also cover signed coordinate picking, pan/zoom, gesture-level
autosave and undo, unchanged collision layout, file errors, playtest return,
and window layouts at 480×400, 800×720, and 1600×1000. Atlas pixels are compared with the
native and DX scenery renderers. Optional headless UI review PNGs can be
exported into the system temporary directory with:

```sh
cargo test -p sml2-scenery-editor export_workspace_previews --lib -- --ignored --nocapture
```

A bounded native-window smoke run can capture a screenshot without changing
the user's normal settings or save:

```sh
cargo run -p sml2-desktop -- --play --no-audio \
  --config /tmp/sml2-preview/settings.json --save /tmp/sml2-preview/game.sav \
  --smoke-frames 30 --screenshot /tmp/sml2-game.png
```

When the game view is visible without debug overlays, this also compares every
game pixel in the GPU screenshot against the submitted CPU frame. A mismatch or
screenshot write failure returns an error. `--smoke-frames` without a screenshot
simply closes the app after that many UI updates.

Native rendering has been exercised on Linux/Wayland in windowed, 2x zoom and
fullscreen modes, including exact GPU/CPU pixel comparisons. Physical controller
hotplug and Windows/macOS behavior still need device/platform testing; synthetic
controller events exercise the same action routing and menu paths in tests.
