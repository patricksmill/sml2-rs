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

In **Settings → Mods**, turn on **DX Mode** or choose **Import SML2DX ROM…**, and
select your patched v1.8.1 ROM built against the original UE v1.0 release.
Turning DX Mode on before anything is installed opens the import straight away.
The importer supports the exact ROM digest documented in the
[DX audit](sml2dx-audit.md); other revisions and modified builds are rejected.
Existing extracted base assets are required.

If the system dialog returns no file (including when a Linux desktop portal is
unavailable), an in-app file browser opens. Navigate folders or paste the full
ROM path and choose **Open ROM**. **Cancel** or Escape closes the browser.

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

The title and file select use DX's own screens. The map and other non-level
screens keep the display filter for now. So do boss
arenas and boss fights: DX gives each arena its own palettes, VRAM1 graphics and
attribute map and draws bosses through a separate path, which are not imported
yet. Eight levels (`$03`, `$07`, `$0C`, `$0D`, `$15`, `$16`, `$18`, `$19`) also
switch palette sets as Mario crosses regions; they currently keep their starting
DX palette throughout. DX's palette fades are not reproduced. A DX rendering
failure falls back to the filter with the error shown on the Mods page.

Installations from earlier versions covered only the opening level and must be
re-imported (**Re-import SML2DX ROM…**). DX physics and importing without
extracted base assets remain future work. No ROM or imported graphics are
distributed with the application.

## Controls and settings

| Action | Keyboard | Controller |
|---|---|---|
| Movement | Arrows | D-pad / left stick |
| Jump / A | Z or Space | South |
| Run / B | X or Shift | West or East |
| Original game Start | Enter | Left stick click |
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

Settings include auto integer fit, 1x–8x output scale, fullscreen, volume/mute,
controller selection, stick dead zone, and gameplay rebinding. Selecting a
binding row captures a single keyboard key or controller button. Esc cancels;
conflicts and reserved desktop shortcuts are reported. Rebinding one device
keeps the other device's bindings; Reset controls restores the defaults.
Keyboard bindings use physical key positions, matching ordinary game controls.

The game view remains 160x144. Output scaling uses integer **physical** pixels,
with nearest sampling and centered letterboxing, independent of UI DPI.
Manual scale presets resize the window when windowed and fit within the current
window if the platform limits its size. Auto fit follows window dimensions.
Debug inspection shows map/player/camera/scene state, object slots, sound-driver
state and diagnostics. Debug stepping advances exactly one outer session tick.

F4 cycles display palettes: Grayscale, DMG green, Pocket, GBC auto, the 32 SGB
presets (1-A through 4-H), and Amber. **Settings → Display** provides the same
choices with a preview: an opening-level frame (background, Mario and the HUD)
recolored live as you step through filters. The launcher backdrop follows it too.
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

## Validation

Automated tests cover settings persistence/recovery, key/controller input,
rebinding, short taps, dead-zone hysteresis, focus loss, menu confirmation,
fullscreen requests, exact debug stepping, tick budgets at 60/90/120/144 Hz,
and physical-pixel viewport geometry. The existing core/runtime suite and
frozen replay gate remain the parity checks.

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
