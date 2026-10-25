use super::*;
use crate::app::tests::{app, frame};
use crate::input::{Button, PadButton};
use sml2_runtime::session::SessionState;

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
            match button {
                Button::Key(name) => {
                    let key = egui::Key::from_name(name).unwrap();
                    frame(
                        &mut app,
                        &ctx,
                        vec![egui::Event::Key {
                            key,
                            physical_key: Some(key),
                            pressed: true,
                            repeat: false,
                            modifiers: Default::default(),
                        }],
                    );
                }
                Button::Pad(_) => {
                    app.input.event(button.clone(), true);
                    frame(&mut app, &ctx, vec![]);
                }
            }
            assert_eq!(app.menu.selected, expected, "navigation with {button:?}");
            app.input.event(button.clone(), false);
        }
        app.input.event(confirm, true);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.screen, Screen::Settings);
        assert!(matches!(
            app.game.as_ref().unwrap().session.state,
            SessionState::Title(_)
        ));
        let _ = std::fs::remove_dir_all(path);
    }
}
