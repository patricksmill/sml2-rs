# Approved scenery

This directory contains scenery selected for inclusion in port releases.
It starts empty. Local editor drafts are never bundled automatically.

1. In the companion Scenery Editor, select a level and choose **Export level…**.
2. Export into this directory as `scenery-NN.json`, where `NN` is the zero-based
   level ID (`00` through `31`). Files use the grouped `sml2-scenery/2` format:
   each line is a rectangle tiled with a small pattern of metatile ids, such as
   `{"at":[-16,0],"size":[17,27],"tiles":[[34,35]]}`. Files in the older
   per-cell `{"paints":[...]}` format still load.
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

Scene backgrounds and surroundings (zone maps, minigames, fixed screens, boss
arenas, the ending) export as names such as `scenery-map-09.json` and register
in `BUNDLED` in `crates/sml2-runtime/src/scene_scenery.rs`. Paints within the
original 10×9 picture replace background pixels beneath actors and the HUD;
paints outside it decorate the scene's surroundings:

```rust
(SceneTarget::Map(9), include_str!("../../../assets/scenery/scenery-map-09.json")),
```

Both the desktop game and CLI load these registries through `SessionAssets`.
Malformed files, repeated level registrations, and invalid metatile references
are startup errors. Gameplay never reopens scenery files.

## Macro Zone live review

Macro Zone 1–4 (`scenery-20.json` through `scenery-23.json`), its map
(`scenery-map-03.json`), and its boss arena (`scenery-arena-23-04.json`) are
registered together for in-game visual review. These are snapshots of authored
drafts; further editor changes need a fresh export and rebuild.

In the desktop game, press **F1**, then **Macro** to enter its map. Use the
level picker and **Enter level** to jump to levels 20–23. Play level 23 through
to its boss to review the arena in the normal transition. **F2** pauses and
**F3** advances one tick for inspecting motion or palette transitions.

Review the complete levels, pipe rooms, map, and boss arena in enlarged native
and DX views before treating the scenery as visually approved. Successful
loading and a few rendered frames alone do not establish complete coverage.
