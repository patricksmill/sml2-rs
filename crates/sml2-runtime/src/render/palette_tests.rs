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
    session.state = SessionState::GameOver;
    let fallback = assert_gray_parity(&session);
    assert!(fallback.iter().all(|p| *p
        == PalettePixel {
            source: PaletteSource::Bg,
            shade: 3
        }));
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
