# Desktop shell

The desktop hosts the native runtime session, with GPU presentation, keyboard/
controller bindings, settings and debug tools. ROMs and imported graphics are
not distributed. For a quick start, see [Getting Started](Getting-Started.md).

## Build and run

```sh
cargo run -p sml2-desktop
cargo run -p sml2-desktop -- --play
```

A normal launch opens the title with **Play**, **Settings**, **Quit**. If data is
missing, Settings -> Game opens first: import the original **SML2 UE v1.0** ROM.
Replacement/import validates the exact revision, runs in the background and can
be cancelled; failure retains the previous installation. If the system picker
returns no file, an in-app browser accepts navigation or a pasted path.

Play enters file select. `--play` starts on the overworld; `--title` opens the
title; `--file-select` opens the pipes. Developer `--assets DIR` accepts an
existing extraction (`cargo run -p sml2-extract`). Run `--help` for all options.

Linux needs a C toolchain, pkg-config, libudev development files and Vulkan or
OpenGL. X11 also needs libxkbcommon-x11. Packages:

```sh
# Fedora
sudo dnf install gcc gcc-c++ pkgconf-pkg-config systemd-devel libxkbcommon-x11
# Debian/Ubuntu
sudo apt install build-essential pkg-config libudev-dev libxkbcommon-x11-0
```

Wayland shows the coin icon only through a desktop entry for app ID `sml2`.
Run `tools/install-desktop-entry.sh [BINARY] [PREFIX]` (default
`target/release/sml2` into `~/.local`). WSLg reads only system folders, so use
`sudo tools/install-desktop-entry.sh target/release/sml2 /usr/local`.

Wayland/X11 are enabled; platform backends include Vulkan/GLES, DX12 and Metal.
For WSL driver-probing failures, Mesa software Vulkan may help:

```sh
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
WGPU_BACKEND=vulkan WGPU_ADAPTER_NAME=llvmpipe \
cargo run --release -p sml2-desktop
```

Use your distribution's installed `lvp_icd` filename. This renders on the CPU
for that launch. Mesa probing errors do not necessarily mean rendering failed.
Windows packaging/build instructions: [Windows builds](windows-builds.md).
Live audio uses WASAPI on Windows, CoreAudio on macOS, and PulseAudio (`pacat`)
or PipeWire (`pw-play`) on Linux. Without an output device, the game runs
silently.

## SML2DX Mode

After importing the original ROM, use **Settings -> Game -> Add DX ROM / IPS…**
for the audited v1.8.1 ROM or IPS patch against UE v1.0, then enable **DX Mode**.
IPS applies to the retained original without modifying your file. Older original
installs may need **Replace ROM…** once to retain that source. Old DX caches
require **Replace DX ROM / IPS…**. Unsupported digests/builds are rejected.

Choose Mario/Luigi in **Settings -> Mods**, or walk right past Clear through the
DX file-select character door; that choice is saved. Luigi uses DX's movement
curves and friction. DX supplies every level/pipe room/demo, space character
graphics, title, file select, maps/minigames, boss arenas, castle, Game Over,
ending and credits. Eight levels have regional BG/OBJ changes. Star/pause
colors, DX fades, white blanking, castle entry and coin flash, and the credit
palettes are supported. Regional palettes follow the pose pass's staged
references, and boss scene loads override them. Palette uploads do not yet
follow the ROM's VBlank job-queue order. Full-game DX verification is open.
See the [DX audit](sml2dx-audit.md) for exact compatibility and evidence limits.

DX ignores the selected display filter. A render failure falls back to the
filter and reports an error in Settings -> Game. Saves/progress remain shared.
Imported data stays in the local `dx` cache; startup validates its identity,
base assets and integrity before use.

## Controls and settings

| Action | Keyboard | Controller |
|---|---|---|
| Move | Arrows | D-pad / left stick |
| Jump / A | Z / Space | South |
| Run / B | X / Shift | West / East |
| Start / confirm / original level pause | Enter | Left stick click |
| Select | Backspace | Select / Back |
| Desktop menu / resume | Escape | Start |
| Menu navigate / confirm / back | Arrows / Enter or Space / Escape | D-pad or stick / South / East or Start |
| Fullscreen | F11 | Settings |
| Output scale | + / - | Settings |
| Display palette | F4 | Settings |
| Toolbar | F10 | Mouse |
| Debug inspector / pause / one tick | F1 / F2 / F3 | Debug panel |

The desktop menu, focus loss, minimization and selected-controller disconnect
suspend ticking independently of the original game's pause. Confirmation/binding
buttons wait for release; short taps survive to the next tick, opposite directions
cancel consistently. Debug stepping advances one outer session tick.

Settings pages: Display, Audio, Controls, Mods, Game, Debug. PgUp/PgDn or LB/RB
switches pages; left/right changes choices, Enter/click advances them. Escape
cancels binding capture; conflicts/reserved shortcuts are reported. Bindings use
physical key positions; changing one device retains the other's bindings.

Menus preserve the last displayed frame underneath. **Fit to window** adjusts
output scale without changing the logical view; **Integer scaling** is on by
default. Disabling Fit exposes fixed 1x–8x presets; disabling Integer allows
fractional scaling. Output uses physical pixels and nearest sampling.

**Mods -> View** offers widescreen Off/16:9/21:9/Window size and camera zoom
100/125/150/200%. Native logical size is 160x144; zoom 200% doubles dimensions.
Window size follows the game-area aspect ratio. Enlarged levels put three
quarters of extra height above the native rect and one quarter below. The
**Original** status bar stays bottom-center; **Modern** (Display -> HUD) floats
its tiles in the view's top corners. **Motion smoothing** draws levels between
ticks on displays faster than 60 Hz, one frame behind. Wider object
spawn/update windows affect gameplay; attract demos
retain native spawning. Hub maps expand within 512x512 bounds; zones and fixed
screens keep their authored 160x144 picture, with approved surroundings or bars.

**Display** includes Grayscale, DMG green, Pocket, GBC auto, 32 SGB presets
1-A..4-H, and Amber. Preview and paused game update as filters change. GBC auto
preserves separate BG/OBJ palettes; old `SGB color` settings migrate to 1-A.
Startup accepts `--filter gray|dmg|pocket|gbc|sgb|sgb-1a|...|sgb-4h|amber`.
See [hardware palettes](hardware-palettes.md) for source tables/colors.

Settings/saves use platform config/data directories. On Linux:
`~/.config/sml2-rs/settings.json`, `~/.local/share/sml2-rs/saves/default.sav`
(with XDG overrides). Writes are atomic; unreadable/unknown-version settings are
retained until explicitly reset. CLI `sml2.sav` is separate: use `--save ./sml2.sav`
to select it deliberately. `--config FILE`, `--save FILE`, `--scale 1..8` and
`--fullscreen` override startup choices.

## Scenery

Approved authored scenery is embedded and shared across saves and CLI/desktop.
It changes rendering only. Local drafts are ignored by the game; author/export
with the optional [Scenery Editor](scenery-editor.md). No editor menu is built
into the game.

## Validation

Tests cover persistence/recovery, input/rebinding/focus, controller routing,
menus, debug stepping, tick budgets and viewport geometry. GUI-independent
policy tests: `cargo test -p sml2-desktop --no-default-features --lib`.
The default workspace members exclude desktop; choose `-p sml2-desktop` or
`cargo test --workspace` explicitly.

A bounded native-window smoke run uses isolated settings/save paths:

```sh
cargo run -p sml2-desktop -- --play --no-audio \
  --config /tmp/sml2-preview/settings.json --save /tmp/sml2-preview/game.sav \
  --smoke-frames 30 --screenshot /tmp/sml2-game.png
```

Visible game pixels without debug overlays are compared against the submitted
CPU frame; mismatch/write failures return errors. Without screenshot, the smoke
flag only closes after that many UI updates. Linux/Wayland rendering has been
checked; physical controller hotplug and Windows/macOS still need hands-on
validation. Core tests/replays do not establish GPU or full-game pixel parity.
