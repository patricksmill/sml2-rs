use crate::session::{FileSelectLoading, FrameInput, Session, SessionAssets, SessionState};
use sml2_core::object::MappingEntry;
use sml2_core::objects::BossOamCell;
use sml2_core::video::{PalettePixel, PaletteSource};

fn assert_gray_parity(session: &Session) -> Vec<PalettePixel> {
    let pixels = session.render_palette_frame((160, 144));
    assert_eq!(pixels.len(), 160 * 144);
    assert!(pixels.iter().all(|p| p.shade < 4));
    assert_eq!(
        pixels.iter().map(|p| p.grayscale()).collect::<Vec<_>>(),
        session.render_frame((160, 144))
    );
    assert_eq!(
        session.render_palette_frame((160, 144)),
        pixels,
        "read-only repeated rendering"
    );
    pixels
}

#[test]
fn session_palette_producer_reaches_title_file_select_all_maps_rooms_and_castle() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    for screen in 0..=14 {
        if let SessionState::Overworld(map) = &mut session.state {
            map.load_screen(screen, 0, &session.assets.node_records);
        }
        assert_gray_parity(&session);
    }
    session.begin_castle_sequence(Default::default(), 0, 0, 0);
    assert_gray_parity(&session);
    session.enter_title();
    let title = assert_gray_parity(&session);
    assert!(title.iter().all(|p| p.source == PaletteSource::Bg));
    session.state = SessionState::FileSelectLoading(FileSelectLoading::new());
    assert_eq!(
        assert_gray_parity(&session),
        title,
        "pending load retains title"
    );
    session.advance(FrameInput::default());
    let blank = assert_gray_parity(&session);
    assert!(
        blank.iter().all(|p| *p
            == PalettePixel {
                source: PaletteSource::Bg,
                shade: 0
            }),
        "LCD blank resets provenance"
    );
    session.enter_file_select();
    for _ in 0..48 {
        session.advance(FrameInput::default());
    }
    let file_select = assert_gray_parity(&session);
    assert!(file_select.iter().any(|p| p.source == PaletteSource::Obj0));
    let mut sim = sml2_core::player_sim::PlayerSim::new();
    sim.game_over_enter();
    session.state = SessionState::GameOver(Box::new(crate::session::GameOverScreen {
        sim,
        obp: [0xD0, 0xE4],
    }));
    let game_over = assert_gray_parity(&session);
    assert!(game_over.iter().any(|p| p.source == PaletteSource::Bg));
    assert!(
        game_over.iter().any(|p| p.source == PaletteSource::Obj0),
        "Mario is drawn over the castle"
    );
}

#[test]
fn session_palette_level_composes_player_objects_boss_hud_priority_and_resets_sources() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        unreachable!()
    };
    ctx.cam.x = 80;
    ctx.cam.y = 72;
    ctx.manifest.header.palettes[0] = 0xE4;
    ctx.layout_live.cells.fill(0);
    ctx.manifest.metatiles.metatiles[0].tiles = [0; 4];
    ctx.sim.layout_writes.clear();
    ctx.bg_live.fill(0);
    for y in 0..8 {
        ctx.bg_live[0x800 + y * 2] = 255; // BG tile 0 = colour 1
        ctx.bg_live[0x810 + y * 2 + 1] = 255; // HUD tile 1 = colour 2
    }
    ctx.hud.template = vec![1; 20];
    ctx.hud.digit_columns.clear();
    ctx.hud_y = 136;
    let spr = ctx.spr.as_mut().unwrap();
    spr.pipe_background = None;
    spr.insts.clear();
    spr.player = Some((0, 0, 0));
    spr.player_shield = None;
    spr.streak_counter = None;
    spr.bumpers.clear();
    spr.fireballs.clear();
    spr.player_behind_bg = false;
    spr.player_star_flash = false;
    spr.player_pan = 0;
    spr.scroll_pan = 0;
    spr.obp0 = 0xE4;
    // OBP1 raw colour 1 maps to shade 1, demonstrating identical shades
    // with different provenance, while colour 2 maps to shade 0.
    spr.obp1 = 0xC4;
    spr.objblock.fill(0);
    for y in 0..8 {
        spr.objblock[y * 2] = 255;
        spr.objblock[32 + y * 2 + 1] = 255; // tile 2 = colour 2
    }
    spr.tables.player_mappings[0] = vec![MappingEntry {
        x: -8,
        y: -10,
        tile: 0,
        attr: 0,
    }];
    spr.tables.sprite_mappings_a[0] = vec![MappingEntry {
        x: 0,
        y: 0,
        tile: 0,
        attr: 0,
    }];
    spr.insts.push(crate::render::SpriteInst {
        sprite: 0,
        x: 32,
        y: 0,
        flip: 0x10,
        slot: 0,
    });
    spr.boss_oam = vec![
        BossOamCell {
            x: 72,
            y: 16,
            tile: 0,
            attr: 0x10,
        },
        BossOamCell {
            x: 88,
            y: 16,
            tile: 1,
            attr: 0x10,
        }, // transparent
        BossOamCell {
            x: 104,
            y: 16,
            tile: 0,
            attr: 0x90,
        }, // behind BG
        BossOamCell {
            x: 128,
            y: 16,
            tile: 2,
            attr: 0x10,
        },
        BossOamCell {
            x: 8,
            y: 152,
            tile: 0,
            attr: 0x90,
        }, // behind HUD
    ];
    let pixels = assert_gray_parity(&session);
    for (at, source, shade) in [
        (0, PaletteSource::Obj0, 1),
        (32, PaletteSource::Obj1, 1),
        (64, PaletteSource::Obj1, 1),
        (80, PaletteSource::Bg, 1),
        (96, PaletteSource::Bg, 1),
        (112, PaletteSource::Bg, 1),
        (120, PaletteSource::Obj1, 0),
        (136 * 160, PaletteSource::Bg, 2),
    ] {
        assert_eq!(pixels[at], PalettePixel { source, shade }, "pixel {at}");
    }
    let SessionState::InLevel(ctx) = &mut session.state else {
        unreachable!()
    };
    let spr = ctx.spr.as_mut().unwrap();
    spr.player_star_flash = true;
    spr.insts.clear();
    spr.boss_oam.clear();
    let pixels = assert_gray_parity(&session);
    assert_eq!(
        pixels[0].source,
        PaletteSource::Obj1,
        "star flash selects second OBJ palette"
    );
    assert_eq!(
        pixels[32].source,
        PaletteSource::Bg,
        "source resets after object removal"
    );
    assert_eq!(
        pixels[64].source,
        PaletteSource::Bg,
        "source resets after boss removal"
    );
}

#[test]
fn smoothing_at_a_full_tick_is_the_native_frame_upsampled() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    for _ in 0..30 {
        session.advance(FrameInput::default());
    }
    let native = session.render_palette_frame((256, 144));
    let (smooth, scale) = session.render_palette_frame_smooth(
        (256, 144),
        crate::render::motion::Smooth {
            alpha: 1.0,
            scale: 2,
        },
    );
    assert_eq!(scale, 2);
    let upsampled: Vec<_> = (0..288)
        .flat_map(|y| (0..512).map(move |x| (x, y)))
        .map(|(x, y)| native[(y / 2) * 256 + x / 2])
        .collect();
    assert_eq!(smooth, upsampled);
}

#[test]
fn smoothing_shifts_the_level_but_not_the_status_bar() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    for _ in 0..30 {
        session.advance(FrameInput::default());
    }
    let mut prev = session
        .level_ctx()
        .and_then(crate::render::motion::MotionSnapshot::of)
        .unwrap();
    prev.cam.0 = prev.cam.0.wrapping_sub(4);
    session.motion_prev = Some(prev);
    let half = crate::render::motion::Smooth {
        alpha: 0.5,
        scale: 1,
    };
    let (shifted, _) = session.render_palette_frame_smooth((160, 144), half);
    let native = session.render_palette_frame((160, 144));
    assert_ne!(shifted[..160 * 136], native[..160 * 136]);
    // Row 0 is pure background: half of a 4 px camera step moves it 2 px right.
    assert_eq!(shifted[2..160], native[..158]);
    assert!(
        native[..158].windows(2).any(|w| w[0] != w[1]),
        "row 0 has detail"
    );
    assert_eq!(shifted[160 * 136..], native[160 * 136..]);
}

#[test]
fn smoothing_moves_mario_by_his_share_of_the_tick() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    for _ in 0..30 {
        session.advance(FrameInput::default());
    }
    let mut prev = session
        .level_ctx()
        .and_then(crate::render::motion::MotionSnapshot::of)
        .unwrap();
    let mario = prev.player.as_mut().unwrap();
    mario.0 = mario.0.wrapping_sub(2);
    session.motion_prev = Some(prev);
    let half = crate::render::motion::Smooth {
        alpha: 0.5,
        scale: 1,
    };
    let (smoothed, _) = session.render_palette_frame_smooth((160, 144), half);
    assert_ne!(smoothed, session.render_palette_frame((160, 144)));
    // Halfway through a 2 px step Mario is 1 px behind where the tick drew him.
    let crate::session::SessionState::InLevel(ctx) = &mut session.state else {
        panic!("in a level");
    };
    let player = ctx.spr.as_mut().unwrap().player.as_mut().unwrap();
    player.0 = player.0.wrapping_sub(1);
    assert_eq!(smoothed, session.render_palette_frame((160, 144)));
}

#[test]
fn corner_hud_moves_the_status_bar_to_the_top_left_without_its_backing() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        unreachable!()
    };
    ctx.manifest.header.palettes[0] = 0xE4;
    ctx.layout_live.cells.fill(0);
    ctx.manifest.metatiles.metatiles[0].tiles = [0; 4];
    ctx.sim.layout_writes.clear();
    ctx.bg_live.fill(0);
    for y in 0..8 {
        ctx.bg_live[0x800 + y * 2] = 255; // BG tile 0 = colour 1
        ctx.bg_live[0x810 + y * 2 + 1] = 0x18; // HUD tile 1: a colour-2 bar at x 3-4
    }
    ctx.hud.template = vec![1; 20];
    ctx.hud.digit_columns.clear();
    ctx.spr = None;
    let shade = |frame: &[u8], x: usize, y: usize| frame[y * 160 + x];
    let classic = session.render_frame((160, 144));
    assert_eq!(shade(&classic, 3, 140), 170 - 85, "classic strip ink");
    assert_eq!(shade(&classic, 0, 140), 255, "classic strip paper");
    session.hud_style = crate::render::HudStyle::Corner;
    let corner = session.render_frame((160, 144));
    assert!(
        (136..144).all(|y| (0..160).all(|x| shade(&corner, x, y) == 170)),
        "the bottom strip shows the level"
    );
    // Column 0's tile sits at (8, 8): its ink, the paper ring beside it,
    // and level background where colour 0 is not beside ink.
    assert_eq!(shade(&corner, 8 + 3, 8), 85);
    assert_eq!(shade(&corner, 8 + 2, 8), 255);
    assert_eq!(shade(&corner, 8, 8), 170);
    // The strip's spacer column 4 is not drawn: tile x 4 holds nothing.
    assert!((8..16).all(|y| shade(&corner, 8 + 4 * 8 + 3, y) == 170));
    // The timer's last tile ends 8 pixels from the right edge.
    assert_eq!(shade(&corner, 160 - 16 + 3, 8), 85);
    assert!((17..24).all(|y| shade(&corner, 160 - 16 + 3, y) == 170), "one row, under its ring");
    let wide = session.render_frame((256, 144));
    assert_eq!(wide[8 * 256 + 256 - 16 + 3], 85, "the timer follows a wider view's edge");
    let mut real = Session::new(SessionAssets::load().unwrap());
    real.begin_level(1, 0).unwrap();
    real.hud_style = crate::render::HudStyle::Classic;
    let classic = real.render_frame((160, 144));
    real.hud_style = crate::render::HudStyle::Corner;
    let corner = real.render_frame((160, 144));
    // Terrain and sprites move down together, hiding the rows the strip covered.
    assert_eq!(corner[32 * 160..], classic[24 * 160..136 * 160]);
    if let Ok(dir) = std::env::var("SML2_HUD_PNG") {
        for style in [crate::render::HudStyle::Classic, crate::render::HudStyle::Corner] {
            real.hud_style = style;
            for size in [(160, 144), (256, 144)] {
                let frame = real.render_frame(size);
                crate::image_output::write_png_scaled(
                    &std::path::Path::new(&dir).join(format!("{style:?}_{}.png", size.0)),
                    size.0,
                    size.1,
                    4,
                    &frame,
                )
                .unwrap();
            }
        }
    }
}
