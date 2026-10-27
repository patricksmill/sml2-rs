# Optional Scenery Editor

The editor is a separate companion app. The normal game includes approved
scenery automatically and has no editor menus. Download
**sml2-scenery-editor-windows-x64** separately from a successful Windows build,
or run from source:

```sh
cargo run -p sml2-scenery-editor
```

It shares the game's imported original ROM and installed DX graphics. If no
usable game data is installed, choose **Game data… → Import original ROM…**.
The same verified ROM importer and cache are used by both apps. You can also
use existing extracted assets:

```sh
cargo run -p sml2-scenery-editor -- --assets ./extracted --no-audio
```

`--config FILE` selects the game settings file and the directory for drafts.
The editor reads the game's display and control preferences but only updates
installation identities when importing graphics. Its playtests never open or
write your game save.

## Painting and playtesting

Select a level or a **Scene surroundings** target. Levels allow cosmetic
painting inside and outside their map. Scene targets cover zone maps,
minigames, fixed screens, boss arenas, and the ending. Their magenta boundary
marks the protected original 160×144 picture: only its surroundings can be
painted. The scene tile tray contains background swatches, without actors.

| Action | Control |
|---|---|
| Brush / Eraser / Pick / Rectangle | B / E / I / R |
| Paint / temporary eraser | Left-drag / right-drag |
| Pan | Middle-drag or Space + left-drag |
| Scroll / horizontal scroll | Wheel / Shift-wheel |
| Zoom | Ctrl/Cmd-wheel, + / − |
| Grid | G |
| Undo / Redo | Ctrl/Cmd+Z / Ctrl/Cmd+Shift+Z |
| Playtest / Return | Enter / Escape |
| Cancel unfinished gesture | Escape |

The canvas has independent pan and 1×–8× zoom. Game display settings determine
the playtest view; an enlarged view is needed to see scene surroundings.
The tile tray follows the selected display filter or DX graphics. Visual
changes never modify collision, actors, terrain layout, or routes.

Gestures and undo/redo autosave atomically. The latest 100 undo steps and each
target's viewport remain available during the app session. A failed save keeps
the edits; use **Retry**. An unreadable or invalid draft locks editing until
you repair it and choose **Reload file**.

## Drafts and release exports

Existing level drafts remain `scenery-NN.json` beside the game's
`settings.json`, with zero-based level IDs. Scene drafts use names such as
`scenery-map-09.json` or `scenery-arena-07-00.json`. Neither kind is loaded by
the normal game. A target without a draft starts from its bundled scenery;
an explicitly empty draft overrides that starting point in the editor.

To ship your changes:

1. Select a target and choose **Export level…**. Export finishes the current
   gesture and takes a snapshot of that target.
2. Choose a destination using **Browse…**, or type its path and choose
   **Export**. Export failures leave the draft and undo history intact.
3. Save approved exports in the repository's `assets/scenery/` directory and
   register them (levels and scenes have separate registries) as described in
   [Approved scenery](../assets/scenery/README.md).
4. Review native and DX play, run the documented checks, and commit the export
   and registry entry together.

The next port release embeds those files. Players receive the improvements
when updating the game; they do not need the editor or a scenery installer.
The initial registries are empty. Existing drafts are never published
automatically. Exports contain authored coordinates and tile references,
without ROM graphics, saves, or settings.

Developer contract: validated overlays reject duplicate/out-of-range cells and
unknown metatiles. Both native and DX rendering preserve the original playfield,
live tile mutations and status bar; surroundings sample authored tiles or edge
pixels. Tests live in core scenery, runtime render and editor modules. The old
centered-view draft spec is superseded by the current upward level-view bias
in [DESIGN.md](../DESIGN.md#viewports-and-composition).
