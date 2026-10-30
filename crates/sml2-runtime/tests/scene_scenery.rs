use sml2_core::character::Character::Mario;
use sml2_dx::{LevelGraphics, PlayerGraphics, Presentation};
use sml2_runtime::{
    content::{ExtractedContent, SessionAssets},
    session::{Session, SessionState},
};
use std::path::PathBuf;

fn session() -> Session {
    let source =
        ExtractedContent::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"));
    Session::new(SessionAssets::load_from(&source).unwrap())
}

/// Every level's BG indices are literal red; OBJ zero is transparent
/// irrespective of RGB, and every tile is blank.
fn synthetic_bundle() -> Presentation {
    let level = LevelGraphics {
        background: [vec![0; 0x1000], vec![0; 0x1000]],
        tile_attributes: vec![0; 256],
        bg_palettes: [0x1f, 0].repeat(32),
        obj_palettes: [0xe0, 3].repeat(32),
        hud_attributes: vec![0; 20],
        theme: 0,
        object_table: 0,
        metatiles: sml2_core::level::MetatileTable::from_bytes(&[0; 512]).unwrap(),
        layout: vec![0; 0x3000],
        spawns: Vec::new(),
    };
    let player = PlayerGraphics {
        normal: vec![0; 0x800],
        fire: vec![0; 0x800],
        bank1: vec![0; 0x800],
        mappings: vec![vec![]; 242],
    };
    Presentation {
        movement: std::array::from_fn(|_| {
            std::sync::Arc::new(sml2_core::character::MovementCurves {
                horizontal: sml2_core::player_sim::SPEED_H,
                normal: sml2_core::player_sim::SPEED_V[..76].try_into().unwrap(),
                swim: sml2_core::player_sim::SPEED_V_GOO[..76].try_into().unwrap(),
                moon: sml2_core::player_sim::SPEED_V_WATER[..76]
                    .try_into()
                    .unwrap(),
            })
        }),
        levels: vec![level; 32],
        players: [[player.clone(), player.clone()], [player.clone(), player]],
        object_tables: vec![vec![vec![]; 228]],
        hud_tiles: vec![0; 20],
        animation_rows: vec![0; 0x1000],
        banks: [(0x26, {
            let mut bank = vec![0; 0x4000];
            bank[0x2100..0x21c0].copy_from_slice(&[0x1f, 0].repeat(96));
            bank
        })]
        .into_iter()
        .collect(),
        title: screen(),
        file_select: screen(),
        map: sml2_dx::MapGraphics {
            tilemaps: Default::default(),
            manifest: Default::default(),
            tiles: Default::default(),
            zone_animation_tiles: Vec::new(),
            hub_attributes: vec![0; 0x100],
            screens: vec![
                sml2_dx::MapScreen {
                    bg_palettes: [0x1f, 0].repeat(32),
                    alternate_bg_palettes: None,
                    obj_palettes: [[0xe0, 3].repeat(32), [0xe0, 3].repeat(32)],
                    attributes: Some(vec![0; 0x240]),
                    alternate_attributes: None,
                };
                15
            ],
            bell_tiles: Default::default(),
            room_tiles: Default::default(),
            sprites: sml2_dx::MapSprites {
                hub_decor: vec![vec![]; 17],
                zone_decor: vec![vec![]; 188],
                status_bar: [(0, 0, 0, 0); 9],
                room_metasprites: Default::default(),
            },
        },
        arenas: Vec::new(),
        arena_metatiles: Vec::new(),
        boss_oam_lists: Vec::new(),
    }
}

/// A blank screen whose background is palette 0 color 0.
fn screen() -> sml2_dx::Screen {
    sml2_dx::Screen {
        vram: [vec![0; 0x1800], vec![0; 0x1800]],
        tilemap: vec![0; 0x400],
        attributes: vec![0; 0x400],
        bg_palettes: [0x1f, 0].repeat(32),
        obj_palettes: [0xe0, 3].repeat(32),
    }
}

use sml2_core::scenery::{SceneryOverlay, SceneryPaint};
use sml2_runtime::scene_scenery::{SceneScenery, SceneTarget, SCENE_SWATCHES};

fn surround() -> SceneryOverlay {
    let paints = (-4..14)
        .flat_map(|y| {
            (-8..18).map(move |x| SceneryPaint {
                x,
                y,
                metatile: ((x + 8 + (y + 4) * 10) % 90) as u8,
            })
        })
        .collect();
    SceneryOverlay::try_from_paints(paints, SCENE_SWATCHES).unwrap()
}

#[test]
fn all_available_scene_borders_preserve_native_pixels_and_simulation() {
    let mut game = session();
    let targets = game.scenery_scenes();
    assert_eq!(
        targets
            .iter()
            .filter(|t| matches!(t, SceneTarget::Arena { .. }))
            .count(),
        7
    );
    for target in targets {
        game.stage_scenery_scene(target)
            .unwrap_or_else(|error| panic!("{target:?}: {error}"));
        assert_eq!(game.scene_scenery_target(), Some(target));
        let native = game.render_frame((160, 144));
        let baseline = game.render_frame((385, 217));
        let palette = game.scene_palette_frame().unwrap();
        assert_eq!(palette.len(), 160 * 144);
        let saved = game.save_record();
        let frame_n = game.frame_n;
        game.set_scene_scenery(target, surround()).unwrap();
        let painted = game.render_frame((385, 217));
        assert_eq!(native, game.render_frame((160, 144)), "{target:?}");
        let (mx, my) = game.scene_border_origin((385, 217));
        for y in 0..144 {
            let at = (my as usize + y) * 385 + mx as usize;
            assert_eq!(
                &painted[at..at + 160],
                &baseline[at..at + 160],
                "{target:?} row {y}"
            );
        }
        assert_eq!(saved, game.save_record(), "render must preserve progress");
        assert_eq!(frame_n, game.frame_n, "render must not tick");
        assert_eq!(game.active_slot, None, "authoring sessions own no pipe");
        game.set_scene_scenery(target, SceneryOverlay::empty())
            .unwrap();
    }
}

#[test]
fn border_pixels_use_background_swatches_in_native_and_dx() {
    let mut game = session();
    let mut dx = synthetic_bundle();
    for id in 0..15 {
        dx.map.screens[id].bg_palettes = [0xe0, 3].repeat(32);
    }
    for bank in [0x05, 0x0c, 0x10, 0x11, 0x1a, 0x27, 0x29] {
        dx.banks.insert(bank, vec![0; 0x4000]);
    }
    for target in [
        SceneTarget::Title,
        SceneTarget::FileSelect,
        SceneTarget::Map(6),
        SceneTarget::Map(9),
        SceneTarget::Map(13),
        SceneTarget::Arena { level: 9, gfx: 2 },
        SceneTarget::Castle,
        SceneTarget::GameOver,
        SceneTarget::Ending,
    ] {
        game.stage_scenery_scene(target).unwrap();
        let native_source = game.scene_palette_frame().unwrap();
        let dx_source = game.scene_palette_dx(&dx, Mario).unwrap();
        let original = game.render_dx_frame(&dx, Mario, (384, 216)).unwrap();
        game.set_scene_scenery(
            target,
            SceneryOverlay::try_from_paints(
                vec![
                    SceneryPaint {
                        x: -1,
                        y: 0,
                        metatile: 23,
                    },
                    SceneryPaint {
                        x: 0,
                        y: 0,
                        metatile: 23,
                    },
                ],
                90,
            )
            .unwrap(),
        )
        .unwrap();
        let native = game.render_palette_frame((384, 216));
        let colored = game.render_dx_frame(&dx, Mario, (384, 216)).unwrap();
        let (mx, my) = game.scene_border_origin((384, 216));
        let (mx, my) = (mx as usize, my as usize);
        for y in 0..16 {
            for x in 0..16 {
                let at = (my + y) * 384 + mx - 16 + x;
                let source = (32 + y) * 160 + 48 + x;
                assert_eq!(native[at], native_source[source], "{target:?} {x},{y}");
                assert_eq!(colored[at], dx_source[source], "{target:?} {x},{y}");
            }
        }
        for y in 0..144 {
            let at = (my + y) * 384 + mx;
            assert_eq!(
                &colored[at..at + 160],
                &original[at..at + 160],
                "{target:?} center row {y}"
            );
        }
        game.set_scene_scenery(target, SceneryOverlay::empty())
            .unwrap();
    }
}

#[test]
fn arena_borders_preserve_the_level_camera_region_and_relocated_hud() {
    let mut game = session();
    let target = SceneTarget::Arena { level: 9, gfx: 2 };
    game.stage_scenery_scene(target).unwrap();
    let view = (384, 216);
    let baseline = game.render_frame(view);
    let rects = game.scene_border_protected_rects(view);
    assert_eq!(rects.len(), 2);
    game.set_scene_scenery(target, surround()).unwrap();
    let painted = game.render_frame(view);
    let (mx, my) = game.scene_border_origin(view);
    for (left, top, width, height) in rects {
        for y in top + my..top + my + height {
            for x in left + mx..left + mx + width {
                if (0..view.0 as i32).contains(&x) && (0..view.1 as i32).contains(&y) {
                    let at = y as usize * view.0 + x as usize;
                    assert_eq!(painted[at], baseline[at], "protected arena pixel {x},{y}");
                }
            }
        }
    }
}

#[test]
fn swatch_backgrounds_exclude_file_select_cursor_and_tallies() {
    let mut game = session();
    game.stage_scenery_scene(SceneTarget::FileSelect).unwrap();
    let palette = game.scene_palette_frame().unwrap();
    let dx = synthetic_bundle();
    let dx_palette = game.scene_palette_dx(&dx, Mario).unwrap();
    let SessionState::FileSelect(select) = &mut game.state else {
        unreachable!()
    };
    for _ in 0..20 {
        select.step(&mut game.slots, sml2_core::front_end::keys::RIGHT, 0);
    }
    assert_eq!(palette, game.scene_palette_frame().unwrap());
    assert_eq!(dx_palette, game.scene_palette_dx(&dx, Mario).unwrap());
}

#[test]
fn scene_exports_validate_targets_swatches_and_duplicate_entries() {
    let valid = r#"{"paints":[{"x":-1,"y":0,"metatile":89}]}"#;
    let scenery = SceneScenery::from_exports(&[(SceneTarget::Map(9), valid)]).unwrap();
    assert_eq!(scenery.get(SceneTarget::Map(9)).get(-1, 0), Some(89));
    assert!(scenery.get(SceneTarget::Map(10)).is_empty());
    for exports in [
        vec![(SceneTarget::Map(15), valid)],
        vec![(SceneTarget::Arena { level: 32, gfx: 0 }, valid)],
        vec![(SceneTarget::Title, "broken")],
        vec![(
            SceneTarget::Title,
            r#"{"paints":[{"x":-1,"y":0,"metatile":90}]}"#,
        )],
        vec![(SceneTarget::Title, valid), (SceneTarget::Title, valid)],
    ] {
        assert!(SceneScenery::from_exports(&exports).is_err());
    }
    assert!(SceneScenery::bundled()
        .unwrap()
        .get(SceneTarget::Map(9))
        .is_empty());
}

/// An arena door's scene runs inside the room slide that crossing its
/// screen's locked edge began, and the slide then carries the camera one
/// screen on. Staging shows the screen the fight is played on: for Tree
/// Zone 5, arena screen 5, where walking through the door arrives.
#[test]
fn a_staged_arena_shows_the_screen_the_fight_plays_on() {
    use sml2_core::player_sim::PlayerKeys;
    use sml2_runtime::session::FrameInput;
    let mut play = session();
    play.begin_level(5, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut play.state else {
        unreachable!()
    };
    // On the ledge in front of the trunk's door pipe.
    (ctx.sim.x, ctx.sim.y, ctx.cam.x, ctx.cam.y) = (0xBA0, 0x2A0, 0xBA0, 0x2B0);
    let right = FrameInput {
        keys: PlayerKeys {
            right: true,
            ..Default::default()
        },
        div: 0,
    };
    let arrived = (0..300)
        .find_map(|_| {
            play.advance(right);
            let SessionState::InLevel(ctx) = &play.state else {
                panic!("left the level")
            };
            (ctx.arena_loaded && ctx.cam.transition == 0).then_some((ctx.cam.x, ctx.cam.y))
        })
        .expect("Mario reaches the arena");
    assert_eq!(arrived, (0x550, 0x060));

    let mut staged = session();
    staged
        .stage_scenery_scene(SceneTarget::Arena { level: 5, gfx: 0 })
        .unwrap();
    let SessionState::InLevel(ctx) = &staged.state else {
        unreachable!()
    };
    assert_eq!((ctx.cam.x, ctx.cam.y), arrived);
}

#[test]
fn tree_arena_surround_follows_the_camera_and_uses_only_arena_swatches() {
    let mut game = session();
    game.expanded_levels = true;
    let target = SceneTarget::Arena { level: 5, gfx: 0 };
    game.stage_scenery_scene(target).unwrap();
    assert!(!game.scene_scenery(target).is_empty());
    let source = game.scene_palette_frame().unwrap();
    // Swatches 5 and 43 are the arena's sky and cloud interior, respectively.
    let sky = source[80];
    let cloud = source[64 * 160 + 48];
    assert_ne!(sky, cloud);
    for (view, origin) in [((384, 216), (112, 24)), ((672, 288), (256, 24))] {
        assert_eq!(game.scene_border_origin(view), origin);
        let painted = game.render_palette_frame(view);
        assert_eq!(painted[0], sky);
        assert_eq!(painted[(view.1 - 16) * view.0], cloud);
        let native = game.render_frame((160, 144));
        let enlarged = game.render_frame(view);
        // The native terrain stays at the level camera's position; the HUD
        // moves to the bottom of the enlarged view.
        for y in 0..128 {
            let at = (origin.1 as usize + y) * view.0 + origin.0 as usize;
            assert_eq!(&enlarged[at..at + 160], &native[y * 160..y * 160 + 160]);
        }
    }
}

#[test]
fn arena_reload_stops_level_scenery_in_native_and_dx_presentation() {
    let mut game = session();
    game.stage_scenery_scene(SceneTarget::Arena { level: 5, gfx: 0 })
        .unwrap();
    let mut dx = synthetic_bundle();
    dx.levels[5].bg_palettes = [0x1f, 0, 0xe0, 3, 0, 0x7c, 0xff, 0x7f].repeat(8);
    dx.levels[5].bg_palettes[8..16].copy_from_slice(&[0xe0, 3].repeat(4));
    dx.levels[5].metatiles.metatiles[127].tiles = [1; 4];
    dx.levels[5].tile_attributes[1] = 1;
    for view in [(160, 144), (384, 216), (672, 288)] {
        let SessionState::InLevel(ctx) = &mut game.state else {
            unreachable!()
        };
        ctx.scenery = SceneryOverlay::empty();
        let native = game.render_frame(view);
        let colored = game.render_dx_frame(&dx, Mario, view).unwrap();
        let SessionState::InLevel(ctx) = &mut game.state else {
            unreachable!()
        };
        // A level-scoped paint over the fight itself must never survive the
        // arena tileset swap, even at the original view size.
        ctx.scenery = SceneryOverlay::try_from_paints(
            (0..12)
                .flat_map(|y| {
                    (70..110).map(move |x| SceneryPaint {
                        x,
                        y,
                        metatile: 127,
                    })
                })
                .collect(),
            128,
        )
        .unwrap();
        assert_eq!(game.render_frame(view), native);
        assert_eq!(game.render_dx_frame(&dx, Mario, view).unwrap(), colored);
    }
}

#[test]
fn modern_arena_hud_gaps_show_scenery_while_ink_and_outlines_survive() {
    use sml2_runtime::render::{motion::Smooth, HudStyle};
    let mut game = session();
    game.hud_style = HudStyle::Modern;
    let mut dx = synthetic_bundle();
    dx.hud_tiles.fill(0x80);
    for level in &mut dx.levels {
        level.bg_palettes = [0x1f, 0, 0xe0, 3, 0, 0x7c, 0xff, 0x7f].repeat(8);
        // Swatch 0 is blue, distinct from the red off-screen fill.
        level.background[0][0x800..0x810].copy_from_slice(&[0, 255].repeat(8));
    }
    let targets: Vec<_> = game
        .scenery_scenes()
        .into_iter()
        .filter(|target| matches!(target, SceneTarget::Arena { .. }))
        .collect();
    assert!(targets.len() >= 6);
    for target in targets {
        game.stage_scenery_scene(target).unwrap();
        let SessionState::InLevel(ctx) = &mut game.state else {
            unreachable!()
        };
        let bgp = ctx.manifest.header.palettes[0];
        let paper = (0..4).find(|&c| (bgp >> (c * 2)) & 3 == 0).unwrap();
        let ink = (0..4).find(|&c| (bgp >> (c * 2)) & 3 == 3).unwrap();
        ctx.hud.template.fill(0x80);
        ctx.hud.digit_columns.clear();
        for y in 0..8 {
            ctx.bg_live[y * 2] = if paper & 1 != 0 { 255 } else { 0 };
            ctx.bg_live[y * 2 + 1] = if paper & 2 != 0 { 255 } else { 0 };
            ctx.bg_live[y * 2] ^= if (paper ^ ink) & 1 != 0 { 0x18 } else { 0 };
            ctx.bg_live[y * 2 + 1] ^= if (paper ^ ink) & 2 != 0 { 0x18 } else { 0 };
        }
        for view in [(384, 216), (672, 288)] {
            let (mx, my) = game.scene_border_origin(view);
            let scenery = SceneryOverlay::try_from_paints(
                (-20..40)
                    .flat_map(|y| (-30..50).map(move |x| SceneryPaint { x, y, metatile: 0 }))
                    .collect(),
                SCENE_SWATCHES,
            )
            .unwrap();
            for phase in [None, Some(0), Some(16)] {
                let SessionState::InLevel(ctx) = &mut game.state else {
                    unreachable!()
                };
                ctx.sim.paused = phase.is_some();
                ctx.sim.frame_counter = phase.unwrap_or(0);
                for scale in [1, 2] {
                    let smooth = Smooth { alpha: 1.0, scale };
                    game.set_scene_scenery(target, SceneryOverlay::empty())
                        .unwrap();
                    let (native, _) = game.render_palette_frame_smooth(view, smooth);
                    let (colored, _) = game
                        .render_dx_frame_smooth(&dx, Mario, view, smooth)
                        .unwrap();
                    let source = game.scene_palette_frame().unwrap();
                    let dx_source = game.scene_palette_dx(&dx, Mario).unwrap();
                    game.set_scene_scenery(target, scenery.clone()).unwrap();
                    let (painted, _) = game.render_palette_frame_smooth(view, smooth);
                    let (dx_painted, _) = game
                        .render_dx_frame_smooth(&dx, Mario, view, smooth)
                        .unwrap();
                    // First and last cells of both counter groups: backing, ring, ink.
                    for x0 in [8, 8 + 9 * 8, view.0 - 16] {
                        for sub_y in 0..scale {
                            for sub_x in 0..scale {
                                let at =
                                    |x| (8 * scale + sub_y) * view.0 * scale + x * scale + sub_x;
                                let sx = (x0 as i32 - mx).rem_euclid(16) as usize;
                                let sy = (8 - my).rem_euclid(16) as usize;
                                assert_eq!(
                                    painted[at(x0)],
                                    source[sy * 160 + sx],
                                    "{target:?} gap"
                                );
                                assert_eq!(
                                    dx_painted[at(x0)],
                                    dx_source[sy * 160 + sx],
                                    "{target:?} DX gap"
                                );
                                for x in [x0 + 2, x0 + 3] {
                                    assert_eq!(
                                        painted[at(x)],
                                        native[at(x)],
                                        "{target:?} ink/ring"
                                    );
                                    assert_eq!(
                                        dx_painted[at(x)],
                                        colored[at(x)],
                                        "{target:?} DX ink/ring"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
