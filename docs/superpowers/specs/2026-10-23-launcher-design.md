# Launcher — design

Status: approved. Implementation plan: `docs/superpowers/plans/2026-10-23-launcher.md`.

## Goal

A normal launch opens a library. The user imports their own Super Mario Land 2
ROM and plays that install. With nothing else added, Play is the original game.
SML2DX is an optional layer on the same screen. Level packs and a level editor
are visible there and do nothing in this version.

The library is the cold start only. Play enters the title. Quit to title, saves,
and the rest of the session stay as they are. Changing the ROM or DX applies on
the next launch.

## Decisions

- One desktop app. The library is not a second process.
- Play loads one published vanilla directory through the existing
  `SessionAssets` loader. The launcher does not discover `extracted/`.
- `--play`, `--file-select`, `--assets`, and the smoke flags skip the library
  and load the explicit path or the developer tree, as they do now. `--title`
  is a normal launch and opens the library. A missing developer tree on a
  skipped launch reports the existing error.
- The vanilla importer is a ROM-only entry point in `sml2-extract`. It uses an
  offset table compiled into that crate. `cargo run -p sml2-extract` remains
  the developer command that checks those ranges against the disassembly. The
  user's machine does not need that tree.
- The allow-list is data: revision id, display name, exact length, SHA-256, and
  offset-table id. The first entry is revision `ue-v1.0`, display name
  "Super Mario Land 2", length 512 KiB, offset table `ue-v1.0`. Its digest is
  the SHA-256 of the developer `baserom.gb`. That ROM is the `MARIOLAND2` image
  with title checksum `$C9`. Acceptance is the length and the digest. A later
  audited revision is another row, not a new control.
- No ROM, DX ROM, IPS or other patch, extracted tree, imported graphic, or
  reference render is committed. The repo holds the digest and the offset
  table. Gameplay tables already transcribed into the engine stay there.
- The existing scenery editor stays on the pause menu. This spec does not
  move it onto the library's Level editor row.

## Library

The library is drawn with the app's own text. Game tiles are not loaded until
Play. The rows, top to bottom:

| Row | Before a vanilla install | After one |
|---|---|---|
| ROM | Import ROM…, status Required | Allow-list display name, plus Replace ROM… |
| DX | Visible, inactive, "Add a ROM first" | Import SML2DX ROM… until installed. Then an On/Off switch for the next Play, plus Replace |
| Level packs | Visible, inactive, "Not in this version" | Same |
| Level editor | Visible, inactive, "Not in this version" | Same |
| Play | Inactive | Active. The label is Play with DX when that switch is on |
| Exit | Closes the app | Same |

Enter activates the selected row. Esc exits.

While a vanilla install is being checked, the ROM row says "Checking install…"
and Play stays off. While an import is running, that row shows progress and
Cancel, and the other import action is inactive. The file picker is the system
dialog, then the in-app browser already used for DX when the dialog returns no
file.

A successful DX import turns the switch on. A failed or cancelled DX import
leaves the previous DX installation and switch as they were. Turning the switch
off leaves the DX files in place, and the next Play is vanilla. The switch is
the existing `dx_mode` setting.

Settings → Mods keeps widescreen, camera zoom, and the character choice. The DX
import and the DX on/off switch leave that page. A running game keeps the
choice from this launch. The character door still works when the session
started with DX on.

## Install

The importer reads at most one byte past the longest allow-list length. A
longer file is rejected before the rest of it is read. A matching entry is
extracted with that entry's offset table into the directory layout
`SessionAssets` already loads, including `cartridge_header.bin`. Reference
PNGs and `renders/` are omitted. The ROM is not copied into the cache or the
repo.

Extraction writes a staging directory of regular files. Publish loads it with
`SessionAssets::load_from` and requires all 32 levels, ids 0 through 31, to
load.
`load_from` returns success while recording a per-level error, so the publish
gate checks those level results itself. The staging directory then receives
`manifest.json`:

- revision id
- ROM SHA-256
- importer version `1`
- schema `1`

Renaming the staging directory to the vanilla cache directory is the commit.
That directory is `ProjectDirs` cache dir `sml2-rs` joined with `vanilla`, the
sibling of the existing `dx` cache. The previous install is renamed aside
first. Settings then store the revision id and digest. The optional `vanilla`
field defaults to absent, and the settings version stays 3 so an existing file
still loads. If the settings write fails, the previous directory is restored
and the new install is removed.

After a committed publish, the base fingerprint is the hash DX already computes
for a directory. When it differs from the fingerprint of the stored DX
installation, that installation's cache directory is deleted, `dx_installation`
is cleared, and `dx_mode` is set false. A failure to delete the DX directory
does not roll back the vanilla install. The switch stays off, and the DX row
says the old install could not be removed. Re-importing a ROM that produces the
same fingerprint keeps DX.

## Launch and Play

No stored vanilla identity: the ROM row says Required and Play stays off.

A stored identity starts a background check. The check accepts the install only
when `manifest.json` matches the stored revision and digest and this importer
version and schema, and `SessionAssets` loads all 32 levels. A manifest that
disagrees with the stored identity fails the check. Play turns on after that check.
A failed check keeps the files, shows the error on the ROM row, and offers
Replace ROM…. An older importer version or schema uses the message to import
the ROM again.

Play loads the published directory, boots the title, and applies DX only when
the switch is on and the DX installation's fingerprint still matches. If the DX
installation is missing or does not match, the switch is stored off, Play is
vanilla when the vanilla check passed, and the DX row asks for the ROM again.
The DX files stay until a successful DX import replaces them, or a vanilla
publish with a different fingerprint removes them. Saves stay in the user data
directory.

DX import is the current importer. Its base assets are the published vanilla
directory. Its cache stays `cache/dx`.

## Errors

A wrong length, a digest that is not on the list, a failed extract, or any
level failing to load deletes the staging directory and leaves the previous
vanilla install and DX install in place. Cancel does the same. The live
directory is replaced only after validation.

## Tests

Rejection, rollback, cancel, and the fingerprint rule use synthetic buffers and
temporary directories. No test commits a ROM, a patch, or extracted bytes.

Library tests cover Required, a finished install with DX off and on, inactive
pack and editor rows, and Play staying off until the background check passes.
`--assets`, `--play`, `--file-select`, and the smoke flags do not open the
library. `--title` does.

One ignored local test runs the ROM-only importer against a developer
`baserom.gb` and compares its relative paths and bytes with the disassembly
extractor, excluding `renders/` and any `.png`. The existing DX revision test
stays local and ignored.

## Out of scope

Level-pack format, loading, and authoring. The level editor. Moving or
replacing the scenery editor. Returning to the library from the pause menu.
Accepting any ROM other than the first allow-list entry. Shipping a
pre-extracted game.
