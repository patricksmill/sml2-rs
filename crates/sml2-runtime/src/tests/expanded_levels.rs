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

/// A session whose assets carry a 16x4 rebuild of `level_id` that moves
/// screens x5-7 of row 1 into the new fourth row.
fn session_with_rebuild(level_id: u8) -> crate::session::Session {
    let mut assets = SessionAssets::load().unwrap();
    let level = assets.level(level_id).unwrap();
    let mut r = recipe(level_id, [16, 4]);
    r.sources.push(layout_fingerprint(&level.layout));
    r.moves.push(RegionMove {
        from: [0x50, 0x10],
        size: [0x30, 0x10],
        to: [0x50, 0x30],
    });
    assets.recipes.push(r);
    crate::session::Session::new(assets)
}

fn level_world(session: &crate::session::Session) -> World {
    let crate::session::SessionState::InLevel(ctx) = &session.state else {
        panic!("in level")
    };
    assert_eq!(
        [ctx.sim.world, ctx.cam.world, ctx.obj.world],
        [ctx.layout_live.world; 3]
    );
    ctx.layout_live.world
}

#[test]
fn the_session_enters_a_rebuilt_level_only_with_expanded_levels_on() {
    let mut session = session_with_rebuild(2);
    assert!(!session.expanded_levels, "ROM-exact unless a host opts in");
    session.begin_level(2, 0).unwrap();
    assert_eq!(level_world(&session), World::ROM);
    session.expanded_levels = true;
    session.begin_level(2, 0).unwrap();
    assert_eq!(level_world(&session), World::expanded(16, 4).unwrap());
}

#[test]
fn a_recipe_that_cannot_apply_loads_the_original_layout() {
    let mut session = session_with_rebuild(2);
    session.assets.recipes[0].moves[0].to = [0x50, 0x38];
    session.expanded_levels = true;
    session.begin_level(2, 0).unwrap();
    assert_eq!(level_world(&session), World::ROM);
}

#[test]
fn demos_keep_the_rom_layouts() {
    let demo_level = sml2_core::front_end::demo::DEMO_LEVELS[0];
    let mut session = session_with_rebuild(demo_level);
    session.expanded_levels = true;
    assert!(session.start_demo(0));
    let crate::session::SessionState::Demo(demo) = &session.state else {
        panic!("demo")
    };
    assert_eq!(demo.ctx.layout_live.world, World::ROM);
}

/// `$25AF` swaps the ROM-sized arena layout into a rebuilt level: the sims
/// return to the ROM world and the camera to the tables the arena expects.
#[test]
fn an_arena_reload_returns_a_rebuilt_level_to_the_rom_world() {
    let mut session = session_with_rebuild(8);
    session.expanded_levels = true;
    session.begin_level(8, 0).unwrap();
    assert_eq!(level_world(&session), World::expanded(16, 4).unwrap());
    let crate::session::SessionState::InLevel(ctx) = &mut session.state else {
        panic!("in level")
    };
    ctx.load_arena().unwrap();
    let rom = session.assets.level(8).unwrap();
    assert_eq!(level_world(&session), World::ROM);
    let crate::session::SessionState::InLevel(ctx) = &session.state else {
        panic!("in level")
    };
    assert_eq!(Some(&ctx.cam.edge_map), rom.edge_map.as_ref());
    assert_eq!(Some(&ctx.cam.scene_map), rom.scene_map.as_ref());
}

/// The ROM's 12-bit wrap would carry a camera left of 0 to screen 15; the
/// rebuilt level's own left locks hold it at the edge instead. Mario starts
/// against that edge, so holding left pushes on it every frame.
#[test]
fn walking_left_in_a_rebuilt_level_stops_at_the_world_edge() {
    use crate::session::FrameInput;
    let mut session = session_with_rebuild(2);
    session.expanded_levels = true;
    session.begin_level(2, 0).unwrap();
    let held = |right: bool| FrameInput {
        keys: sml2_core::player_sim::PlayerKeys {
            right,
            left: !right,
            ..Default::default()
        },
        div: 0,
    };
    let at = |session: &crate::session::Session| {
        let crate::session::SessionState::InLevel(ctx) = &session.state else {
            panic!("left the level")
        };
        (ctx.sim.x, ctx.cam.x)
    };
    let start = at(&session);
    for _ in 0..120 {
        session.advance(held(true));
    }
    assert!(
        at(&session).0 > start.0 + 0x40,
        "Mario walks right: {:04X?}",
        at(&session)
    );
    for _ in 0..600 {
        session.advance(held(false));
        let (x, cam) = at(&session);
        assert!(
            x < 0x1000 && cam < 0x1000,
            "wrapped to {x:#06X} / {cam:#06X}"
        );
    }
    assert_eq!(
        at(&session).1,
        start.1,
        "the camera parks on the left lock again"
    );
}

#[test]
fn bundled_recipes_name_their_layouts_and_apply_to_them() {
    let assets = SessionAssets::load().unwrap();
    for recipe in &assets.recipes {
        let level = assets.level(recipe.level_id).unwrap();
        if recipe.sources.contains(&layout_fingerprint(&level.layout)) {
            level.expanded(recipe).map_err(|e| e.to_string()).unwrap();
        }
    }
}
