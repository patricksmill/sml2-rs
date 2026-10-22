use sml2_dx::{Bundle, BUNDLE_SCHEMA, ROM_SHA256};
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

fn synthetic_bundle() -> Bundle {
    Bundle {
        schema: BUNDLE_SCHEMA,
        source_sha256: ROM_SHA256.into(),
        background: [vec![0; 0x1000], vec![0; 0x1000]],
        mario_normal: vec![0; 0x800],
        mario_fire: vec![0; 0x800],
        mario_bank1: vec![0; 0x800],
        tile_attributes: vec![0; 256],
        hud_tiles: vec![0; 20],
        hud_attributes: vec![0; 20],
        // Every BG index is literal red; OBJ zero is transparent irrespective of RGB.
        bg_palettes: [0x1f, 0].repeat(32),
        obj_palettes: [0xe0, 3].repeat(32),
        animation: vec![0; 256],
        object_mappings: vec![vec![]; 168],
        player_mappings: vec![vec![]; 242],
    }
}

#[test]
fn dx_public_api_requires_opening_level_and_outputs_direct_rgb() {
    let bundle = synthetic_bundle();
    let mut game = session();
    assert!(game.render_dx_frame(&bundle).is_err());
    game.begin_level(0, 0).unwrap();
    assert_eq!(
        game.render_dx_frame(&bundle).unwrap(),
        vec![[255, 0, 0]; 160 * 144]
    );
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.manifest.level_id = 1;
    assert!(game.render_dx_frame(&bundle).is_err());
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
        let color = displayed.render_dx_frame(&bundle).unwrap();
        assert_eq!(displayed.render_dx_frame(&bundle).unwrap(), color);
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

/// Local acceptance with user-provided ROM; never embeds copyrighted content.
/// Set DX_OUTPUT_IMAGE to write the initial native frame for independent comparison.
#[test]
#[ignore = "requires SML2_DX_ROM with the audited local ROM"]
fn audited_rom_forms_effects_and_initial_frame() {
    let rom = std::fs::read(std::env::var_os("SML2_DX_ROM").expect("set SML2_DX_ROM")).unwrap();
    let bundle = sml2_dx::extract(&rom).unwrap();
    let mut game = session();
    game.begin_level(0, 0).unwrap();
    let initial = game.render_dx_frame(&bundle).unwrap();
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
        let pixels = preview.render_dx_frame(&bundle).unwrap();
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
        let color = form.render_dx_frame(&bundle).unwrap();
        assert_eq!(form.render_dx_frame(&bundle).unwrap(), color);
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
    let pause0 = game.render_dx_frame(&bundle).unwrap();
    let SessionState::InLevel(ctx) = &mut game.state else {
        unreachable!()
    };
    ctx.sim.frame_counter = 16;
    assert_ne!(pause0, game.render_dx_frame(&bundle).unwrap());
    let mut stars = Vec::new();
    for low in [4, 6] {
        let SessionState::InLevel(ctx) = &mut game.state else {
            unreachable!()
        };
        ctx.sim.paused = false;
        ctx.sim.star_high = 1;
        ctx.sim.star_low = low + 1; // advance decrements before staging draw.
        game.advance(FrameInput::default());
        stars.push(game.render_dx_frame(&bundle).unwrap());
    }
    assert_ne!(stars[0], stars[1]);
}
