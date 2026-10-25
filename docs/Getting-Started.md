# Getting Started with Super Mario Land 2 (sml2-rs)

Welcome to the native port of Super Mario Land 2!

## Requirements

1. **Operating System**:
   - Windows 10/11 (x64)
   - Linux (x86_64, glibc-based distributions like Ubuntu, Debian, Fedora, Arch)
   - macOS 12+ (Apple Silicon arm64)

2. **Game Data**:
   - You need a copy of the original Game Boy ROM: **Super Mario Land 2 - 6 Golden Coins (USA/Europe) v1.0** (standard `.gb` or `.gbc` file).
   - *Note: This project does not contain or distribute copyrighted ROMs or game assets.*

---

## First Run & Setup

1. **Launch the game**:
   - On Windows: Double-click `sml2.exe`
   - On Linux: Run `./sml2` (make sure it's executable: `chmod +x sml2`)
   - On macOS: Right-click `sml2` and select **Open** (to bypass the unsigned binary prompt)

2. **Import your ROM**:
   - On first launch, the game opens the **ROM Setup** screen.
   - Simply **drag and drop** your `.gb` / `.gbc` file onto the window, or click the file picker button to select it.
   - The game will verify the file checksum and import the game data in seconds.

3. **Enjoy!** Once verified, click **Play** to start.

---

## Default Controls

| Action | Keyboard | Gamepad (Xbox / Standard) |
| :--- | :--- | :--- |
| **Move / Aim** | Arrow Keys / WASD | D-Pad / Left Stick |
| **Jump (A)** | Z / Space | South Button (A / Cross) |
| **Run / Fire (B)** | X / Left Shift | West Button (X / Square) |
| **Pause / Menu** | Escape | Start Button |
| **Fullscreen** | F11 | — |
| **Zoom (+ / -)** | `+` / `-` | — |
| **Toolbar** | F10 | — |

*All controls can be rebound at any time under **Settings → Controls**.*

---

## Advanced Options

Run `sml2 --help` from your terminal or command prompt to view command-line flags, including:
- `--filter <NAME>`: Choose DMG green, grayscale, pocket, gbc, or sgb presets.
- `--no-audio`: Disable sound output.
- `--play`: Skip launcher and jump directly to overworld.
- `--fullscreen`: Launch in fullscreen mode.
