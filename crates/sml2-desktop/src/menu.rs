use crate::input::{Button, InputState, PadButton};
use eframe::egui::{self, Button as Widget, Ui};

pub fn pressed(input: &mut InputState, buttons: &[Button]) -> bool {
    buttons
        .iter()
        .fold(false, |found, button| input.take_press(button) | found)
}

pub struct Menu {
    pub selected: usize,
    count: usize,
    row: usize,
    confirm: bool,
    enabled: bool,
    moved: bool,
    pub adjustment: i32,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            selected: 0,
            count: 1,
            row: 0,
            confirm: false,
            enabled: true,
            moved: false,
            adjustment: 0,
        }
    }
}

impl Menu {
    pub fn freeze(&mut self) {
        self.row = 0;
        self.confirm = false;
        self.adjustment = 0;
        self.enabled = false;
        self.moved = false;
    }
    pub fn start(&mut self, input: &mut InputState) {
        self.enabled = true;
        let up = pressed(
            input,
            &[
                Button::key("ArrowUp"),
                Button::Pad(PadButton::Up),
                Button::Pad(PadButton::StickUp),
            ],
        );
        let down = pressed(
            input,
            &[
                Button::key("ArrowDown"),
                Button::Pad(PadButton::Down),
                Button::Pad(PadButton::StickDown),
            ],
        );
        self.moved = up || down;
        self.selected = (self.selected as i32 + i32::from(down) - i32::from(up))
            .rem_euclid(self.count.max(1) as i32) as usize;
        let left = pressed(
            input,
            &[
                Button::key("ArrowLeft"),
                Button::Pad(PadButton::Left),
                Button::Pad(PadButton::StickLeft),
            ],
        );
        let right = pressed(
            input,
            &[
                Button::key("ArrowRight"),
                Button::Pad(PadButton::Right),
                Button::Pad(PadButton::StickRight),
            ],
        );
        self.adjustment = i32::from(right) - i32::from(left);
        self.confirm = pressed(
            input,
            &[
                Button::key("Enter"),
                Button::key("Space"),
                Button::Pad(PadButton::South),
            ],
        );
        self.row = 0;
    }
    pub fn button(&mut self, ui: &mut Ui, text: impl Into<String>) -> bool {
        let selected = self.row == self.selected;
        let response = ui
            .add_enabled_ui(self.enabled, |ui| {
                ui.add_sized([340.0, 32.0], Widget::new(text.into()).selected(selected))
            })
            .inner;
        if selected && self.moved {
            response.scroll_to_me(Some(egui::Align::Center));
        }
        // Keyboard/controller selection is owned by this menu, not stale egui focus.
        let clicked = response.clicked_by(egui::PointerButton::Primary);
        let activated = clicked || selected && self.confirm;
        if clicked {
            self.selected = self.row;
        }
        self.row += 1;
        activated
    }
    pub fn row_adjustment(&self) -> i32 {
        if self.row == self.selected {
            self.adjustment
        } else {
            0
        }
    }
    pub fn finish(&mut self) {
        self.count = self.row.max(1);
        self.selected = self.selected.min(self.count - 1);
    }
}

pub fn release_focus(ctx: &egui::Context) {
    ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
}
