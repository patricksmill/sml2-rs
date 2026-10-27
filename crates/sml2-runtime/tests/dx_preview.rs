use sml2_core::character::Character::{self, Luigi, Mario};
use sml2_dx::{LevelGraphics, PlayerGraphics, Presentation};
use sml2_runtime::{
    content::{ExtractedContent, SessionAssets},
    session::{FrameInput, Session, SessionState},
};
use std::path::PathBuf;

fn session() -> Session {
    let source =
        ExtractedContent::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"));
    Session::new(SessionAssets::load_from(&source).unwrap())
}

const RED: [u8; 3] = [255, 0, 0];

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
        movement: std::array::from_fn(|_| std::sync::Arc::new(sml2_core::character::MovementCurves {
            horizontal: sml2_core::player_sim::SPEED_H,
            normal: sml2_core::player_sim::SPEED_V[..76].try_into().unwrap(),
            swim: sml2_core::player_sim::SPEED_V_GOO[..76].try_into().unwrap(),
            moon: sml2_core::player_sim::SPEED_V_WATER[..76].try_into().unwrap(),
        })),
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
/// The synthetic look over the native levels' geometry: DX gameplay
/// requires every level's spawns to match its objects.
fn native_geometry_bundle() -> Presentation {
    let source =
        ExtractedContent::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"));
    let assets = SessionAssets::load_from(&source).unwrap();
    let mut bundle = synthetic_bundle();
    for (id, level) in bundle.levels.iter_mut().enumerate() {
        let content = assets.level(id as u8).unwrap();
        level.layout = content.layout.cells.clone();
        level.spawns = content.manifest.spawns.records.clone();
    }
    bundle
}

fn screen() -> sml2_dx::Screen {
    sml2_dx::Screen {
        vram: [vec![0; 0x1800], vec![0; 0x1800]],
        tilemap: vec![0; 0x400],
        attributes: vec![0; 0x400],
        bg_palettes: [0x1f, 0].repeat(32),
        obj_palettes: [0xe0, 3].repeat(32),
    }
}

#[test]
fn dx_public_api_covers_every_level_with_its_own_tables() {
    let mut bundle = synthetic_bundle();
    // Level 1 is pure green.
    bundle.levels[1].bg_palettes = [0xe0, 3].repeat(32);
    let mut game = session();
    game.enter_title();
    assert!(game.dx_supported(), "the title has its own DX screen");
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![RED; 160 * 144]
    );
    game.begin_level(0, 0).unwrap();
    assert!(game.dx_supported());
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![RED; 160 * 144]
    );
    game.begin_level(1, 0).unwrap();
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![[0, 255, 0]; 160 * 144]
    );
    // An arena the bundle has no look for keeps the level's.
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.arena_loaded = true;
    assert!(game.dx_supported());
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![[0, 255, 0]; 160 * 144]
    );
}

#[test]
fn both_minigames_draw_in_dx_color() {
    let mut bundle = synthetic_bundle();
    // Screen 9 is pure green; screen 13 is pure blue.
    bundle.map.screens[9].bg_palettes = [0xe0, 3].repeat(32);
    bundle.map.screens[13].bg_palettes = [0x00, 0x7c].repeat(32);
    let mut game = session();
    game.begin_coin_room(9);
    assert!(game.dx_supported());
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![[0, 255, 0]; 160 * 144]
    );
    let wide = game.render_dx_frame(&bundle, Mario, (256, 144)).unwrap();
    assert_eq!(wide.len(), 256 * 144);
    assert_eq!(&wide[48..208], &[[0, 255, 0]; 160]);
    game.begin_bell_room(13, 0, 0);
    assert!(game.dx_supported());
    assert_eq!(
        game.render_dx_frame(&bundle, Luigi, (160, 144)).unwrap(),
        vec![[0, 0, 255]; 160 * 144]
    );
}

#[test]
fn dx_bounded_view_draws_inside_scenery_and_hides_outside_scenery() {
    let mut bundle = synthetic_bundle();
    let mut tiles = [0u8; 512];
    tiles[4..8].fill(1);
    bundle.levels[0].metatiles = sml2_core::level::MetatileTable::from_bytes(&tiles).unwrap();
    let mut background = vec![0u8; 0x1000];
    for row in 0..8 {
        background[0x810 + row * 2] = 0xff;
    }
    bundle.levels[0].background[0] = background;
    let mut palettes = [0x1f, 0].repeat(32);
    palettes[2] = 0xe0;
    palettes[3] = 0x03;
    bundle.levels[0].bg_palettes = palettes;

    let mut game = session();
    game.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut game.state else {
        panic!("in level")
    };
    ctx.cam.x = 80;
    ctx.cam.y = 448;
    ctx.scenery = sml2_core::scenery::SceneryOverlay::try_from_paints(
        vec![
            sml2_core::scenery::SceneryPaint {
                x: -7,
                y: 24,
                metatile: 1,
            },
            sml2_core::scenery::SceneryPaint {
                x: 0,
                y: 24,
                metatile: 1,
            },
        ],
        128,
    )
    .unwrap();
    let view = (384, 216);
    let frame = game.render_dx_frame(&bundle, Mario, view).unwrap();
    let (mx, my) = sml2_runtime::render::view::level_margin(view);
    assert_eq!(frame[(my as usize + 40) * view.0 + mx as usize + 40], RED);
    assert_eq!(frame[76 * view.0], [0, 255, 0]);
    assert_eq!(frame[76 * view.0 + 16], RED);
    // Moving the simulation camera inside the parked range keeps the same view.
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.cam.x = 160;
    assert!(game.render_dx_frame(&bundle, Mario, view).unwrap() == frame);
}

#[test]
fn dx_frames_take_the_view_size_and_keep_native_parity() {
    let mut bundle = synthetic_bundle();
    // Mario is one opaque green cell and the HUD row is blue, so the frame
    // is not uniform red.
    bundle.players[0][0].normal[..16].copy_from_slice(&[0xff, 0].repeat(8));
    bundle.players[0][0].mappings = vec![
        vec![sml2_core::object::MappingEntry {
            y: 0,
            x: 0,
            tile: 0,
            attr: 0,
        }];
        242
    ];
    bundle.levels[0].bg_palettes[8..10].copy_from_slice(&[0, 0x7c]);
    bundle.levels[0].hud_attributes = vec![1; 20];
    let mut game = session();
    game.enter_title();
    let native = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
    let wide = game.render_dx_frame(&bundle, Mario, (384, 216)).unwrap();
    assert_eq!(wide.len(), 384 * 216);
    assert_eq!(wide[108 * 384 + 192], native[72 * 160 + 80]);
    assert_eq!(wide[0], native[0], "the title backdrop fills the corner");
    game.begin_level(0, 0).unwrap();
    game.advance(FrameInput::default());
    for (camera, crop_at) in [
        ((80, 448), (0, 54)),
        ((4016, 448), (224, 54)),
        ((80, 704), (0, 80)),
    ] {
        let SessionState::InLevel(ctx) = &mut game.state else {
            unreachable!()
        };
        ctx.cam.x = camera.0;
        ctx.cam.y = camera.1;
        ctx.sim.x = camera.0 - 46;
        ctx.sim.y = camera.1 + 20;
        game.advance(FrameInput::default());
        game.preview_scenery(camera, sml2_core::scenery::SceneryOverlay::empty());
        let native = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
        assert!(native[..160 * 136].iter().any(|&p| p != RED), "Mario shows");
        let wide = game.render_dx_frame(&bundle, Mario, (384, 216)).unwrap();
        assert_eq!(wide.len(), 384 * 216);
        let picture: Vec<_> = (0..120)
            .flat_map(|y| {
                wide[(crop_at.1 + y) * 384 + crop_at.0..(crop_at.1 + y) * 384 + crop_at.0 + 160]
                    .to_vec()
            })
            .collect();
        assert!(picture == native[..160 * 120], "camera {camera:?}");
        assert_eq!(wide[215 * 384 + 192], native[143 * 160 + 80]);
        assert_ne!(wide[215 * 384 + 192], RED);
    }
}

#[test]
fn file_select_clear_draws_bomb_mario_in_dx_and_restores_the_regular_cursor() {
    use sml2_core::player_sim::PlayerKeys;

    let mut game = session();
    game.enter_file_select();
    for _ in 0..45 {
        game.advance(FrameInput::default());
    }
    for _ in 0..3 {
        game.advance(FrameInput {
            keys: PlayerKeys {
                right: true,
                ..Default::default()
            },
            div: 0,
        });
        for _ in 0..32 {
            game.advance(FrameInput::default());
        }
    }

    let mut bundle = synthetic_bundle();
    // Only the ROM's six Bomb Mario mappings draw a green 8x8 cell. Regular
    // Mario and the tally tiles stay transparent over the red background.
    for character in &mut bundle.players {
        let player = &mut character[0];
        player.normal[16..32].copy_from_slice(&[0xff, 0].repeat(8));
        for frame in 201..=206 {
            player.mappings[frame] = vec![sml2_core::object::MappingEntry {
                y: 0,
                x: 0,
                tile: 1,
                attr: 0,
            }];
        }
    }
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![RED; 160 * 144]
    );

    for armed in [true, false] {
        game.advance(FrameInput {
            keys: PlayerKeys {
                a: true,
                ..Default::default()
            },
            div: 0,
        });
        for _ in 0..55 {
            game.advance(FrameInput::default());
        }
        let SessionState::FileSelect(fs) = &game.state else {
            panic!("file select")
        };
        assert_eq!(fs.clear_mode(), armed);
        assert_eq!(fs.cursor(), (0x88, 0x7c));
        for character in [Mario, Luigi] {
            let pixels = game
                .render_dx_frame(&bundle, character, (160, 144))
                .unwrap();
            assert_eq!(
                pixels.iter().filter(|&&p| p == [0, 255, 0]).count(),
                if armed { 64 } else { 0 }
            );
            if armed {
                assert_eq!(
                    pixels[109 * 160 + 128],
                    [0, 255, 0],
                    "Bomb Mario at the Clear button"
                );
            } else {
                assert_eq!(pixels, vec![RED; 160 * 144]);
            }
        }
    }
}

#[test]
fn luigi_and_the_alternate_theme_use_their_own_graphics() {
    let mut game = session();
    game.begin_level(0, 0).unwrap();
    game.advance(FrameInput::default());
    let mut bundle = synthetic_bundle();
    // One opaque cell over the start position: tile 0 is solid color 1.
    let cell = vec![
        vec![sml2_core::object::MappingEntry {
            y: 0,
            x: 0,
            tile: 0,
            attr: 0,
        }];
        242
    ];
    for (character, theme) in [(1, 0), (0, 1)] {
        let player = &mut bundle.players[character][theme];
        player.normal[..16].copy_from_slice(&[0xff, 0].repeat(8));
        player.mappings = cell.clone();
    }
    let mario = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
    let luigi = game.render_dx_frame(&bundle, Luigi, (160, 144)).unwrap();
    assert_ne!(mario, luigi);
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        mario
    );
    bundle.levels[0].theme = 1;
    assert_ne!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        mario
    );
}

#[test]
fn dx_render_frequency_preserves_native_frames_gameplay_and_save_bytes() {
    let bundle = synthetic_bundle();
    let mut plain = session();
    let mut displayed = session();
    plain.begin_level(0, 0).unwrap();
    displayed.begin_level(0, 0).unwrap();
    for tick in 0..40u8 {
        let old_frame = displayed.render_frame((160, 144));
        let save = displayed.save_record();
        let color = displayed
            .render_dx_frame(&bundle, Mario, (160, 144))
            .unwrap();
        assert_eq!(
            displayed
                .render_dx_frame(&bundle, Mario, (160, 144))
                .unwrap(),
            color
        );
        assert_eq!(displayed.render_frame((160, 144)), old_frame);
        assert_eq!(displayed.save_record(), save);
        let input = FrameInput {
            div: tick.wrapping_mul(17),
            ..Default::default()
        };
        let a = plain.advance(input);
        let b = displayed.advance(input);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.saves, b.saves);
        assert_eq!(a.persist_slots, b.persist_slots);
        assert_eq!(plain.save_record(), displayed.save_record());
        assert_eq!(plain.sound_driver, displayed.sound_driver);
        let (SessionState::InLevel(a), SessionState::InLevel(b)) = (&plain.state, &displayed.state)
        else {
            panic!("opening level ended")
        };
        assert_eq!(format!("{:?}", a.sim), format!("{:?}", b.sim));
        assert_eq!(a.layout_live.cells, b.layout_live.cells);
        assert_eq!(a.tile_animation, b.tile_animation);
        for (a, b) in a.obj.slots.iter().zip(&b.obj.slots) {
            assert_eq!(a.b, b.b);
        }
    }
}

fn audited() -> Presentation {
    let rom = std::fs::read(std::env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM")).unwrap();
    Presentation::new(&sml2_dx::extract(&rom).unwrap()).unwrap()
}

/// Local acceptance with user-provided ROM; never embeds copyrighted content.
/// Set DX_OUTPUT_IMAGE to write the initial native frame for independent comparison.
#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_rom_forms_effects_and_initial_frame() {
    let bundle = audited();
    let mut game = session();
    game.begin_level(0, 0).unwrap();
    let initial = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
    assert!(initial.iter().any(|p| p[0] != p[1] || p[1] != p[2]));
    if let Some(path) = std::env::var_os("DX_OUTPUT_IMAGE") {
        let mut preview = session();
        preview.begin_level(0, 0).unwrap();
        let ticks = std::env::var("DX_OUTPUT_TICKS")
            .unwrap_or_else(|_| "1".into())
            .parse::<u32>()
            .unwrap();
        for _ in 0..ticks {
            preview.advance(FrameInput::default());
        }
        let pixels = preview.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
        if let SessionState::InLevel(ctx) = &preview.state {
            eprintln!("DX comparison: ticks={ticks}, Mario=({},{}), camera=({},{}), animation={:?}, HUD={}",
                ctx.sim.x, ctx.sim.y, ctx.cam.x, ctx.cam.y, ctx.tile_animation, ctx.hud_y);
        }
        let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), 160, 144);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels.concat())
            .unwrap();
    }
    let mut frames = Vec::new();
    for powerup in 0..=3 {
        let mut form = session();
        form.begin_level(0, 0).unwrap();
        let SessionState::InLevel(ctx) = &mut form.state else {
            unreachable!()
        };
        ctx.sim.powerup = powerup;
        form.advance(FrameInput::default());
        let save = form.save_record();
        let grayscale = form.render_frame((160, 144));
        let color = form.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
        assert_eq!(
            form.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
            color
        );
        assert_eq!(form.render_frame((160, 144)), grayscale);
        assert_eq!(form.save_record(), save);
        frames.push(color);
    }
    assert_ne!(frames[0], frames[1]);
    assert_ne!(frames[1], frames[2]);
    assert_ne!(frames[1], frames[3]);
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.sim.paused = true;
    ctx.sim.frame_counter = 0;
    let pause0 = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.sim.frame_counter = 16;
    assert_ne!(
        pause0,
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap()
    );
    let mut stars = Vec::new();
    for low in [4, 6] {
        let SessionState::InLevel(ctx) = &mut game.state else {
            unreachable!()
        };
        ctx.sim.paused = false;
        ctx.sim.star_high = 1;
        ctx.sim.star_low = low + 1; // advance decrements before staging draw.
        game.advance(FrameInput::default());
        stars.push(game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap());
    }
    assert_ne!(stars[0], stars[1]);
    // Luigi's forms differ from Mario's and from each other on the same native state.
    let mut luigi_frames = Vec::<Vec<[u8; 3]>>::new();
    for powerup in 0..=3 {
        let mut form = session();
        form.begin_level(0, 0).unwrap();
        let SessionState::InLevel(ctx) = &mut form.state else {
            unreachable!()
        };
        ctx.sim.powerup = powerup;
        form.advance(FrameInput::default());
        let color = form
            .render_dx_frame(&bundle, Character::Luigi, (160, 144))
            .unwrap();
        assert_ne!(color, frames[powerup as usize]);
        luigi_frames.push(color);
    }
    assert_ne!(luigi_frames[0], luigi_frames[1]);
    assert_ne!(luigi_frames[1], luigi_frames[3]);
}

/// Every level runs in DX for both characters, and so does a boss arena once
/// `$25AF` swaps its tileset in.
#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_rom_draws_every_level_and_its_arenas() {
    let bundle = audited();
    for level in 0..32u8 {
        let mut game = session();
        game.begin_level(level, 0).unwrap();
        for tick in 0..240u32 {
            if let SessionState::InLevel(ctx) = &mut game.state {
                ctx.sim.star_high = 1; // Keep Mario alive through the run.
            }
            let keys = sml2_core::player_sim::PlayerKeys {
                right: true,
                b: true,
                a: tick % 40 < 18,
                ..Default::default()
            };
            game.advance(FrameInput {
                keys,
                div: tick as u8,
            });
            if !game.dx_supported() {
                break;
            }
            for character in [Mario, Luigi] {
                let frame = game.render_dx_frame(&bundle, character, (160, 144));
                let frame = frame.unwrap_or_else(|e| panic!("level {level} tick {tick}: {e}"));
                assert!(frame.iter().any(|p| p[0] != p[1] || p[1] != p[2]));
            }
        }
    }
    // Level $09's side pipe at the door boss's arena (see boss_entry/door.rs).
    let mut game = session();
    game.begin_level(9, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.sim.x = 0x0EB8;
    ctx.sim.y = 0x01D4;
    let (edge, scene) = (ctx.cam.edge_map, ctx.cam.scene_map.clone());
    ctx.cam = sml2_core::camera::CameraSim::new(0x0EB0, 0x01C0);
    ctx.cam.edge_map = edge;
    ctx.cam.scene_map = scene;
    ctx.cam.prev_mario_y_lo = ctx.sim.y as u8;
    ctx.cam.update_enables(ctx.sim.x);
    assert!(game.dx_supported());
    let right = FrameInput {
        keys: sml2_core::player_sim::PlayerKeys {
            right: true,
            ..Default::default()
        },
        div: 0,
    };
    for _ in 0..120 {
        game.advance(right);
    }
    let SessionState::InLevel(ctx) = &game.state else {
        panic!("the arena is part of the level")
    };
    assert!(ctx.arena_loaded);
    assert!(game.dx_supported());
    for character in [Mario, Luigi] {
        game.render_dx_frame(&bundle, character, (160, 144))
            .unwrap();
    }
}

/// The title, and the file select 60 ticks after it opens, matched PyBoy's
/// frames of the audited ROM on all 23,040 pixels; their RGB hashes pin them.
#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_rom_title_and_file_select_match_their_pyboy_frames() {
    let bundle = audited();
    let mut game = session();
    game.enter_title();
    let title = game
        .render_dx_frame(&bundle, Mario, (160, 144))
        .unwrap()
        .concat();
    assert_eq!(
        sml2_dx::sha256(&title),
        "877cefb301d5aabf14414f03f20e5ef3f39e1e5e55ceb8dfc0ade1f41dd1e1c7"
    );
    game.enter_file_select();
    for _ in 0..60 {
        game.advance(FrameInput::default());
    }
    let file_select = game
        .render_dx_frame(&bundle, Mario, (160, 144))
        .unwrap()
        .concat();
    assert_eq!(
        sml2_dx::sha256(&file_select),
        "b34891637e549b91cf3d56ed65bb2fd7c79f725814a3824453a57bbb35ae6232"
    );
}

/// The hub and every zone screen draw in DX color; the hub and Tree Zone
/// matched PyBoy's frames of this ROM except where animation timing differs.
#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_rom_draws_the_hub_and_zone_screens() {
    let bundle = audited();
    for (screen, node) in [
        (0u8, 0x14u8),
        (1, 0x28),
        (2, 0x23),
        (3, 0x32),
        (4, 0x3C),
        (5, 0x4B),
        (6, 0x50),
        (7, 0x5A),
        (8, 0x72),
    ] {
        let mut game = session();
        game.debug_clear_all_levels();
        game.debug_warp_map(screen, node).unwrap();
        game.advance(FrameInput::default());
        assert!(game.dx_supported(), "screen {screen}");
        for character in [Mario, Luigi] {
            let frame = game
                .render_dx_frame(&bundle, character, (160, 144))
                .unwrap();
            let colors: std::collections::BTreeSet<_> = frame.iter().collect();
            assert!(colors.len() > 8, "screen {screen} has its palettes");
            if screen != 0 {
                for view in ZONE_VIEWS {
                    let wide = game.render_dx_frame(&bundle, character, view).unwrap();
                    assert_zone_picture(&wide, &frame, view, screen);
                }
            }
        }
    }
}

const ZONE_VIEWS: [(usize, usize); 6] = [
    (200, 180),
    (240, 216),
    (256, 144),
    (320, 288),
    (384, 216),
    (512, 288),
];

fn assert_zone_picture(frame: &[[u8; 3]], native: &[[u8; 3]], view: (usize, usize), screen: u8) {
    let (mx, my) = ((view.0 - 160) / 2, (view.1 - 144) / 2);
    assert_eq!(frame.len(), view.0 * view.1);
    for y in 0..view.1 {
        let row = &frame[y * view.0..(y + 1) * view.0];
        if (my..my + 144).contains(&y) {
            assert_eq!(
                &row[mx..mx + 160],
                &native[(y - my) * 160..(y - my + 1) * 160],
                "screen {screen}, view {view:?}, row {y}"
            );
            assert!(
                row[..mx]
                    .iter()
                    .chain(&row[mx + 160..])
                    .all(|pixel| *pixel == [0; 3]),
                "unused map columns on screen {screen}, view {view:?}, row {y}"
            );
        } else {
            assert!(
                row.iter().all(|pixel| *pixel == [0; 3]),
                "unused map rows on screen {screen}, view {view:?}, row {y}"
            );
        }
    }
}

#[test]
fn every_dx_zone_hides_unused_columns_at_all_view_sizes() {
    let bundle = synthetic_bundle();
    let mut game = session();
    for cleared in [false, true] {
        if cleared {
            game.debug_clear_all_levels();
        }
        for (screen, node) in [
            (1, 0x28),
            (2, 0x23),
            (3, 0x32),
            (4, 0x3C),
            (5, 0x4B),
            (6, 0x50),
            (7, 0x5A),
            (8, 0x72),
        ] {
            game.debug_warp_map(screen, node).unwrap();
            for _ in 0..3 {
                for character in [Mario, Luigi] {
                    let native = game
                        .render_dx_frame(&bundle, character, (160, 144))
                        .unwrap();
                    assert!(
                        native.iter().any(|pixel| *pixel != [0; 3]),
                        "the fixture must expose incorrectly sampled columns"
                    );
                    for view in ZONE_VIEWS {
                        assert_zone_picture(
                            &game.render_dx_frame(&bundle, character, view).unwrap(),
                            &native,
                            view,
                            screen,
                        );
                    }
                }
                game.advance(FrameInput::default());
                assert!(
                    matches!(&game.state, SessionState::Overworld(map) if map.screen_id == screen)
                );
            }
        }
    }
}

#[test]
#[ignore = "exports every zone in normal and DX rendering; requires SML2_DX_ROM"]
fn export_all_zone_previews() {
    let bundle = audited();
    let mut game = session();
    game.debug_clear_all_levels();
    let view = (256, 144);
    let size = (512, 8 * 144);
    let mut pixels = vec![[0u8; 3]; size.0 * size.1];
    for (index, (screen, node)) in [
        (1, 0x28),
        (2, 0x23),
        (3, 0x32),
        (4, 0x3C),
        (5, 0x4B),
        (6, 0x50),
        (7, 0x5A),
        (8, 0x72),
    ]
    .into_iter()
    .enumerate()
    {
        game.debug_warp_map(screen, node).unwrap();
        let native = game.render_frame(view);
        let dx = game.render_dx_frame(&bundle, Mario, view).unwrap();
        for y in 0..144 {
            let row = (index * 144 + y) * size.0;
            for x in 0..256 {
                pixels[row + x] = [native[y * 256 + x]; 3];
            }
            pixels[row + 256..row + 512].copy_from_slice(&dx[y * 256..(y + 1) * 256]);
        }
    }
    let path = std::env::temp_dir().join("sml2-zone-previews.png");
    let mut encoder = png::Encoder::new(
        std::fs::File::create(&path).unwrap(),
        size.0 as u32,
        size.1 as u32,
    );
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels.into_iter().flatten().collect::<Vec<_>>())
        .unwrap();
}

#[test]
fn dx_smoothing_at_a_full_tick_is_the_dx_frame_upsampled() {
    let bundle = synthetic_bundle();
    let mut game = session();
    game.begin_level(0, 0).unwrap();
    for _ in 0..30 {
        game.advance(FrameInput::default());
    }
    let native = game.render_dx_frame(&bundle, Mario, (256, 144)).unwrap();
    let (smooth, scale) = game
        .render_dx_frame_smooth(
            &bundle,
            Mario,
            (256, 144),
            sml2_runtime::render::motion::Smooth {
                alpha: 1.0,
                scale: 3,
            },
        )
        .unwrap();
    assert_eq!(scale, 3);
    assert_eq!(smooth.len(), 768 * 432);
    assert!(
        (0..432).all(|y| (0..768).all(|x| smooth[y * 768 + x] == native[(y / 3) * 256 + x / 3]))
    );
}

#[test]
fn file_select_camera_stays_at_zero_across_the_three_pipes_and_clear() {
    use sml2_core::{object::MappingEntry, player_sim::PlayerKeys};
    let mut bundle = synthetic_bundle();
    bundle.players[0][0].normal[..16].copy_from_slice(&[255, 0].repeat(8));
    bundle.players[0][0].mappings = vec![
        vec![MappingEntry {
            x: 0,
            y: 0,
            tile: 0,
            attr: 0
        }];
        242
    ];
    let mut game = session();
    game.enter_file_select();
    for _ in 0..64 {
        game.advance(FrameInput::default());
    }
    for cursor_x in [40, 72, 104, 136] {
        let SessionState::FileSelect(file_select) = &game.state else {
            panic!("file select")
        };
        assert_eq!(file_select.cursor(), (cursor_x, 124));
        let frame = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
        assert_eq!(
            frame[108 * 160 + usize::from(cursor_x) - 8],
            [0, 255, 0],
            "cursor {cursor_x} must remain at its original screen position"
        );
        if cursor_x != 136 {
            game.advance(FrameInput {
                keys: PlayerKeys {
                    right: true,
                    ..Default::default()
                },
                div: 0,
            });
            for _ in 0..32 {
                game.advance(FrameInput::default());
            }
        }
    }
}

#[test]
fn minigame_surrounds_use_room_color_instead_of_unused_map_columns() {
    let mut bundle = synthetic_bundle();
    for screen in 9..=14 {
        let colors = &mut bundle.map.screens[screen];
        colors.bg_palettes = [0xe0, 3].repeat(32);
        colors.bg_palettes[8..10].copy_from_slice(&[0, 0x7c]);
        let mut attrs = vec![0; 32 * 18];
        for y in 0..18 {
            attrs[y * 32 + 20..y * 32 + 32].fill(1);
        }
        colors.attributes = Some(attrs);
        let mut game = session();
        if screen < 13 {
            game.begin_coin_room(screen as u8);
        } else {
            game.begin_bell_room(screen as u8, 0, 0);
        }
        for view in [(256, 144), (384, 216), (512, 288)] {
            let frame = game.render_dx_frame(&bundle, Mario, view).unwrap();
            assert_eq!(
                frame,
                vec![[0, 255, 0]; view.0 * view.1],
                "screen {screen} at {view:?}"
            );
        }
    }
}

#[test]
fn castle_reveal_and_game_over_use_their_own_dx_palettes() {
    let mut bundle = synthetic_bundle();
    for bank in [0x05, 0x0c, 0x26, 0x27] {
        bundle.banks.insert(bank, vec![0; 0x4000]);
    }
    let palette = bundle.banks.get_mut(&0x26).unwrap();
    palette[0x3c39..0x3c3b].copy_from_slice(&[0x1f, 0]);
    palette[0x3042..0x3044].copy_from_slice(&[0xe0, 3]);
    let mut game = session();
    game.state = SessionState::GameOver(Box::new(sml2_runtime::session::GameOverScreen {
        sim: Default::default(),
        obp: [0xd0, 0x38],
    }));
    assert!(game.dx_supported());
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![RED; 160 * 144]
    );
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (384, 216)).unwrap(),
        vec![RED; 384 * 216],
        "Game Over extends its own palette"
    );
    game.state = SessionState::Castle(Default::default());
    assert!(game.dx_supported());
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![[0, 255, 0]; 160 * 144]
    );
    let large = game.render_dx_frame(&bundle, Luigi, (384, 216)).unwrap();
    assert_eq!(large.len(), 384 * 216);
    assert_eq!(large[36 * 384 + 112], [0, 255, 0]);
    assert_eq!(
        large[0],
        [0, 255, 0],
        "the castle scenery continues into the enlarged view"
    );
}

#[test]
fn conveyor_section_colors_survive_corridors_and_widescreen() {
    let mut bundle = synthetic_bundle();
    let bank = bundle.banks.get_mut(&0x26).unwrap();
    bank[0x2140..0x2180].copy_from_slice(&[0xe0, 3].repeat(32));
    bank[0x2180..0x21c0].copy_from_slice(&[0, 0x7c].repeat(32));
    let mut game = session();
    game.begin_level(0x0c, 0).unwrap();
    for (x, expected) in [
        (0x200u16, [0, 255, 0]),
        (0x300, [0, 255, 0]),
        (0x500, [0, 0, 255]),
        (0x600, [0, 0, 255]),
        (0xe00, RED),
    ] {
        let SessionState::InLevel(ctx) = &mut game.state else {
            panic!("level")
        };
        ctx.obj.mario_ref_x = x.wrapping_add(8);
        ctx.obj.mario_ref_y = ctx.sim.y.wrapping_add(32);
        ctx.cam.scene_id = 0;
        ctx.scene_vm = sml2_core::scene::SceneVm::new();
        game.advance(FrameInput::default());
        let frame = game.render_dx_frame(&bundle, Mario, (256, 144)).unwrap();
        assert_eq!(frame[0], expected, "Mario X {x:04x}");
        let again = game.render_dx_frame(&bundle, Mario, (256, 144)).unwrap();
        assert_eq!(again, frame, "drawing does not advance palette timing");
        let luigi = game.render_dx_frame(&bundle, Luigi, (256, 144)).unwrap();
        assert_eq!(luigi[0], expected, "Luigi shares regional BG colors");
    }
}

/// `$26:$6B0B`: past a trigger, the eight regional levels draw with the set
/// it loads. BG swaps recolor the whole frame; OBJ-only levels keep drawing.
#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_rom_regional_palettes_follow_mario() {
    let bundle = audited();
    // (level, Mario X, Mario Y, whether the background set changes)
    for (level, x, y, bg_changes) in [
        (0x0Cu8, 0x0240u16 - 8, 0x0100u16, true),
        (0x15, 0x0940 - 8, 0x0240 - 32, true),
        (0x19, 0x0180 - 8, 0x0100, true),
        (0x0D, 0x0840 - 8, 0x0100, true),
        (0x03, 0x0640 - 8, 0x0100, false),
        (0x07, 0x0840 - 8, 0x0100, false),
        (0x16, 0x0940 - 8, 0x0100, false),
        (0x18, 0x0840 - 8, 0x0240 - 32, false),
    ] {
        let mut game = session();
        game.begin_level(level, 0).unwrap();
        game.advance(FrameInput::default());
        let before = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
        let SessionState::InLevel(ctx) = &mut game.state else {
            unreachable!()
        };
        ctx.sim.x = x;
        ctx.sim.y = y;
        ctx.obj.mario_ref_x = x.wrapping_add(8);
        ctx.obj.mario_ref_y = y.wrapping_add(32);
        ctx.cam.scene_id = 0;
        ctx.scene_vm = sml2_core::scene::SceneVm::new();
        game.advance(FrameInput::default());
        let after = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
        if bg_changes {
            let colors =
                |f: &[[u8; 3]]| f.iter().copied().collect::<std::collections::BTreeSet<_>>();
            assert_ne!(
                colors(&before),
                colors(&after),
                "level {level:02X} keeps its palette"
            );
        }
    }
}

#[test]
fn dx_character_selection_reaches_level_entry_and_falls_back_to_mario() {
    let dx = std::sync::Arc::new(native_geometry_bundle());
    let mut session = session();
    session.set_dx(Some(dx.clone()), Luigi);
    session.debug_warp_level(0).unwrap();
    let SessionState::InLevel(ctx) = &session.state else { panic!("level is live") };
    assert_eq!(ctx.sim.character, Luigi);
    assert!(ctx.sim.dx_enabled());

    // Disabling DX holds until the next entry, which runs vanilla Mario.
    session.set_dx(None, Luigi);
    let SessionState::InLevel(ctx) = &session.state else { panic!("level is live") };
    assert!(ctx.sim.dx_enabled());
    session.debug_warp_level(0).unwrap();
    let SessionState::InLevel(ctx) = &session.state else { panic!("level is live") };
    assert_eq!(ctx.sim.character, Mario);
    assert!(!ctx.sim.dx_enabled());
}

#[test]
fn attract_demos_always_run_dx_mario() {
    let mut session = session();
    session.set_dx(Some(std::sync::Arc::new(native_geometry_bundle())), Luigi);
    assert!(session.start_demo(0));
    let SessionState::Demo(demo) = &session.state else { panic!("demo is live") };
    assert_eq!(demo.ctx.sim.character, Mario);
    assert!(demo.ctx.sim.dx_enabled());
    assert_eq!(session.character, Luigi, "the selection survives the demo");
}

/// `$25AF` uploads `$36:$7746`'s white; it lasts until the door scroll's end
/// runs `$0EC0`/`$7306`, exactly the frames native play blanks.
#[test]
fn dx_boss_reload_shows_white_for_the_native_blank_window() {
    let bundle = synthetic_bundle();
    let mut game = session();
    game.begin_level(13, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut game.state else { panic!("level") };
    ctx.cam.scene_id = 14;
    ctx.cam.transition = 1;
    ctx.cam.x = 0x6B0;
    ctx.scene_vm.start();
    let mut white_frames = 0;
    for _ in 0..80 {
        game.advance(FrameInput::default());
        let native_blank = game.render_frame((160, 144)).iter().all(|&p| p == 255);
        let dx = game.render_dx_frame(&bundle, Luigi, (160, 144)).unwrap();
        assert_eq!(dx.iter().all(|&p| p == [255; 3]), native_blank);
        if native_blank {
            white_frames += 1;
        } else if white_frames != 0 {
            assert_eq!(dx[0], RED, "the level's colors return with the arena");
            break;
        }
    }
    assert!(white_frames > 20, "reload and door scroll stay white");
}

/// `$3F307` -> `$26:$765A` uploads white, then queues job 7 before job 4, so
/// the room's first frame keeps the white palettes.
#[test]
fn dx_coin_room_entry_shows_one_white_frame() {
    let mut bundle = synthetic_bundle();
    bundle.map.screens[9].bg_palettes = [0xe0, 3].repeat(32);
    let mut game = session();
    game.debug_warp_map(0, 0x71).unwrap();
    let SessionState::Overworld(map) = &mut game.state else { panic!("map") };
    (map.coins_low, map.coins_high) = (0x99, 0x09);
    let press_a = FrameInput {
        keys: sml2_core::player_sim::PlayerKeys { a: true, ..Default::default() },
        div: 0,
    };
    game.advance(press_a);
    let SessionState::Overworld(map) = &game.state else { panic!("map") };
    assert_eq!(map.screen_id, 9, "the coin door opened");
    let white = game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap();
    assert_eq!(white, vec![[255; 3]; 160 * 144]);
    assert_eq!(game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(), white);
    game.advance(FrameInput::default());
    assert_eq!(
        game.render_dx_frame(&bundle, Mario, (160, 144)).unwrap(),
        vec![[0, 255, 0]; 160 * 144]
    );
}

/// DX sessions simulate DX's layout and spawn positions from level entry;
/// sessions without DX keep the native level.
#[test]
fn dx_geometry_and_spawns_drive_simulation_from_entry() {
    let mut bundle = native_geometry_bundle();
    // Decoration everywhere: DX ground the native level has is gone.
    bundle.levels[0].layout.fill(0x7F);
    let moved = bundle.levels[0].spawns[0].y.wrapping_add(8);
    bundle.levels[0].spawns[0].y = moved;
    let bundle = std::sync::Arc::new(bundle);

    let mut native = session();
    native.begin_level(0, 0).unwrap();
    let mut dx = session();
    dx.set_dx(Some(bundle.clone()), Mario);
    dx.begin_level(0, 0).unwrap();
    let y = |game: &Session| match &game.state {
        SessionState::InLevel(ctx) => ctx.sim.y,
        _ => panic!("level"),
    };
    let start = y(&dx);
    assert_eq!(start, y(&native), "the header still places Mario");
    let SessionState::InLevel(ctx) = &dx.state else { panic!("level") };
    assert!(ctx.layout_live.cells.iter().all(|&c| c == 0x7F));
    assert_eq!(ctx.obj.records[0].y, moved);
    let SessionState::InLevel(native_ctx) = &native.state else { panic!("level") };
    assert_ne!(native_ctx.obj.records[0].y, moved);
    // The DX renderer draws the simulated cells; nothing substitutes native ones.
    assert!(dx.render_dx_frame(&bundle, Mario, (160, 144)).is_ok());

    for _ in 0..30 {
        native.advance(FrameInput::default());
        dx.advance(FrameInput::default());
    }
    assert_eq!(y(&native), start, "native ground holds Mario");
    assert!(y(&dx) > start + 16, "Mario falls through DX's missing ground");

    // A bundle whose objects disagree with the level is refused at entry.
    let mut mismatched = native_geometry_bundle();
    mismatched.levels[0].spawns.pop();
    let mut game = session();
    game.set_dx(Some(std::sync::Arc::new(mismatched)), Mario);
    assert!(game.begin_level(0, 0).is_err());
}

/// Every audited level enters on DX geometry; only level 0B moves a spawn.
#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_rom_levels_enter_with_dx_layouts_and_spawns() {
    let bundle = std::sync::Arc::new(audited());
    let native = session();
    for id in 0..32u8 {
        let mut game = session();
        game.set_dx(Some(bundle.clone()), Mario);
        game.begin_level(id, 0).unwrap();
        let SessionState::InLevel(ctx) = &game.state else { panic!("level {id:02X}") };
        assert_eq!(ctx.layout_live.cells, bundle.levels[usize::from(id)].layout, "level {id:02X}");
        let native_records = &native.assets.level(id).unwrap().manifest.spawns.records;
        let moved: Vec<_> = ctx
            .obj
            .records
            .iter()
            .zip(native_records)
            .enumerate()
            .filter(|(_, (dx, native))| (dx.x, dx.y) != (native.x, native.y))
            .map(|(i, (dx, native))| (i, native.y, dx.y))
            .collect();
        let expected: &[(usize, u16, u16)] = if id == 0x0B { &[(30, 392, 400)] } else { &[] };
        assert_eq!(moved, expected, "level {id:02X}");
    }
}
