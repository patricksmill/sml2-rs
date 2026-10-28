//! Expanded-level recipes applied to the extracted levels.

use crate::session::{LevelContext, SessionAssets};
use sml2_core::level_recipe::{layout_fingerprint, LevelRecipe, RegionMove, RECIPE_SCHEMA};
use sml2_core::world::World;

fn recipe(level_id: u8, world: [u16; 2]) -> LevelRecipe {
    LevelRecipe {
        schema: RECIPE_SCHEMA.into(),
        level_id,
        sources: Vec::new(),
        world,
        fill: 0,
        moves: Vec::new(),
        paint: Vec::new(),
        screens: Vec::new(),
        spawns: Vec::new(),
        start: None,
        secret_start: None,
    }
}

#[test]
fn an_empty_recipe_leaves_every_level_as_extracted() {
    let assets = SessionAssets::load().unwrap();
    for id in 0..32 {
        let Ok(level) = assets.level(id) else {
            continue;
        };
        let out = level.expanded(&recipe(id, [16, 3])).unwrap();
        assert_eq!(out.layout.world, World::expanded(16, 3).unwrap());
        assert_eq!(out.layout.cells, level.layout.cells, "level {id:02X}");
        let table = |t: &Option<Vec<u8>>| t.clone().unwrap_or_else(|| vec![0; 48]);
        assert_eq!(out.edge_map, Some(table(&level.edge_map)), "level {id:02X}");
        assert_eq!(
            out.scene_map,
            Some(table(&level.scene_map)),
            "level {id:02X}"
        );
        assert_eq!(out.manifest.spawns, level.manifest.spawns, "level {id:02X}");
        assert_eq!(out.manifest.header, level.manifest.header, "level {id:02X}");
        assert_eq!(out.manifest.secret_entry, level.manifest.secret_entry);
        assert_eq!(out.scenery, level.scenery, "level {id:02X}");
    }
}

/// Tree Zone 2's pipe section (screens x5-7 of row 1) moved into a fourth
/// screen row: the sims all see the larger world and its tables.
#[test]
fn a_rebuilt_tree_zone_2_loads_into_every_sim_at_its_size() {
    let assets = SessionAssets::load().unwrap();
    let level = assets.level(2).unwrap();
    let mut r = recipe(2, [16, 4]);
    r.sources.push(layout_fingerprint(&level.layout));
    r.moves.push(RegionMove {
        from: [0x50, 0x10],
        size: [0x30, 0x10],
        to: [0x50, 0x30],
    });
    let out = level.expanded(&r).unwrap();
    let world = out.layout.world;
    assert_eq!((world.cells_wide(), world.cells_high()), (256, 64));
    assert_eq!(out.layout.get(0x60, 0x38), level.layout.get(0x60, 0x18));
    assert_eq!(out.layout.get(0x60, 0x18), Some(0), "vacated");

    let records = &out.manifest.spawns.records;
    assert_eq!(records.len(), level.manifest.spawns.records.len());
    assert!(
        records.windows(2).all(|w| w[0].x <= w[1].x),
        "the stream stays X-sorted"
    );
    let moved = level
        .manifest
        .spawns
        .records
        .iter()
        .filter(|s| (0x500..0x800).contains(&s.x) && (0x100..0x200).contains(&s.y));
    for s in moved {
        assert!(records
            .iter()
            .any(|o| (o.x, o.y, o.anim_index) == (s.x, s.y + 0x200, s.anim_index)));
    }

    let ctx = LevelContext::from_content(
        2,
        &out,
        0,
        5,
        0,
        0,
        0,
        0,
        assets.scripts.clone(),
        assets.tilesets.clone(),
        None,
        assets.anim_rows.clone(),
        assets.hud.clone(),
        assets.object_tables.clone(),
    );
    assert_eq!([ctx.sim.world, ctx.cam.world, ctx.obj.world], [world; 3]);
    assert_eq!((ctx.cam.edge_map.len(), ctx.cam.scene_map.len()), (64, 64));
    let rom = |t: &Option<Vec<u8>>| t.as_ref().unwrap()[16 + 5];
    assert_eq!(ctx.cam.scene_map[3 * 16 + 5], rom(&level.scene_map));
    assert_eq!(ctx.cam.edge_map[3 * 16 + 5], rom(&level.edge_map));

    r.sources = vec!["0".repeat(16)];
    assert!(level.expanded(&r).err().unwrap().contains("written for"));
    assert!(assets
        .level(3)
        .unwrap()
        .expanded(&r)
        .err()
        .unwrap()
        .contains("applied to level 03"));
}
