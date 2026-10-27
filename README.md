# Super Mario Land 2 Plus

An enhanced desktop edition of **Super Mario Land 2: 6 Golden Coins**.

## Features

- Native Rust port, not an emulator, verified against the original game code.
- Widescreen: 16:9, 21:9, or your window's aspect ratio.
- Camera zoom-out presets up to 200%.
- Optional frame interpolation for high refresh rate displays.
- SML2DX graphics and colors, with Mario or Luigi.
- Game Boy, Pocket, Game Boy Color, and Super Game Boy display palettes.
- Rebindable keyboard and controller inputs, fullscreen, and saved settings.

View options are in **Settings → Mods**; palettes and scaling are in
**Settings → Display**. Expanded views activate enemies earlier. Frame
interpolation adds one frame of display latency.

## Getting started

Release builds target **Windows x64**, **Linux x86_64**, and **macOS Apple
Silicon**. The first release is pending.

1. Download your platform's package from
   [Releases](https://github.com/patricksmill/sml2-rs/releases) when available,
   or [build locally](docs/desktop.md#build-and-run).
2. Launch it and use **Settings → Game → Import ROM…** to select your original
   **Super Mario Land 2 UE v1.0** ROM, then choose **Play**.

ROMs and imported game graphics are not included.

## SML2DX

Import an **SML2DX v1.8.1** ROM or IPS patch through
**Settings → Game → Add DX ROM / IPS…**, then enable **DX Mode**.
Choose Mario or Luigi in **Settings → Mods**. Saves work in either mode.

Both characters use the port's native physics. Some DX palette transitions and
fades are still missing. See [DX compatibility](docs/desktop.md#sml2dx-mode).

## Controls

| Action | Keyboard | Controller |
| --- | --- | --- |
| Move | Arrow keys | D-pad / left stick |
| Jump | Z / Space | Bottom face button |
| Run / fire | X / Shift | Left or right face button |
| Start / confirm | Enter | Left stick click |
| Select | Backspace | Select / Back |
| Pause menu / resume | Escape | Start |
| Fullscreen | F11 | Settings menu |
| Cycle display palettes | F4 | Settings menu |

Change bindings in **Settings → Controls**. See the
[desktop guide](docs/desktop.md) for setup help and save locations.
