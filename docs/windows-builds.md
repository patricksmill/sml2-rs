# Windows builds

Releases/workflow artifacts provide the game and a separate tools bundle; no
ROMs or extracted graphics are included. Download from GitHub Releases when
published, or a successful Windows workflow run under Actions. Releases put the
optional scenery editor in the tools bundle; the Windows workflow uploads it as
its own artifact.

## Run

Extract the archive and open `sml2.exe`. Use Settings -> Game to import your own
original UE v1.0 ROM and optionally DX. The game/editor share installed assets.
Audio plays through WASAPI; `--no-audio` disables it. See the tools package's
`Desktop.md` or the [desktop guide](desktop.md) for controls/settings.

Tools can use an existing extracted tree:

```powershell
.\sml2.exe --assets .\extracted --no-audio
```

For developer extraction from a byte-identical disassembly and base ROM:

```powershell
$env:SML2_DECOMP_ROOT = "C:\path\to\sml2decomp"
.\sml2-extract.exe
```

Extraction writes `extracted` in the current directory. Downloaded executables
do not need Rust. Game/editor launch without a console; startup failures use a
dialog. Runtime/extractor remain command-line tools. The audio ROM bank loads
from assets rather than being embedded in the executable.

## Build

Install stable Rust's MSVC toolchain, Visual Studio C++ Build Tools and Windows SDK:

```powershell
cargo test --locked -p sml2-desktop --no-default-features --lib
cargo build --locked --release --target x86_64-pc-windows-msvc -p sml2-desktop -p sml2-runtime -p sml2-extract -p sml2-scenery-editor
```

Executables are under `target\x86_64-pc-windows-msvc\release`. CI builds and
checks CLI help/policy tests, not graphical rendering or the ROM-dependent parity
suite. GPU presentation, hotplug and gameplay require hands-on Windows testing.
