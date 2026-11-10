use crate::app::editor::palette::{dx_tile, native_tile};
use crate::app::{colorized, tests::app, tests::red_bundle};
use crate::settings::Filter;
use eframe::egui::Color32;
use sml2_core::camera::Room;
use sml2_core::scenery::{SceneryOverlay, SceneryPaint};
use sml2_runtime::render::{motion::Smooth, view};
use sml2_runtime::session::{Session, SessionState};

/// The previewed level camera's room, which holds an enlarged view.
fn room(session: &Session) -> Room {
    match &session.state {
        SessionState::InLevel(ctx) => ctx.cam.room(),
        _ => Room::WORLD,
    }
}

#[test]
fn enabling_dx_after_opening_the_editor_restores_and_refreshes_its_graphics() {
    use crate::app::{Screen, tests::frame};
    use crate::dx_import::{Event, Worker};
    use sml2_dx::cache::CacheKey;
    use std::sync::{Arc, atomic::AtomicBool, mpsc};

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
    assert!(
        ctx.tessellate(output.shapes, output.pixels_per_point)
            .iter()
            .any(|shape| matches!(
                &shape.primitive,
                eframe::egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id == dx_atlas
            ))
    );

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
    assert!(
        image
            .pixels
            .iter()
            .all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b())
    );
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
        let origin = view::level_origin(size, (80, 448), 0, room(session));
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
    let origin = view::level_origin(size, (80, 448), 0, room(session));
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
    let palettes = sml2_core::video::cgb::CgbPalettes::from_bytes(
        &graphics.bg_palettes,
        &graphics.obj_palettes,
    )
    .unwrap();
    let expected = dx_tile(graphics, 1, &palettes).unwrap();
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
    let origin = view::level_origin(size, (80, 448), 0, room(session));
    for y in 0..16 {
        for x in 0..16 {
            let (px, py) = (x as i32 - origin.0, 384 + y as i32 - origin.1);
            let [r, g, b] = frame[py as usize * size.0 + px as usize];
            assert_eq!(expected[y * 16 + x], Color32::from_rgb(r, g, b));
        }
    }
    std::fs::remove_dir_all(path).ok();
}

/// Drive the visible control rather than changing palette state behind the UI.
fn click_palette_control(
    app: &mut crate::app::EditorApp,
    ctx: &eframe::egui::Context,
    label: &str,
) -> eframe::egui::FullOutput {
    use crate::app::tests::frame;
    use eframe::egui::{Event, Modifiers, PointerButton, Shape};
    let output = frame(app, ctx, vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing palette control: {label}"));
    frame(
        app,
        ctx,
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        app,
        ctx,
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ],
    )
}

#[test]
fn regional_palette_preview_updates_visible_tiles_and_resets() {
    use crate::app::tests::frame;
    let (mut app, ctx, path) = app();
    app.settings.dx_mode = true;
    let mut dx = red_bundle();
    let mut bank = vec![0; 0x4000];
    bank[0x20c0..0x2100].copy_from_slice(&[0xe0, 3].repeat(32));
    dx.banks.insert(0x26, bank);
    app.dx_bundle = Some(dx);
    app.open_editor(&ctx);
    app.select_editor_level(21);
    for _ in 0..2 {
        super::workspace_ui::frame(&mut app, &ctx, (480.0, 400.0), vec![]);
    }
    assert!(app.editor.canvas_rect.unwrap().height() >= 48.0);
    app.editor.work.as_mut().unwrap().viewport.center = eframe::egui::pos2(2360.0, 544.0);
    frame(&mut app, &ctx, vec![]);
    for (label, color) in [
        ("Preview at view center", Color32::GREEN),
        ("Reset palette", Color32::RED),
    ] {
        let output = click_palette_control(&mut app, &ctx, label);
        let texture = app.editor.atlas.as_ref().unwrap().texture.id();
        let (_, delta) = output
            .textures_delta
            .set
            .iter()
            .find(|(id, _)| *id == texture)
            .unwrap();
        let eframe::egui::ImageData::Color(image) = &delta.image;
        assert!(image.pixels.iter().all(|pixel| *pixel == color), "{label}");
        if color == Color32::GREEN {
            // A non-trigger retains the selected palette rather than returning to the starting set.
            app.editor.work.as_mut().unwrap().viewport.center = eframe::egui::pos2(800.0, 544.0);
            click_palette_control(&mut app, &ctx, "Preview at view center");
            let output = frame(&mut app, &ctx, vec![]);
            assert_eq!(app.editor.atlas.as_ref().unwrap().texture.id(), texture);
            assert!(
                !output
                    .textures_delta
                    .set
                    .iter()
                    .any(|(id, _)| *id == texture)
            );
        }
    }
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn palette_preview_does_not_change_exports_or_playtest_palette_transitions() {
    use crate::app::tests::frame;
    let (mut app, ctx, path) = app();
    app.settings.dx_mode = true;
    let mut dx = red_bundle();
    let mut bank = vec![0; 0x4000];
    bank[0x20c0..0x2100].copy_from_slice(&[0xe0, 3].repeat(32));
    dx.banks.insert(0x26, bank);
    app.dx_bundle = Some(dx);
    app.open_editor(&ctx);
    app.select_editor_level(21);
    let work = app.editor.work.as_mut().unwrap();
    work.scenery = SceneryOverlay::try_from_paints(
        vec![SceneryPaint {
            x: 0,
            y: 24,
            metatile: 1,
        }],
        128,
    )
    .unwrap();
    work.viewport.center = eframe::egui::pos2(2360.0, 544.0);
    app.begin_export();
    let before = path.join("before.json");
    app.export_to(&before).unwrap();
    app.editor.export = None;
    frame(&mut app, &ctx, vec![]);
    click_palette_control(&mut app, &ctx, "Preview at view center");
    app.begin_export();
    let after = path.join("after.json");
    app.export_to(&after).unwrap();
    assert_eq!(
        std::fs::read(&before).unwrap(),
        std::fs::read(&after).unwrap()
    );
    app.editor.export = None;
    app.play_editor(&ctx);
    let session = &mut app.game.as_mut().unwrap().session;
    let scenery = session.assets.level(21).unwrap().scenery.clone();
    let size = (384, 216);
    for expected in [[255, 0, 0], [0, 255, 0]] {
        session.preview_scenery((80, 448), scenery.clone());
        let origin = view::level_origin(size, (80, 448), 0, room(session));
        let pixels = session
            .render_dx_frame(
                app.dx_bundle.as_ref().unwrap(),
                app.settings.character,
                size,
            )
            .unwrap();
        let at = (384 - origin.1) as usize * size.0 + (-origin.0) as usize;
        assert_eq!(
            pixels[at], expected,
            "preview selection must not override gameplay"
        );
        let SessionState::InLevel(level) = &mut session.state else {
            unreachable!()
        };
        level.obj.mario_ref_x = 0x0940;
        level.obj.mario_ref_y = 0x0240;
        session.advance(sml2_runtime::session::FrameInput::default());
    }
    std::fs::remove_dir_all(path).ok();
}
