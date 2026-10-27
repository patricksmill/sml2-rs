# Changelog

All notable changes to Six Golden Coins will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.1.0] - 2026-10-24

### Added
- **Native Desktop App (`sml2`)**: High-performance, cross-platform port written in Rust with fixed 60 Hz simulation and variable display refresh rates.
- **In-App ROM Verification & Importer**: Use the file picker on first launch to verify and extract assets directly from your legally-owned Super Mario Land 2 ROM (`.gb` / `.gbc`). No pre-extracted assets or manual extraction required.
- **SML2DX Support**: Import DX graphics, palettes, level layouts and Luigi's movement from a user-supplied SML2DX v1.8.1 ROM or IPS patch, and play as Mario or Luigi.
- **Custom Display Filters**: Grayscale, DMG green, Pocket, GBC auto, 32 Super Game Boy (SGB) hardware presets, and Amber phosphor.
- **Controller & Input Configuration**: Full gamepad support via gilrs with runtime rebinding, customizable keyboard mapping, hotplugging, and fullscreen toggle.
- **Packaging Pipeline**: Standalone distribution binaries for Windows x64, Linux x86_64, and macOS Apple Silicon (arm64).
- **Tools Bundle**: Accompanying tools package containing headless simulation runner (`sml2-runtime`), offline extractor (`sml2-extract`), and optional scenery editor (`sml2-scenery-editor`).

### Known Limitations
- The game mechanics port is progressing in parallel. Zone bosses, final Wario encounters, and select late-game route combinations are actively being finalized.
- Live audio uses WASAPI on Windows and PulseAudio / PipeWire on Linux. macOS has no native backend and runs silently unless PulseAudio's `pacat` is installed.
