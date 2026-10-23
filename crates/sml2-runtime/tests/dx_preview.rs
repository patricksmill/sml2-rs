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
    };
    let player = PlayerGraphics {
        normal: vec![0; 0x800],
        fire: vec![0; 0x800],
        bank1: vec![0; 0x800],
        mappings: vec![vec![]; 242],
    };
    Presentation {
        levels: vec![level; 32],
        players: [[player.clone(), player.clone()], [player.clone(), player]],
        object_tables: vec![vec![vec![]; 228]],
        hud_tiles: vec![0; 20],
        animation_rows: vec![0; 0x1000],
        banks: Default::default(),
    }
}

#[test]
fn dx_public_api_covers_every_level_with_its_own_tables() {
    let mut bundle = synthetic_bundle();
    // Level 1 is pure green.
    bundle.levels[1].bg_palettes = [0xe0, 3].repeat(32);
    let mut game = session();
    assert!(!game.dx_supported(), "the title keeps its native picture");
    assert!(game.render_dx_frame(&bundle, Mario).is_err());
    game.begin_level(0, 0).unwrap();
    assert!(game.dx_supported());
    assert_eq!(
        game.render_dx_frame(&bundle, Mario).unwrap(),
        vec![RED; 160 * 144]
    );
    game.begin_level(1, 0).unwrap();
    assert_eq!(
        game.render_dx_frame(&bundle, Mario).unwrap(),
        vec![[0, 255, 0]; 160 * 144]
    );
    // Boss arenas use DX tables that are not imported: native picture.
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.arena_loaded = true;
    assert!(!game.dx_supported());
    assert!(game.render_dx_frame(&bundle, Mario).is_err());
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
    let mario = game.render_dx_frame(&bundle, Mario).unwrap();
    let luigi = game.render_dx_frame(&bundle, Luigi).unwrap();
    assert_ne!(mario, luigi);
    assert_eq!(game.render_dx_frame(&bundle, Mario).unwrap(), mario);
    bundle.levels[0].theme = 1;
    assert_ne!(game.render_dx_frame(&bundle, Mario).unwrap(), mario);
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
        let color = displayed.render_dx_frame(&bundle, Mario).unwrap();
        assert_eq!(displayed.render_dx_frame(&bundle, Mario).unwrap(), color);
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
    let initial = game.render_dx_frame(&bundle, Mario).unwrap();
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
        let pixels = preview.render_dx_frame(&bundle, Mario).unwrap();
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
        let color = form.render_dx_frame(&bundle, Mario).unwrap();
        assert_eq!(form.render_dx_frame(&bundle, Mario).unwrap(), color);
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
    let pause0 = game.render_dx_frame(&bundle, Mario).unwrap();
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.sim.frame_counter = 16;
    assert_ne!(pause0, game.render_dx_frame(&bundle, Mario).unwrap());
    let mut stars = Vec::new();
    for low in [4, 6] {
        let SessionState::InLevel(ctx) = &mut game.state else {
            unreachable!()
        };
        ctx.sim.paused = false;
        ctx.sim.star_high = 1;
        ctx.sim.star_low = low + 1; // advance decrements before staging draw.
        game.advance(FrameInput::default());
        stars.push(game.render_dx_frame(&bundle, Mario).unwrap());
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
        let color = form.render_dx_frame(&bundle, Character::Luigi).unwrap();
        assert_ne!(color, frames[powerup as usize]);
        luigi_frames.push(color);
    }
    assert_ne!(luigi_frames[0], luigi_frames[1]);
    assert_ne!(luigi_frames[1], luigi_frames[3]);
}

/// Every level runs in DX for both characters, and a boss arena hands back to
/// the native picture once `$25AF` swaps its tileset in.
#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_rom_draws_every_level_and_leaves_arenas_native() {
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
                let frame = game.render_dx_frame(&bundle, character);
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
    assert!(!game.dx_supported());
}
