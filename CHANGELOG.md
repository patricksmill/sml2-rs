# Changelog

All notable changes to Six Golden Coins will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [v0.2.0] - Unreleased

### Added
- Expanded levels (Settings → Mods → Levels, on by default): Tree Zone 2 is
  rebuilt as one connected map. Its pipe no longer teleports Mario between
  sections, and the camera scrolls freely to the level's edges. Turn the
  setting off to play the original layout.
- Tree Zone 3's camera scrolls freely through its pipes instead of sliding
  between rooms (with Expanded levels on).
- Scenery for Tree Zones 1, 2 (rebuilt), 3, 4 and 5, the zone's bonus level
  and the Tree Zone map in widescreen and zoomed-out views.
- Cloud scenery around the Tree Zone boss arena in widescreen and zoomed-out
  views, also available in the Scenery Editor.
- Scenery Editor: rebuilt levels open as their rebuilt map with their own
  scenery file, and a Select tool copies an area to stamp it repeatedly.
- Spin jump button: ZL / ZR on a controller (C on the keyboard) spins Mario
  while he is in the air, without holding Down. Rebind it in Settings →
  Controls; existing settings files pick up the new default.

### Changed
- With the Modern status bar, the overworld hub's lives and coins appear in
  the top-left corner instead of the bottom-right strip.

### Fixed
- Boss arenas no longer draw the preceding level's scenery with the arena's
  DX tiles and palettes. Arena scenery follows the camera's picture instead
  of the center of the window.
- Modern status bar counters render correctly in levels whose palettes use
  different colors for the original status strip's background and numbers.
- Scenery Editor: boss arenas show the screen the fight is played on, not the
  part of the arena layout beside it.
- In widescreen and zoomed-out views, the camera pans smoothly between rooms
  instead of jumping at the start and end of each screen transition.
- In widescreen and zoomed-out views, the camera no longer races past the end
  of an autoscroll level when Mario walks left of the screen's middle.
- In widescreen and zoomed-out views, distant objects no longer trigger
  proximity behavior too early, including Pumpkin Zone spikes and the
  hippo level's waiting bobbers.
- In widescreen and zoomed-out views, defeated enemies, respawning enemies
  and Wario's Castle mines use the visible area's edges, preventing enemies
  from disappearing or respawning while still in view.
- In zoomed-out views, Mario's death animation follows a continuous arc and
  fades after he falls below the visible area.

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
