use super::*;
use eframe::egui::{Context, RawInput};

fn frame(app: &mut DesktopApp, ctx: &Context, size: egui::Vec2, dpi: f32) -> egui::FullOutput {
    let mut raw = RawInput {
        focused: true,
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
        ..Default::default()
    };
    let viewport = raw.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
    viewport.inner_rect = raw.screen_rect;
    viewport.native_pixels_per_point = Some(dpi);
    ctx.run(raw, |ctx| app.ui(ctx))
}

#[test]
fn castle_scenery_fills_the_window_without_resizing_the_scene() {
    let (mut app, ctx, path) = crate::app::tests::app();
    app.game.as_mut().unwrap().session.state = SessionState::Castle(Default::default());
    app.paused = true;
    app.settings.show_toolbar = false;
    let view = app.settings.view();
    let native = app.frame_image(Smooth::OFF).unwrap();
    for (size, dpi, integer_scaling) in [
        (egui::vec2(1051.0, 961.0), 1.0, true),
        (egui::vec2(1051.0, 961.0), 2.0, true),
        (egui::vec2(1051.0, 961.0), 1.0, false),
        (egui::vec2(1051.0, 961.0), 2.0, false),
    ] {
        app.settings.integer_scaling = integer_scaling;
        app.castle_backdrop = None;
        let output = frame(&mut app, &ctx, size, dpi);
        assert_eq!(app.settings.view(), view);
        assert_eq!(app.texture.size(), native.size);
        if integer_scaling {
            assert_eq!(app.game_rect[2], view.0 as u32 * app.effective_scale);
            assert_eq!(app.game_rect[3], view.1 as u32 * app.effective_scale);
        } else {
            assert_eq!(app.game_rect[2], (size.x * dpi) as u32);
            assert_ne!(app.game_rect[2] % view.0 as u32, 0);
        }
        let backdrop = app.castle_backdrop.as_ref().unwrap();
        let id = backdrop.texture.id();
        let expanded = backdrop.key.view;
        let (_, image) = output
            .textures_delta
            .set
            .iter()
            .find(|(texture, _)| *texture == id)
            .unwrap();
        let egui::ImageData::Color(image) = &image.image;
        let at = ((expanded.0 - view.0) / 2, (expanded.1 - view.1) / 2);
        for y in 0..view.1 {
            assert_eq!(
                &image.pixels
                    [(at.1 + y) * expanded.0 + at.0..(at.1 + y) * expanded.0 + at.0 + view.0],
                &native.pixels[y * view.0..(y + 1) * view.0]
            );
        }
        let drawn = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == id => Some(Rect::from_points(
                    &mesh.vertices.iter().map(|v| v.pos).collect::<Vec<_>>(),
                )),
                _ => None,
            })
            .expect("the castle backdrop is painted behind the original picture");
        assert!(drawn.contains(egui::Pos2::ZERO) && drawn.contains(egui::pos2(size.x, size.y)));
        let repeated = frame(&mut app, &ctx, size, dpi);
        assert!(
            !repeated
                .textures_delta
                .set
                .iter()
                .any(|(texture, _)| *texture == id),
            "idle frames reuse the backdrop"
        );
    }
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 0);
    app.game.as_mut().unwrap().session.enter_file_select();
    frame(&mut app, &ctx, egui::vec2(1051.0, 961.0), 1.0);
    assert!(app.castle_backdrop.is_none());
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn castle_surroundings_follow_filter_fade_and_dx_changes() {
    let (mut app, ctx, path) = crate::app::tests::app();
    app.game.as_mut().unwrap().session.state = SessionState::Castle(Default::default());
    app.paused = true;
    app.settings.show_toolbar = false;
    let size = egui::vec2(1051.0, 961.0);
    frame(&mut app, &ctx, size, 1.0);
    app.settings.filter = Filter::Dmg;
    let output = frame(&mut app, &ctx, size, 1.0);
    let backdrop = app.castle_backdrop.as_ref().unwrap();
    assert_eq!(backdrop.key.filter, Filter::Dmg);
    assert!(output
        .textures_delta
        .set
        .iter()
        .any(|(id, _)| *id == backdrop.texture.id()));
    let SessionState::Castle(seq) = &mut app.game.as_mut().unwrap().session.state else {
        panic!("castle")
    };
    seq.bgp = 0xFF;
    frame(&mut app, &ctx, size, 1.0);
    assert_eq!(app.castle_backdrop.as_ref().unwrap().key.bgp, 0xFF);
    let mut dx = crate::app::dx_tests::red_bundle();
    for bank in [0x05, 0x0c, 0x26, 0x27] {
        dx.banks.insert(bank, vec![0; 0x4000]);
    }
    dx.banks.get_mut(&0x26).unwrap()[0x3042..0x3044].copy_from_slice(&[0x1f, 0]);
    app.dx_bundle = Some((dx).into());
    app.settings.dx_mode = true;
    let output = frame(&mut app, &ctx, size, 1.0);
    let backdrop = app.castle_backdrop.as_ref().unwrap();
    assert!(backdrop.key.dx);
    let (_, image) = output
        .textures_delta
        .set
        .iter()
        .find(|(id, _)| *id == backdrop.texture.id())
        .unwrap();
    let egui::ImageData::Color(image) = &image.image;
    assert!(image
        .pixels
        .iter()
        .all(|pixel| *pixel == Color32::from_rgb(255, 0, 0)));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
#[ignore = "exports a castle extension preview to the system temporary directory"]
fn export_extended_castle_preview() {
    let (mut app, _, path) = crate::app::tests::app();
    let mut seq = sml2_core::castle_sequence::CastleSequence::default();
    seq.coins[5] = 0x80;
    app.game.as_mut().unwrap().session.state = SessionState::Castle(seq);
    app.settings.filter = Filter::Gbc;
    if let Some(rom) = std::env::var_os("SML2_DX_ROM") {
        let bundle = sml2_dx::extract(&std::fs::read(rom).unwrap()).unwrap();
        app.dx_bundle = Some((sml2_dx::Presentation::new(&bundle).unwrap()).into());
        app.settings.dx_mode = true;
    }
    let image = app.frame_image_with_view((256, 192), Smooth::OFF).unwrap();
    let scaled = eframe::egui::ColorImage::new(
        [1024, 768],
        (0..768)
            .flat_map(|y| {
                let image = &image;
                (0..1024).map(move |x| image.pixels[(y / 4) * 256 + x / 4])
            })
            .collect(),
    );
    crate::app::screenshot::save(
        &std::env::temp_dir().join("sml2-extended-castle.png"),
        &scaled,
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(path);
}
