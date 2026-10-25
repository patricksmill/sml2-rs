# Windows builds

The **Windows build** GitHub Actions workflow builds Windows x64 MSVC release
executables on every push and pull request. It can also be started with **Run
workflow** from the repository's Actions tab after the workflow reaches the
default branch. Builds use stable Rust and the committed Cargo.lock.

## Download and run

1. Open the repository's **Actions → Windows build** page.
2. Select a successful run and download **sml2-windows-x64** under **Artifacts**
   (GitHub sign-in is required). Downloads expire after 14 days.
3. Unzip the archive. It contains `sml2.exe`, `sml2-runtime.exe`,
   `sml2-extract.exe`, and documentation.
4. Copy your existing `extracted` game-data directory beside the executables.
5. Open PowerShell in that directory and run:

   ```powershell
   .\sml2.exe --assets .\extracted --no-audio
   ```

You can also point `--assets` at an existing extracted directory elsewhere.
The optional **sml2-scenery-editor-windows-x64** artifact contains the separate
editor executable and instructions; the game download excludes it.
The game and scenery editor open without a console window and include the
application icon in the executable for Explorer, shortcuts, and the taskbar.
Startup failures appear in an error dialog. The included `Desktop.md`
(or `docs/desktop.md` in the source repository) lists startup options, controls,
and settings. The runtime and extractor remain command-line tools.

## Game data and current limitations

Neither archive contains ROMs or extracted game assets. The game and editor
share the verified original-ROM importer and optional DX graphics cache.
Open Settings → Game in the game, or Game data in the editor, to import your
supported original ROM. Authored scenery data approved for release is embedded
in the executables.

As an alternative, `sml2-extract` can produce data from a local byte-identical
`\sml2decomp` checkout and base ROM. It expects that checkout at
`../sml2decomp`, or at the path set by `SML2_DECOMP_ROOT`:

```powershell
$env:SML2_DECOMP_ROOT = "C:\path\to\sml2decomp"
.\sml2-extract.exe
```

Extraction writes `extracted` in the current directory. Copying an existing
extraction from another machine works too. Rust is unnecessary when running
the downloaded executables.

The live audio backend currently targets PulseAudio/PipeWire; native Windows
audio is pending. Use `--no-audio` for Windows testing. GPU presentation,
controller hotplug, and gameplay still need hands-on Windows validation.

CI runs the desktop policy tests that do not need game data, builds the game,
runtime, extractor, and optional editor, verifies the GUI subsystem and all
embedded icon sizes, and checks both desktop and editor `--help` startup paths.
It does not exercise a graphical window or the ROM-dependent replay/parity suite.
The music bank is loaded from `extracted/audio/music_bank04.bin` at runtime;
production executables do not embed it or require it during compilation.

## Build locally on Windows

Install stable Rust using rustup with the MSVC toolchain, and the Visual Studio
Build Tools C++ workload and Windows SDK. From the repository root:

```powershell
cargo test --locked -p sml2-desktop --no-default-features --lib
cargo build --locked --release --target x86_64-pc-windows-msvc -p sml2-desktop -p sml2-runtime -p sml2-extract -p sml2-scenery-editor
```

Executables are in `target\x86_64-pc-windows-msvc\release`.
