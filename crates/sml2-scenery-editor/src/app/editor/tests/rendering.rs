use crate::app::editor::palette::{dx_tile, native_tile};
use crate::app::{colorized, tests::red_bundle, tests::app};
use crate::settings::Filter;
use eframe::egui::Color32;
use sml2_core::scenery::{SceneryOverlay, SceneryPaint};
use sml2_runtime::render::{motion::Smooth, view};

#[test]
fn enabling_dx_after_opening_the_editor_restores_and_refreshes_its_graphics() {
    use crate::app::{tests::frame, Screen};
    use crate::dx_import::{Event, Worker};
    use sml2_dx::cache::CacheKey;
    use std::sync::{atomic::AtomicBool, mpsc, Arc};

    let (mut app, ctx, path) = app();
    app.settings.filter = Filter::Gray;
    app.open_editor(&ctx);
    frame(&mut app, &ctx, vec![]);
    let native_atlas = app.editor.atlas.as_ref().unwrap().texture.id();
    app.navigate(&ctx, Screen::Home);

    let key = CacheKey::new("1".repeat(64));
    app.settings.dx_installation = Some(key.clone());
    app.settings.dx_mode = true;
    app.open_editor(&ctx);
    assert!(app.dx_restore, "reopening must restore enabled DX graphics");
    assert!(app.dx_worker.is_some());
    assert_eq!(app.screen, Screen::Editor);

    // Complete the restore deterministically with synthetic colored graphics.
    let (sender, events) = mpsc::channel();
    app.dx_worker = Some(Worker {
        events,
        cancelled: Arc::new(AtomicBool::new(false)),
    });
    sender
        .send(Event::Finished(Ok((key, red_bundle()))))
        .unwrap();
    let output = frame(&mut app, &ctx, vec![]);
    assert!(app.settings.dx_mode);
    assert!(app.dx_worker.is_none());
    let dx_atlas = app.editor.atlas.as_ref().unwrap().texture.id();
    assert_ne!(dx_atlas, native_atlas);
    let (_, delta) = output
        .textures_delta
        .set
        .iter()
        .find(|(id, _)| *id == dx_atlas)
        .expect("the editor must upload its refreshed DX atlas");
    let eframe::egui::ImageData::Color(image) = &delta.image;
    assert!(image.pixels.iter().all(|pixel| *pixel == Color32::RED));
    assert!(ctx
        .tessellate(output.shapes, output.pixels_per_point)
        .iter()
        .any(|shape| matches!(
            &shape.primitive,
            eframe::egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id == dx_atlas
        )));

    app.settings.dx_mode = false;
    let output = frame(&mut app, &ctx, vec![]);
    let native_atlas = app.editor.atlas.as_ref().unwrap().texture.id();
    assert_ne!(native_atlas, dx_atlas);
    let (_, delta) = output
        .textures_delta
        .set
        .iter()
        .find(|(id, _)| *id == native_atlas)
        .unwrap();
    let eframe::egui::ImageData::Color(image) = &delta.image;
    assert!(image
        .pixels
        .iter()
        .all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b()));
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn live_block_changes_override_paints_in_native_and_dx_at_both_view_sizes() {
    let (mut app, _, path) = app();
    let session = &mut app.game.as_mut().unwrap().session;
    session.begin_level(0, 0).unwrap();
    let mut dx = red_bundle();
    let sml2_runtime::session::SessionState::InLevel(level) = &mut session.state else {
        unreachable!()
    };
    level.bg_live.fill(0);
    level.manifest.header.palettes[0] = 0xe4;
    for (id, tile) in [(0, 0), (1, 1), (0x60, 2), (7, 3)] {
        level.manifest.metatiles.metatiles[id].tiles = [tile; 4];
        dx.levels[0].metatiles.metatiles[id].tiles = [tile; 4];
    }
    for (tile, color) in [(1, 1), (2, 2), (3, 3)] {
        let offset = sml2_core::video::bg_tile_offset(tile);
        for y in 0..8 {
            for plane in 0..2 {
                let value = if color & (1 << plane) != 0 { 255 } else { 0 };
                level.bg_live[offset + y * 2 + plane] = value;
                dx.levels[0].background[0][offset + y * 2 + plane] = value;
            }
        }
    }
    dx.levels[0].bg_palettes = [0, 0, 0x1f, 0, 0xe0, 3, 0, 0x7c].repeat(8);
    let scenery = SceneryOverlay::try_from_paints(
        vec![
            SceneryPaint {
                x: 0,
                y: 24,
                metatile: 1,
            },
            SceneryPaint {
                x: 1,
                y: 24,
                metatile: 1,
            },
        ],
        128,
    )
    .unwrap();
    session.preview_scenery((80, 448), scenery.clone());
    let character = crate::settings::Settings::default().character;
    for size in [(160, 144), (384, 216)] {
        let origin = view::level_origin(size, (80, 448), 0);
        let cell_pixels = |frame: &[[u8; 3]], cell_x: i32| {
            let mut pixels = Vec::new();
            for y in 0..16 {
                for x in 0..16 {
                    pixels.push(
                        frame[(384 + y - origin.1) as usize * size.0
                            + (cell_x * 16 + x - origin.0) as usize],
                    );
                }
            }
            pixels
        };
        for changed in [24, 0x60, 7] {
            // temporary bump blank, removed coin, replacement block
            let sml2_runtime::session::SessionState::InLevel(level) = &mut session.state else {
                unreachable!()
            };
            level.sim.layout_writes = vec![(24 * 256, changed)];
            level.scenery = SceneryOverlay::empty();
            let plain_native = session
                .render_frame(size)
                .into_iter()
                .map(|v| [v; 3])
                .collect::<Vec<_>>();
            let plain_dx = session.render_dx_frame(&dx, character, size).unwrap();
            let sml2_runtime::session::SessionState::InLevel(level) = &mut session.state else {
                unreachable!()
            };
            level.scenery = scenery.clone();
            let painted_native = session
                .render_frame(size)
                .into_iter()
                .map(|v| [v; 3])
                .collect::<Vec<_>>();
            let painted_dx = session.render_dx_frame(&dx, character, size).unwrap();
            assert_eq!(
                cell_pixels(&painted_native, 0),
                cell_pixels(&plain_native, 0),
                "native {size:?} tile {changed}"
            );
            assert_eq!(
                cell_pixels(&painted_dx, 0),
                cell_pixels(&plain_dx, 0),
                "DX {size:?} tile {changed}"
            );
            let (native_color, dx_color) = match changed {
                24 => ([255; 3], [0; 3]),
                0x60 => ([85; 3], [0, 255, 0]),
                7 => ([0; 3], [0, 0, 255]),
                _ => unreachable!(),
            };
            assert_eq!(cell_pixels(&painted_native, 0), vec![native_color; 256]);
            assert_eq!(cell_pixels(&painted_dx, 0), vec![dx_color; 256]);
            assert_ne!(
                cell_pixels(&painted_native, 0),
                cell_pixels(&painted_native, 1)
            );
            assert_ne!(cell_pixels(&painted_dx, 0), cell_pixels(&painted_dx, 1));
            let sml2_runtime::session::SessionState::InLevel(level) = &session.state else {
                unreachable!()
            };
            assert_eq!(level.scenery, scenery, "play must retain authoring data");
        }
    }
    std::fs::remove_dir_all(path).ok();
}

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
                x: 0,
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
    let origin = view::level_origin(size, (80, 448), 0);
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
                let (px, py) = (x as i32 - origin.0, 384 + y as i32 - origin.1);
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
                x: 0,
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
    let origin = view::level_origin(size, (80, 448), 0);
    for y in 0..16 {
        for x in 0..16 {
            let (px, py) = (x as i32 - origin.0, 384 + y as i32 - origin.1);
            let [r, g, b] = frame[py as usize * size.0 + px as usize];
            assert_eq!(expected[y * 16 + x], Color32::from_rgb(r, g, b));
        }
    }
    std::fs::remove_dir_all(path).ok();
}
