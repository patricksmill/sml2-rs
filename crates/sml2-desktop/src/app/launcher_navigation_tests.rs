use super::*;
use crate::app::tests::{app, frame};
use crate::input::{Button, PadButton};
use sml2_runtime::session::SessionState;

fn button_frame(
    app: &mut DesktopApp,
    ctx: &egui::Context,
    button: &Button,
    pressed: bool,
) -> egui::FullOutput {
    let events = match button {
        Button::Key(name) => {
            let key = egui::Key::from_name(name).unwrap();
            vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers: Default::default(),
            }]
        }
        Button::Pad(_) => {
            app.input.event(button.clone(), pressed);
            vec![]
        }
    };
    frame(app, ctx, events)
}

#[test]
fn title_actions_support_keyboard_dpad_and_stick_navigation() {
    for (next, previous, confirm) in [
        (
            Button::key("ArrowRight"),
            Button::key("ArrowLeft"),
            Button::key("Enter"),
        ),
        (
            Button::key("ArrowDown"),
            Button::key("ArrowUp"),
            Button::key("Space"),
        ),
        (
            Button::Pad(PadButton::Right),
            Button::Pad(PadButton::Left),
            Button::Pad(PadButton::South),
        ),
        (
            Button::Pad(PadButton::Down),
            Button::Pad(PadButton::Up),
            Button::Pad(PadButton::South),
        ),
        (
            Button::Pad(PadButton::StickRight),
            Button::Pad(PadButton::StickLeft),
            Button::Pad(PadButton::South),
        ),
        (
            Button::Pad(PadButton::StickDown),
            Button::Pad(PadButton::StickUp),
            Button::Pad(PadButton::South),
        ),
        (
            Button::Pad(PadButton::Right),
            Button::Pad(PadButton::Left),
            Button::Pad(PadButton::Start),
        ),
    ] {
        let (mut app, ctx, path) = app();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        for (button, expected) in [(&previous, 2), (&next, 0), (&next, 1)] {
            let output = button_frame(&mut app, &ctx, button, true);
            assert_eq!(app.menu.selected, expected, "navigation with {button:?}");
            let label = ["Play", "Settings", "Quit"][expected];
            let text_rect = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.visual_bounding_rect())
                    }
                    _ => None,
                })
                .expect("the selected action is visible");
            assert!(
                output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Rect(rect) if rect.fill == Color32::from_black_alpha(90)
                        && rect.rect.contains_rect(text_rect)
                )),
                "{label} must visibly highlight after {button:?}"
            );
            frame(&mut app, &ctx, vec![]);
            assert_eq!(app.menu.selected, expected, "holding {button:?}");
            button_frame(&mut app, &ctx, button, false);
        }
        button_frame(&mut app, &ctx, &confirm, true);
        assert_eq!(app.screen, Screen::Settings);
        assert!(matches!(
            app.game.as_ref().unwrap().session.state,
            SessionState::Title(_)
        ));
        let _ = std::fs::remove_dir_all(path);
    }
}

#[test]
fn title_background_clicks_do_nothing_and_only_play_opens_file_select() {
    for target in ["title", "grass", "Settings", "Play"] {
        let (mut app, ctx, path) = app();
        if let SessionState::Title(title) = &mut app.game.as_mut().unwrap().session.state {
            while !super::super::pointer::title_accepts_start(title.timer()) {
                title.step(0, 0);
            }
        }
        frame(&mut app, &ctx, vec![]);
        let output = frame(&mut app, &ctx, vec![]);
        let [x, y, w, h] = app.game_rect;
        let pos = match target {
            "title" => egui::pos2((x + w / 2) as f32, (y + h / 2) as f32),
            "grass" => egui::pos2(30.0, 690.0),
            label => output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .expect("launcher action"),
        };
        for pressed in [true, false] {
            frame(
                &mut app,
                &ctx,
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
        assert!(
            !app.start_click,
            "no pending background start after clicking {target}"
        );
        match target {
            "Play" => assert!(!matches!(
                app.game.as_ref().unwrap().session.state,
                SessionState::Title(_)
            )),
            _ => assert!(
                matches!(
                    app.game.as_ref().unwrap().session.state,
                    SessionState::Title(_)
                ),
                "clicking {target} must not start the game"
            ),
        }
        assert_eq!(
            app.screen,
            if target == "Settings" {
                Screen::Settings
            } else {
                Screen::Game
            }
        );
        if matches!(target, "title" | "grass") {
            button_frame(&mut app, &ctx, &Button::key("ArrowRight"), true);
            button_frame(&mut app, &ctx, &Button::key("ArrowRight"), false);
            assert_eq!(app.menu.selected, 1);
            button_frame(&mut app, &ctx, &Button::key("Enter"), true);
            assert_eq!(app.screen, Screen::Settings);
            assert!(matches!(
                app.game.as_ref().unwrap().session.state,
                SessionState::Title(_)
            ));
        }
        let _ = std::fs::remove_dir_all(path);
    }
}

#[test]
fn play_confirmation_survives_the_opening_hold_and_held_direction() {
    let (mut app, ctx, path) = app();
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    // Wrap from Play to Quit and back, keeping Right held while confirming Play.
    button_frame(&mut app, &ctx, &Button::key("ArrowLeft"), true);
    button_frame(&mut app, &ctx, &Button::key("ArrowLeft"), false);
    button_frame(&mut app, &ctx, &Button::key("ArrowRight"), true);
    assert_eq!(app.menu.selected, 0);
    button_frame(&mut app, &ctx, &Button::key("Enter"), true);
    button_frame(&mut app, &ctx, &Button::key("Enter"), false);
    for _ in 0..80 {
        frame(&mut app, &ctx, vec![]);
    }
    assert!(
        matches!(
            app.game.as_ref().unwrap().session.state,
            SessionState::FileSelect(_) | SessionState::FileSelectLoading(_)
        ),
        "Play must reach file select even during the initial title hold"
    );
    let _ = std::fs::remove_dir_all(path);
}
