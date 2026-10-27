# Getting started

Use a Windows x64, Linux x86_64 or macOS Apple Silicon build and your own
**Super Mario Land 2 UE v1.0** ROM. Game graphics are not included.

1. Download one file from the release: `six-golden-coins-windows-x64.exe`,
   `six-golden-coins-linux-x86_64` or `six-golden-coins-macos-arm64`. On
   Windows, run it (SmartScreen may ask, since it is unsigned). On Linux, run
   `chmod +x six-golden-coins-linux-x86_64` once, then start it. On macOS, run
   `chmod +x six-golden-coins-macos-arm64` and
   `xattr -d com.apple.quarantine six-golden-coins-macos-arm64` once, since the
   build is unsigned, then start it from Terminal.
2. If game data is missing, Settings -> Game opens. Choose **Import ROM…**
   and select your `.gb`/`.gbc` file. Only the supported digest is accepted.
3. Choose **Play**. Arrows move, Z/Space jumps, X/Shift runs; Enter is Start,
   Backspace Select, Escape the desktop menu, and F11 fullscreen.

Rebind controls and configure display/DX under Settings. See the
[desktop guide](desktop.md) for details.
Run the game with `--help` for CLI options. Live audio uses WASAPI on Windows,
CoreAudio on macOS and PulseAudio/PipeWire on Linux.
