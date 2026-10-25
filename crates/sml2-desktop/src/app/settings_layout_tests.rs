use super::{tests::app, *};

fn frame(
    app: &mut DesktopApp,
    ctx: &Context,
    size: egui::Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut raw = RawInput {
        focused: true,
        time: Some(ctx.input(|input| input.time) + 0.25),
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        events,
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .inner_rect = raw.screen_rect;
    ctx.run(raw, |ctx| app.ui(ctx))
}

fn text_rect(output: &egui::FullOutput, label: &str) -> egui::Rect {
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some((text.visual_bounding_rect(), shape.clip_rect))
            }
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 1, "one visible copy of {label}");
    let (rect, clip) = texts[0];
    assert!(clip.contains_rect(rect), "{label} must not be clipped");
    rect
}

fn settle(app: &mut DesktopApp, ctx: &Context, size: egui::Vec2) -> egui::FullOutput {
    frame(app, ctx, size, vec![]);
    frame(app, ctx, size, vec![]);
    frame(app, ctx, size, vec![])
}

fn click(app: &mut DesktopApp, ctx: &Context, size: egui::Vec2, pos: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            app,
            ctx,
            size,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}

#[test]
fn settings_dialog_tabs_and_footer_stay_fixed_across_pages_and_window_sizes() {
    let (mut app, ctx, path) = app();
    app.navigate(&ctx, Screen::Settings);
    for size in [
        egui::vec2(480.0, 400.0),
        egui::vec2(800.0, 720.0),
        egui::vec2(1600.0, 1000.0),
    ] {
        let mut expected = None;
        for page in Page::ALL {
            app.page = page;
            let output = settle(&mut app, &ctx, size);
            let window = ctx
                .memory(|memory| memory.area_rect(egui::Id::new("settings-popup")))
                .unwrap();
            assert!(ctx.available_rect().contains_rect(window));
            let tabs = Page::ALL.map(|page| text_rect(&output, page.label()));
            let back = text_rect(&output, "Back");
            text_rect(&output, "Esc: back · PgUp/PgDn or LB/RB: page");
            let geometry = (window, tabs, back);
            if let Some(expected) = expected {
                assert_eq!(
                    geometry, expected,
                    "settings geometry at {size:?}, {page:?}"
                );
            } else {
                expected = Some(geometry);
            }
        }
    }
    std::fs::remove_dir_all(path).ok();
}

#[test]
fn scrolled_settings_keep_tabs_and_back_reachable_by_mouse_and_controller() {
    let (mut app, ctx, path) = app();
    let size = egui::vec2(480.0, 400.0);
    for controller in [false, true] {
        app.return_screen = Screen::Game;
        app.page = Page::Display;
        app.navigate(&ctx, Screen::Settings);
        let output = settle(&mut app, &ctx, size);
        let display_tab = text_rect(&output, "DISPLAY");
        let controls_tab = text_rect(&output, "CONTROLS");
        let back = text_rect(&output, "Back");
        frame(
            &mut app,
            &ctx,
            size,
            vec![
                egui::Event::PointerMoved(egui::pos2(240.0, 200.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -1000.0),
                    modifiers: Default::default(),
                },
            ],
        );
        let output = settle(&mut app, &ctx, size);
        assert_eq!(text_rect(&output, "DISPLAY"), display_tab);
        assert_eq!(text_rect(&output, "Back"), back);
        assert!(
            !output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Text(text) if text.galley.text() == "Fit to window"
            )),
            "the page content must actually scroll"
        );

        click(&mut app, &ctx, size, controls_tab.center());
        assert_eq!(app.page, Page::Controls);
        let output = settle(&mut app, &ctx, size);
        text_rect(&output, "CONTROLLER");
        assert_eq!(text_rect(&output, "Back"), back);

        if controller {
            // Traverse the bindings: the selected row must scroll into view,
            // while the tabs and footer stay outside the scrolling region.
            for _ in 0..10 {
                app.input
                    .event(Button::Pad(crate::input::PadButton::Down), true);
                frame(&mut app, &ctx, size, vec![]);
                app.input
                    .event(Button::Pad(crate::input::PadButton::Down), false);
                frame(&mut app, &ctx, size, vec![]);
            }
            let output = settle(&mut app, &ctx, size);
            text_rect(&output, "Game Select");
            assert_eq!(text_rect(&output, "DISPLAY"), display_tab);
            assert_eq!(text_rect(&output, "Back"), back);
            // Reset controls follows the bindings, then Back.
            for _ in 0..2 {
                app.input
                    .event(Button::Pad(crate::input::PadButton::Down), true);
                frame(&mut app, &ctx, size, vec![]);
                app.input
                    .event(Button::Pad(crate::input::PadButton::Down), false);
                frame(&mut app, &ctx, size, vec![]);
            }
            app.input
                .event(Button::Pad(crate::input::PadButton::South), true);
            frame(&mut app, &ctx, size, vec![]);
            app.input
                .event(Button::Pad(crate::input::PadButton::South), false);
        } else {
            click(&mut app, &ctx, size, back.center());
        }
        assert_eq!(app.screen, Screen::Game);
    }
    std::fs::remove_dir_all(path).ok();
}
