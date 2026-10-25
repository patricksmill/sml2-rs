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
