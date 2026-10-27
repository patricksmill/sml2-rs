# Getting started

Use a Windows x64, Linux x86_64 or macOS Apple Silicon build and your own
**Super Mario Land 2 UE v1.0** ROM. Game graphics are not included.

1. Launch `sml2.exe` (Windows) or `./sml2` (Linux/macOS). On Linux, use
   `chmod +x sml2` if needed; on macOS, use Open from the context menu for the
   unsigned executable.
2. If game data is missing, Settings -> Game opens. Choose **Import ROM…**
   and select your `.gb`/`.gbc` file. Only the supported digest is accepted.
3. Choose **Play**. Arrows move, Z/Space jumps, X/Shift runs; Enter is Start,
   Backspace Select, Escape the desktop menu, and F11 fullscreen.

Rebind controls and configure display/DX under Settings. See the included
`Desktop.md` (or [source desktop guide](desktop.md)) for details. Run
`sml2 --help` for CLI options. Use `--no-audio` on platforms without the current
PulseAudio/PipeWire backend.
