use super::*;

fn frame(
    menu: &mut Menu,
    ctx: &egui::Context,
    input: &mut InputState,
    events: Vec<egui::Event>,
) -> (i32, egui::Pos2) {
    menu.start(input);
    let mut direction = 0;
    let mut origin = egui::Pos2::ZERO;
    let _ = ctx.run(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                origin = ui.cursor().min;
                direction = menu.choice(ui, "Camera zoom", "125%".into());
            });
        },
    );
    menu.finish();
    (direction, origin)
}

#[test]
fn keyboard_and_controller_arrows_produce_opposite_directions() {
    for (button, expected) in [
        (Button::key("ArrowLeft"), -1),
        (Button::key("ArrowRight"), 1),
        (Button::Pad(PadButton::Left), -1),
        (Button::Pad(PadButton::Right), 1),
    ] {
        let mut input = InputState::default();
        input.event(button, true);
        assert_eq!(
            frame(
                &mut Menu::default(),
                &egui::Context::default(),
                &mut input,
                vec![]
            )
            .0,
            expected
        );
    }
}

#[test]
fn clicking_each_painted_arrow_adjusts_in_its_own_direction() {
    let ctx = egui::Context::default();
    let mut menu = Menu::default();
    let mut input = InputState::default();
    let (_, origin) = frame(&mut menu, &ctx, &mut input, vec![]);
    // These are the visible arrow positions on a 440 px row with a short value.
    for (x, expected) in [(334.0, -1), (422.0, 1)] {
        let pos = origin + egui::vec2(x, 17.0);
        for pressed in [true, false] {
            let (direction, _) = frame(
                &mut menu,
                &ctx,
                &mut input,
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
            assert_eq!(direction, if pressed { 0 } else { expected });
        }
    }
}
