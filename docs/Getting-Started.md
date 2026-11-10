# Getting started

Use a Windows x64, Linux x86_64 or macOS Apple Silicon build and your own
**Super Mario Land 2 UE v1.0** ROM. Game graphics are not included.

1. Download your platform's file from the latest release:
   `SixGoldenCoins-vX.Y.Z-windows-x64.zip`,
   `SixGoldenCoins-vX.Y.Z-linux-x86_64.AppImage` or
   `SixGoldenCoins-vX.Y.Z-macos-arm64.zip`. On Windows, unzip and run
   `sml2.exe` (SmartScreen may ask, since it is unsigned). On Linux, run
   `chmod +x` on the AppImage once, then start it. On macOS, unzip and
   right-click **Six Golden Coins.app** → **Open** the first time, since the
   build is unsigned. Later versions install from **Help → Check for Updates…**.
2. If game data is missing, Settings -> Game opens. Choose **Import ROM…**
   and select your `.gb`/`.gbc` file. Only the supported digest is accepted.
3. Choose **Play**. Arrows move, Z/Space jumps, X/Shift runs; Enter is Start,
   Backspace Select, Escape the desktop menu, and F11 fullscreen.

Rebind controls and configure display/DX under Settings. See the
[desktop guide](desktop.md) for details.
Run the game with `--help` for CLI options. Live audio uses WASAPI on Windows,
CoreAudio on macOS and PulseAudio/PipeWire on Linux.
