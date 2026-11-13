use crate::app::theme::{self, Value};
use crate::input::{Button, InputState, PadButton};
use eframe::egui::{self, Response, Ui};

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
    /// The previous row's label and whether it is hovered, while it is selected or hovered.
    last: Option<(String, bool)>,
    detail: Option<Detail>,
}

/// What the settings detail pane explains: the hovered row, else the selected one.
#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub title: String,
    pub text: String,
    pub preview: Option<Preview>,
    hovered: bool,
}

/// The live sample shown under a row's explanation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Preview {
    /// The whole game frame with the current display settings.
    Frame,
    /// A magnified corner showing how game pixels map to screen pixels.
    Pixels,
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
            last: None,
            detail: None,
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
        self.last = None;
        self.detail = None;
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
        self.last = None;
        self.detail = None;
    }
    pub fn start_horizontal(&mut self, input: &mut InputState) {
        self.start(input);
        // Start confirms the highlighted launcher action, like the south button.
        self.confirm |= pressed(input, &[Button::Pad(PadButton::Start)]);
        self.selected =
            (self.selected as i32 + self.adjustment).rem_euclid(self.count.max(1) as i32) as usize;
        self.moved |= self.adjustment != 0;
        self.adjustment = 0;
    }
    pub fn compact_button(&mut self, ui: &mut Ui, text: &str) -> bool {
        self.item(ui, |ui, selected| {
            ui.add(egui::Button::new(text).selected(selected))
        })
    }
    pub fn button(&mut self, ui: &mut Ui, text: impl Into<String>) -> bool {
        let text = text.into();
        self.item(ui, |ui, selected| theme::menu_button(ui, &text, selected))
    }
    /// A settings row; left/right adjustments are read with [`Self::row_adjustment`]
    /// before the call.
    pub fn row(&mut self, ui: &mut Ui, label: &str, value: Value) -> bool {
        let activated = self.item(ui, |ui, selected| {
            theme::option_row(ui, label, &value, selected)
        });
        self.label_last(label);
        activated
    }

    /// Explains the row just drawn in the detail pane when it is hovered or selected.
    pub fn describe(&mut self, text: &str) {
        self.set_detail(text, None);
    }

    /// Like [`Self::describe`], and asks the pane to show a live sample too.
    pub fn describe_with_preview(&mut self, text: &str, preview: Preview) {
        self.set_detail(text, Some(preview));
    }

    pub fn detail(&self) -> Option<&Detail> {
        self.detail.as_ref()
    }

    fn label_last(&mut self, label: &str) {
        if let Some((title, _)) = &mut self.last {
            *title = label.to_string();
        }
    }

    fn set_detail(&mut self, text: &str, preview: Option<Preview>) {
        let Some((title, hovered)) = self.last.clone() else {
            return;
        };
        if hovered || !self.detail.as_ref().is_some_and(|d| d.hovered) {
            self.detail = Some(Detail {
                title,
                text: text.to_string(),
                preview,
                hovered,
            });
        }
    }

    /// Signed choice input: keyboard/controller arrows and the painted arrows
    /// agree. Confirming or clicking the rest of the row advances one choice.
    pub fn choice(&mut self, ui: &mut Ui, label: &str, text: String) -> i32 {
        let adjustment = self.row_adjustment();
        let mut pointer_direction = 0;
        let activated = self.item(ui, |ui, selected| {
            let response = theme::option_row(ui, label, &Value::Choice(text.clone()), selected);
            if response.clicked_by(egui::PointerButton::Primary) {
                let (previous, _) = theme::choice_arrows(ui, response.rect, &text);
                pointer_direction = if response.interact_pointer_pos().is_some_and(|pos| {
                    egui::Rect::from_center_size(previous, egui::vec2(28.0, response.rect.height()))
                        .contains(pos)
                }) {
                    -1
                } else {
                    1
                };
            }
            response
        });
        self.label_last(label);
        if pointer_direction != 0 {
            pointer_direction
        } else if adjustment != 0 {
            adjustment
        } else {
            i32::from(activated)
        }
    }
    /// A tab strip occupying one row: left/right move between tabs while it is
    /// selected, and a click picks a tab directly. Returns whether `current` changed.
    pub fn tabs(&mut self, ui: &mut Ui, labels: &[&str], current: &mut usize) -> bool {
        let focused = self.row == self.selected;
        let before = *current;
        if focused && self.adjustment != 0 {
            *current = (*current as i32 + self.adjustment).rem_euclid(labels.len() as i32) as usize;
            self.adjustment = 0;
        }
        let response = ui
            .add_enabled_ui(self.enabled, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for (index, label) in labels.iter().enumerate() {
                        if theme::tab(ui, label, index == *current, focused)
                            .clicked_by(egui::PointerButton::Primary)
                        {
                            *current = index;
                            self.selected = self.row;
                        }
                    }
                })
                .response
            })
            .inner;
        if focused && self.moved {
            response.scroll_to_me(Some(egui::Align::Center));
        }
        self.row += 1;
        self.last = None;
        *current != before
    }
    fn item(&mut self, ui: &mut Ui, paint: impl FnOnce(&mut Ui, bool) -> Response) -> bool {
        let selected = self.row == self.selected;
        let response = ui
            .add_enabled_ui(self.enabled, |ui| paint(ui, selected))
            .inner;
        if selected && self.moved {
            response.scroll_to_me(Some(egui::Align::Center));
        }
        let hovered = response.hovered();
        self.last = (selected || hovered).then(|| (String::new(), hovered));
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

pub use crate::ui::release_focus;

#[cfg(test)]
mod tests;
