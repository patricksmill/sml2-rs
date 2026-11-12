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
| Toolbar | F10 | Optional binding in Settings → Controls |
| Debug inspector / pause / one tick | F1 / F2 / F3 | Debug panel |

The desktop menu, focus loss, minimization and controller disconnect
suspend ticking independently of the original game's pause. Confirmation/binding
buttons wait for release; short taps survive to the next tick, opposite directions
cancel consistently. Debug stepping advances one outer session tick.

### Local two-player play: joining and dropping out

Every launch starts single player, even with several controllers connected;
whether Player 2 has joined is never saved. An input device is the keyboard or
one gamepad, and a device belongs to at most one player:

- **Player 1** is the device that first presses Start (gamepad) or Enter
  (keyboard) on the title screen. When there is no title screen (the launcher
  goes straight to a map, or a save is continued into the map), Player 1 is
  the first gamepad in connection order, else the keyboard.
- **Player 2 joins** by pressing Start/Enter on a different device, on the
  title screen or an overworld/zone map only — never in a level, boss arena,
  bonus room, launcher or pause menu. Only Player 1 moves the overworld
  cursor and selects levels. A device that has not joined does nothing else.
- A small corner banner reads "Press + to join" for a gamepad or "Press Enter
  to join" for the keyboard while a joinable device is connected and nobody
  has joined. "Player 2 joined" and "Player 2 left" show briefly afterwards.
- Player 2 can also leave with Select (keyboard: Backspace) on the title or
  an overworld map.
- **Either player's Start/Enter opens the pause menu anywhere**, including
  levels. The menu belongs to the device that opened it; **Drop out** is
  shown while two players are present and removes that owner. Player 1's menu
  never removes Player 2. If Player 1 drops out, Player 2 becomes Player 1,
  keeps its device and takes over the map cursor.

Once joined, both players use the Settings → Controls bindings for their own
device. A keyboard that joins as Player 2 therefore uses Player 1's configured
keyboard layout, not a fixed WASD layout; one keyboard cannot be both players.
Focus loss and the desktop menu release both players' inputs.

Player 2 plays the character Player 1 did not choose (with DX installed,
Luigi's graphics and movement when Player 1 is Mario, and vice versa). Without
DX both players use the cartridge's Mario graphics. The players start 24 px
apart around the entry point.

Controller disconnects follow the gamepad, not the player:

- If **Player 2's** gamepad disconnects inside a level, Player 2 enters a
  bubble and stops responding. It is removed when the map returns, unless the
  same gamepad reconnects first, which resumes control. Outside a level,
  Player 2 is removed immediately.
- If **Player 1's** gamepad disconnects and the keyboard is not Player 2's,
  Player 1 falls back to the keyboard. Otherwise play pauses with "Reconnect
  Player 1's controller" until a gamepad reconnects, or until Player 1 drops
  out from that pause menu and Player 2 is promoted.

The old always-present keyboard Player 2 is still available for testing as
**Settings → Debug → Keyboard Player 2 · WASD**, off by default. With it on,
Player 2 is always present at level entry and uses the fixed layout below;
binding-conflict checks and the conflict screen only run while it is on.
With it off, those WASD keys are ordinary keys and Player 1 may bind them.

| Action | Player 2's debug WASD layout |
|---|---|
| Move | W / A / S / D |
| Jump | F |
| Run | G |
| Spin in the air | H |
| Start / pause | 1 |
| Select | 2 |

Co-op follows New Super Mario Bros. Wii's rules, taken from the
NSMBW-Decomp sources where they exist (`daPlBase_c::checkDispSideLemit`,
`checkDisplayOutDead`, `dAcPy_c::State_Balloon`):

- The camera follows the middle of the players still in play, at up to
  4 px per frame, and stops at the level's scroll locks. Players in bubbles
  do not pull it. NSMBW's own camera code is not decompiled, so this follow
  is this port's design.
- The screen edges of the visible view (native or enlarged) hold both players.
  A player pinned behind a wall is not pushed through it; 16 px off-screen,
  that player goes into a bubble.
- At a scroll lock, the player walking into the edge starts the room change;
  the other player follows just behind. A player who does not take the pipe
  the other player took goes into a bubble.
- A defeated player goes into a bubble that appears after 150 frames, floats
  slowly to the side of the player still in play and pops when touched. The
  popped player
  keeps playing where it burst, with 127 frames of invulnerability. A player
  can also bubble on purpose with Select when not paused. If every player is
  in a bubble, the party spends one shared life and restarts the level
  checkpoint after 210 frames.
- Players do not collide with, carry, or bounce on each other. Moving
  platforms carry each player standing on them, with one shared object-engine
  update per tick.

The bubble outline is drawn by the renderer, because the cartridge has no
bubble graphic.

The keyboard and every connected gamepad are independent input devices, so
two players can use any two of them. A host samples each player's own device,
never a merged stream, so one player's controller cannot move the other.

The runtime's single-player `Session::advance` interface remains available for
diagnostics and replay. A co-op host calls
`set_local_multiplayer(two_players)` before level entry and submits both
inputs with `advance_two_players(FrameInput, PlayerKeys)`. Dropping out uses
`set_local_multiplayer(false)` (or `drop_out_player_one` when Player 2 is
promoted), and a lost Player 2 gamepad uses `bubble_second_player` /
`resume_second_player`.

```sh
cargo run -p sml2-runtime --example local_multiplayer_smoke
cargo run -p sml2-desktop --example keyboard_multiplayer_smoke -- /tmp/sml2-coop.png
```

The runtime smoke exercises all 32 original and expanded level entries,
independent movement, rendering, bubbles, party wipes and exit handoffs,
plus representative pipe, boss-arena and moving-platform probes. The desktop
smoke routes synthetic keyboard events through the actual input sampler and
game host and can write a two-player CPU frame. Neither establishes an
uninterrupted two-player full-game playthrough or physical-keyboard rollover
limits.

### Settings and controllers

Settings pages: Display, Audio, Controls, Mods, Game, Debug. PgUp/PgDn or LB/RB
switches pages; left/right changes choices, Enter/click advances them. Escape
cancels binding capture; conflicts/reserved shortcuts are reported. Bindings use
physical key positions; changing one device retains the other's bindings.

macOS reads extended gamepads through Apple's GameController framework;
Windows and Linux use gilrs. USB and Bluetooth use the same bindings. Select
the device and stick dead zone in Settings → Controls. macOS device preferences
identify a controller model, while the active connection distinguishes identical
pads. An older saved device preference falls back to a connected controller;
selecting a device saves the new preference without resetting bindings.

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

For the macOS Bluetooth input change, physical DualSense validation is still
required before claiming the reported issue is resolved. Test the packaged
executable with Bluetooth connected before launch, USB connected before launch,
USB unplugged during play followed by Bluetooth reconnect, and Bluetooth
disconnect/reconnect. Check movement, face buttons, shoulders/triggers, stick
clicks, Menu/Options, menu navigation, rebinding, focus loss/recovery, and no stuck
inputs. Disconnecting the selected pad should open the desktop menu; the
reconnected pad should control that menu and resumed gameplay. Also test device
cycling with two pads, including two of the same model when available.

`cargo test -p sml2-desktop --lib controllers::` runs portable input policy tests
and, on macOS, tests using Apple's writable snapshot controllers. Snapshot tests
verify API mapping and callback handling without paired hardware; they do not
validate Bluetooth delivery or physical hotplug. The release build matrix runs
these tests on macOS, Windows, and Linux.
