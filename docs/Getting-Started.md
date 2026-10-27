# Getting started

Use a Windows x64, Linux x86_64 or macOS Apple Silicon build and your own
**Super Mario Land 2 UE v1.0** ROM. Game graphics are not included.

1. Launch `sml2.exe` (Windows), `./sml2-linux-x86_64` (Linux) or
   `./sml2-macos-arm64` (macOS). On Linux, use `chmod +x sml2-linux-x86_64` if
   needed; on macOS, use Open from the context menu for the unsigned executable.
2. If game data is missing, Settings -> Game opens. Choose **Import ROM…**
   and select your `.gb`/`.gbc` file. Only the supported digest is accepted.
3. Choose **Play**. Arrows move, Z/Space jumps, X/Shift runs; Enter is Start,
   Backspace Select, Escape the desktop menu, and F11 fullscreen.

Rebind controls and configure display/DX under Settings. See the
[desktop guide](desktop.md) (`Desktop.md` in the tools package) for details.
Run the game with `--help` for CLI options. Live audio uses WASAPI on Windows
and PulseAudio/PipeWire on Linux. macOS plays sound only if PulseAudio's `pacat`
is installed; otherwise the game runs silently.
