# sml2-rs

**Super Mario Land 2, with room to see more.**

A native Rust port of **Super Mario Land 2: 6 Golden Coins** for desktop.
Return to Mario Land, explore its six zones, find the hidden routes, and collect
the golden coins to take back Mario's castle from Wario. Play with the classic
Game Boy look, or add SML2DX colors and Luigi, with widescreen views and smoother
motion for modern displays.

## Make yourself at home

- **Widescreen views.** Choose 16:9, 21:9, or an aspect ratio that follows your
  window. See more of the level around Mario.
- **Zoom the camera out.** Reveal more above, below, and ahead of you, with view
  presets up to 200%.
- **Motion smoothing.** Optional frame interpolation makes scrolling and movement
  smoother on high refresh rate displays while gameplay stays at 60 ticks per
  second.
- **SML2DX colors.** Bring color to levels, maps, menus, bosses, and the ending,
  with a choice of Mario or Luigi graphics.
- **Classic display palettes.** Pick grayscale, Game Boy green, Pocket, Game Boy
  Color, Super Game Boy palettes, or amber.
- **Keyboard or controller.** Rebind your controls, adjust the stick dead zone,
  and play windowed or fullscreen. Saves and settings are kept between launches.

Widescreen, camera zoom, and motion smoothing live in **Settings → Mods**.
Display palettes, scaling, and fullscreen are in **Settings → Display**. Keep
the original view for classic framing, or open things up to suit your screen.
Larger views also activate enemies and items earlier. Motion smoothing adds one
frame of display latency.

## Start playing

1. Get the desktop app. Windows players can follow the
   [download instructions](docs/windows-builds.md#download-and-run); Linux players
   can use the [build and launch guide](docs/desktop.md#build-and-run).
2. Launch the game. On first launch, **Settings → Game** opens so you can choose
   **Import ROM…** and select your original **Super Mario Land 2 UE v1.0** ROM
   (`.gb` or `.gbc`).
3. Choose **Play**, pick a save pipe, and head into Mario Land.

You supply the original game ROM; ROMs and imported game graphics are not
included.

## Play in color

Already have **SML2DX v1.8.1**? After importing the original game, open
**Settings → Game → Add DX ROM / IPS…** and select your patched ROM or IPS patch.
Enable **DX Mode**, then choose Mario or Luigi under **Settings → Mods**.
Your save progress carries between the classic and DX looks.

DX Mode uses SML2DX graphics and colors with the port's native gameplay physics
for both characters. Some palette transitions and fades are still missing. See
[DX setup and compatibility](docs/desktop.md#sml2dx-mode) for supported files and
details.

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

Change bindings in **Settings → Controls**. Press Escape or controller Start
during play to pause, adjust settings, or return to the title. The game also
pauses when you switch away from the window or disconnect your selected
controller.

For more display options, save locations, and setup help, see the
[desktop guide](docs/desktop.md).
