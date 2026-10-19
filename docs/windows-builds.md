# Windows builds

The **Windows build** GitHub Actions workflow builds Windows x64 MSVC release
executables on every push and pull request. It can also be started with **Run
workflow** from the repository's Actions tab after the workflow reaches the
default branch. Builds use stable Rust and the committed Cargo.lock.

## Download and run

1. Open the repository's **Actions → Windows build** page.
2. Select a successful run and download **sml2-windows-x64** under **Artifacts**
   (GitHub sign-in is required). Downloads expire after 14 days.
3. Unzip the archive. It contains `sml2-desktop.exe`, `sml2-runtime.exe`,
   `sml2-extract.exe`, and documentation.
4. Copy your existing `extracted` game-data directory beside the executables.
5. Open PowerShell in that directory and run:

   ```powershell
   .\sml2-desktop.exe --assets .\extracted --no-audio
   ```

You can also point `--assets` at an existing extracted directory elsewhere.
Run `.\sml2-desktop.exe --help` to see startup options. The included `Desktop.md`
(or `docs/desktop.md` in the source repository) lists controls and settings.

## Game data and current limitations

The archive contains no ROM or extracted game assets. Direct ROM import is
planned; this version requires data produced by `sml2-extract` from your local
byte-identical `sml2decomp` checkout and base ROM. The extractor expects that
checkout at `../sml2decomp`, or at the path set by `SML2_DECOMP_ROOT`:

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

CI runs the desktop policy tests that do not need game data, builds all three
executables, and checks the desktop CLI's `--help` startup path. It does not
exercise a graphical window or the ROM-dependent replay/parity suite.
The music bank is loaded from `extracted/audio/music_bank04.bin` at runtime;
production executables do not embed it or require it during compilation.

## Build locally on Windows

Install stable Rust using rustup with the MSVC toolchain, and the Visual Studio
Build Tools C++ workload and Windows SDK. From the repository root:

```powershell
cargo test --locked -p sml2-desktop --no-default-features --lib
cargo build --locked --release --target x86_64-pc-windows-msvc -p sml2-desktop -p sml2-runtime -p sml2-extract
```

Executables are in `target\x86_64-pc-windows-msvc\release`.
