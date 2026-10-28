# Approved scenery

This directory contains scenery selected for inclusion in port releases.
It starts empty. Local editor drafts are never bundled automatically.

1. In the companion Scenery Editor, select a level and choose **Export level…**.
2. Export into this directory as `scenery-NN.json`, where `NN` is the zero-based
   level ID (`00` through `31`). The existing `{"paints":[...]}` format is retained.
3. Add the matching entry to `LEVELS` in
   `crates/sml2-runtime/src/content/bundled_scenery.rs`:

   ```rust
   (0, include_str!("../../../../assets/scenery/scenery-00.json")),
   ```

4. Review the exported scenery in native and DX play, then run the runtime,
   desktop, editor, and core tests and the frozen replay gate.
5. Commit the export and registry entry together. The next game release embeds
   the approved scenery; players receive it by updating the port.

Only authored coordinates and metatile references belong here. Do not add ROMs,
extracted graphics, player saves, or personal settings. Scenery changes rendering
only; collision, actors, and level layouts remain the original simulation.

Rebuilt (expanded) levels export as `scenery-NN-expanded.json`, in the rebuilt
map's cells, and register in `EXPANDED` in the same file. The level must have a
bundled recipe (`assets/expanded/`). The game uses that scenery when it loads the
rebuilt level, and the plain `scenery-NN.json` when it loads the original:

```rust
(2, include_str!("../../../../assets/scenery/scenery-02-expanded.json")),
```

Scene surroundings (zone maps, minigames, fixed screens, boss arenas, the
ending) export as names such as `scenery-map-09.json` and register in `BUNDLED`
in `crates/sml2-runtime/src/scene_scenery.rs`:

```rust
(SceneTarget::Map(9), include_str!("../../../assets/scenery/scenery-map-09.json")),
```

Both the desktop game and CLI load these registries through `SessionAssets`.
Malformed files, repeated level registrations, and invalid metatile references
are startup errors. Gameplay never reopens scenery files.
