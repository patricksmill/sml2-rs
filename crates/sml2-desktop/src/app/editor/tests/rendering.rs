use crate::app::editor::palette::{dx_tile, native_tile};
use crate::app::{colorized, dx_tests::red_bundle, tests::app};
use crate::settings::Filter;
use eframe::egui::Color32;
use sml2_core::scenery::{SceneryOverlay, SceneryPaint};
use sml2_runtime::render::{motion::Smooth, view};

#[test]
fn scenery_inside_map_appears_at_native_size_in_native_and_dx_without_layout_edits() {
    let (mut app, _, path) = app();
    let session = &mut app.game.as_mut().unwrap().session;
    session.begin_level(0, 0).unwrap();
    let original = session.assets.level(0).unwrap().layout.cells.clone();
    session.preview_scenery(
        (80, 448),
        SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: 0,
                y: 24,
                metatile: 1,
            }],
            128,
        )
        .unwrap(),
    );
    let content = session.assets.level(0).unwrap();
    let expected = native_tile(
        &content.bg_base,
        content.manifest.header.palettes[0],
        content.manifest.metatiles.get(1).unwrap(),
        Filter::Gray,
        &session.assets.cartridge_header,
    )
    .unwrap();
    let native = colorized(
        &session.render_palette_frame((160, 144)),
        Filter::Gray,
        &session.assets.cartridge_header,
        (160, 144),
    );
    let dx = red_bundle();
    let colored = session
        .render_dx_frame(
            &dx,
            crate::settings::Settings::default().character,
            (160, 144),
        )
        .unwrap();
    for y in 0..16 {
        for x in 0..16 {
            assert_eq!(native.pixels[(y + 8) * 160 + x], expected[y * 16 + x]);
            assert_eq!(colored[(y + 8) * 160 + x], [255, 0, 0]);
        }
    }
    assert_eq!(session.assets.level(0).unwrap().layout.cells, original);
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn native_atlas_pixels_match_rendered_scenery_with_every_display_filter() {
    let (mut app, _, path) = app();
    let session = &mut app.game.as_mut().unwrap().session;
    session.begin_level(0, 0).unwrap();
    session.preview_scenery(
        (80, 448),
        SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: -1,
                y: 24,
                metatile: 1,
            }],
            128,
        )
        .unwrap(),
    );
    let content = session.assets.level(0).unwrap();
    let size = (384, 216);
    let frame = session.render_palette_frame_smooth(size, Smooth::OFF).0;
    let origin = (
        80 - size.0 as i32 / 2,
        448 - size.1 as i32 / 2 - view::level_bias(size),
    );
    for filter in Filter::ALL {
        let tile = native_tile(
            &content.bg_base,
            content.manifest.header.palettes[0],
            content.manifest.metatiles.get(1).unwrap(),
            filter,
            &session.assets.cartridge_header,
        )
        .unwrap();
        let image = colorized(&frame, filter, &session.assets.cartridge_header, size);
        for y in 0..16 {
            for x in 0..16 {
                let (px, py) = (-16 + x as i32 - origin.0, 384 + y as i32 - origin.1);
                assert_eq!(
                    tile[y * 16 + x],
                    image.pixels[py as usize * size.0 + px as usize],
                    "{filter:?}, {x},{y}"
                );
            }
        }
    }
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn dx_atlas_samples_banks_flips_and_palettes_like_the_scenery_renderer() {
    let (mut app, _, path) = app();
    let mut dx = red_bundle();
    let graphics = &mut dx.levels[0];
    graphics.metatiles.metatiles[1].tiles = [128; 4];
    graphics.tile_attributes[128] = 0x6b; // palette 3, bank 1, both flips
    graphics.background[1][0..2].copy_from_slice(&[0x80, 0x01]);
    graphics.background[1][14..16].copy_from_slice(&[0x01, 0x01]);
    graphics.bg_palettes[24..32].copy_from_slice(&[0, 0, 0xe0, 3, 0, 0x7c, 0xff, 0x7f]);
    let expected = dx_tile(graphics, 1).unwrap();
    assert_eq!(expected[0], Color32::WHITE);
    assert_eq!(expected[7 * 16 + 7], Color32::from_rgb(0, 255, 0));
    assert_eq!(expected[7 * 16], Color32::from_rgb(0, 0, 255));
    let session = &mut app.game.as_mut().unwrap().session;
    session.begin_level(0, 0).unwrap();
    session.preview_scenery(
        (80, 448),
        SceneryOverlay::try_from_paints(
            vec![SceneryPaint {
                x: -1,
                y: 24,
                metatile: 1,
            }],
            128,
        )
        .unwrap(),
    );
    let size = (384, 216);
    let frame = session
        .render_dx_frame(&dx, crate::settings::Settings::default().character, size)
        .unwrap();
    let origin = (
        80 - size.0 as i32 / 2,
        448 - size.1 as i32 / 2 - view::level_bias(size),
    );
    for y in 0..16 {
        for x in 0..16 {
            let (px, py) = (-16 + x as i32 - origin.0, 384 + y as i32 - origin.1);
            let [r, g, b] = frame[py as usize * size.0 + px as usize];
            assert_eq!(expected[y * 16 + x], Color32::from_rgb(r, g, b));
        }
    }
    std::fs::remove_dir_all(path).ok();
}
