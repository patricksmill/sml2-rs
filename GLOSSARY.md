# Six Golden Coins

A native Rust desktop edition of Super Mario Land 2: 6 Golden Coins. Players supply their own cartridge dump; the project ships only its own code and art.

## Distribution

**Official release**:
A tagged GitHub Release of this repository, built by the release workflow, with its packaged assets and `SHA256SUMS.txt`.
_Avoid_: Build, nightly, version (when meaning the published artifact)

**Release asset**:
One packaged file of an official release for one platform, named `SixGoldenCoins-vX.Y.Z-<platform>.<ext>`.
_Avoid_: Binary, download, executable

**Source build**:
A copy of the game compiled locally from the repository rather than taken from an official release; it never checks for or installs updates.
_Avoid_: Dev build, custom build

**Fork build**:
A copy of the game built from a modified copy of the repository; bug reports from it are out of scope.
_Avoid_: Mod, unofficial release

**ROM import**:
The player's one-time act of selecting their own cartridge dump so the game can derive its data locally.
_Avoid_: ROM install, ROM loading, extraction (extraction is the developer-side tool)

## Updates

**Update check**:
A query for the newest official release, made at most once a day or on demand from Help.
_Avoid_: Ping, version poll

**Self-update**:
Replacing the running copy of the game with a newer official release from inside the game, without admin rights.
_Avoid_: Auto-update, patching, upgrade

**Skipped version**:
An official release the player chose to ignore; the update check stays silent about it until a newer one appears.
_Avoid_: Dismissed update, ignored release

## Players

**Single player**:
The default way to play: one person, with no Player 2. Every launch starts here.
_Avoid_: Solo mode, 1P

**Player 1**:
The player whose input device first pressed Start (or Enter) on the title screen, or who opened the map when there was no title screen. Player 1 alone moves the overworld cursor.
_Avoid_: P1, host, main player

**Player 2**:
A second player who has joined and plays alongside Player 1 until they drop out or the game quits.
_Avoid_: P2, Luigi (when meaning the player), guest

**Join**:
Player 2 entering the game by pressing Start (or Enter) on an input device that is not Player 1's, allowed only on the title screen or an overworld map.
_Avoid_: Co-op mode, multiplayer toggle, drop in

**Drop out**:
A player leaving from the pause menu their own input device opened, allowed anywhere. If Player 1 drops out, Player 2 becomes Player 1.
_Avoid_: Remove player, kick, leave game

**Input device**:
The keyboard or one gamepad; each belongs to at most one player.
_Avoid_: Controller (when the keyboard is meant too)
