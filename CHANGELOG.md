# Changelog

All notable changes to Six Golden Coins will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.1.1] - 2026-10-27

### Fixed
- macOS plays sound through CoreAudio; it no longer needs PulseAudio.

## [v0.1.0] - 2026-10-27

First release. Each download is a single executable; bring your own
**Super Mario Land 2 UE v1.0** ROM and import it from **Settings → Game**.

### Added
- Native Rust port of Super Mario Land 2: 6 Golden Coins with a fixed 60 Hz
  simulation, for Windows x64, Linux x86_64 and macOS Apple Silicon.
- In-app ROM verification and import; no extracted files are needed.
- Widescreen (16:9, 21:9 or window size), camera zoom-out up to 200%, and
  motion smoothing for displays faster than 60 Hz.
- Game Boy, Pocket, Game Boy Color, Super Game Boy and Amber display palettes,
  plus an optional Modern status bar.
- SML2DX v1.8.1 support from your own ROM or IPS patch: DX graphics, palettes,
  level layouts and Luigi's movement, playing as Mario or Luigi.
- Keyboard and controller rebinding, controller hotplug, fullscreen and saved
  settings.

### Known limitations
- Not emulator-exact. Widescreen and camera zoom activate enemies earlier,
  which changes some encounters; use Widescreen Off and 100% zoom for the
  original behavior.
- macOS plays sound only if PulseAudio's `pacat` is installed. Builds are
  unsigned.
- DX palette uploads don't yet follow the original's VBlank job order.
