//! Standard egui controls with keyboard/controller selection owned by Menu.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Response, Ui, Vec2};

pub const ACCENT: Color32 = Color32::from_rgb(90, 170, 255);
pub const HIGHLIGHT: Color32 = Color32::from_rgb(255, 220, 100);
pub const ERROR: Color32 = Color32::LIGHT_RED;
pub const ROW_SIZE: Vec2 = Vec2::new(440.0, 34.0);

pub fn menu_button(ui: &mut Ui, text: &str, selected: bool) -> Response {
    ui.add_sized([260.0, 30.0], egui::Button::new(text).selected(selected))
}

pub enum Value {
    None,
    Choice(String),
    Meter(f32, String),
    Toggle(bool),
    Text(String),
}

/// Shared by painting and pointer hit testing.
pub fn choice_arrows(ui: &Ui, rect: Rect, text: &str) -> (Pos2, Pos2) {
    let width = ui
        .painter()
        .layout_no_wrap(
            text.into(),
            FontId::proportional(15.0),
            ui.visuals().text_color(),
        )
        .size()
        .x
        .max(60.0);
    let right = rect.right_center() - Vec2::new(14.0, 0.0);
    (
        right - Vec2::new(32.0 + width, 0.0),
        right - Vec2::new(4.0, 0.0),
    )
}

/// Native button styling keeps hover, disabled and focus feedback consistent.
pub fn option_row(ui: &mut Ui, label: &str, value: &Value, selected: bool) -> Response {
    let response = ui.add_sized(ROW_SIZE, egui::Button::new("").selected(selected));
    let rect = response.rect;
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let ink = ui
        .style()
        .interact_selectable(&response, selected)
        .text_color();
    let font = FontId::proportional(15.0);
    let painter = ui.painter();
    painter.text(
        rect.left_center() + Vec2::new(12.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        font.clone(),
        ink,
    );
    let right = rect.right_center() - Vec2::new(14.0, 0.0);
    match value {
        Value::None => {}
        Value::Text(text) => {
            painter.text(right, Align2::RIGHT_CENTER, text, font, ink);
        }
        Value::Toggle(on) => {
            painter.text(
                right,
                Align2::RIGHT_CENTER,
                if *on { "On" } else { "Off" },
                font,
                ink,
            );
        }
        Value::Choice(text) => {
            let (previous, next) = choice_arrows(ui, rect, text);
            painter.text(previous, Align2::CENTER_CENTER, "‹", font.clone(), ink);
            painter.text(next, Align2::CENTER_CENTER, "›", font.clone(), ink);
            painter.text(
                previous.lerp(next, 0.5),
                Align2::CENTER_CENTER,
                text,
                font,
                ink,
            );
        }
        Value::Meter(level, text) => {
            painter.text(right, Align2::RIGHT_CENTER, text, font, ink);
            let bar = Rect::from_min_size(right - Vec2::new(170.0, 2.0), Vec2::new(110.0, 4.0));
            painter.rect_filled(bar, 2, ui.visuals().extreme_bg_color);
            let fill = Rect::from_min_size(
                bar.min,
                Vec2::new(bar.width() * level.clamp(0.0, 1.0), bar.height()),
            );
            painter.rect_filled(fill, 2, ui.visuals().selection.stroke.color);
        }
    }
    response
}

pub fn tab(ui: &mut Ui, text: &str, active: bool, focused: bool) -> Response {
    let response = ui.selectable_label(active, text);
    if active && focused {
        ui.painter().rect_stroke(
            response.rect,
            2,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    response
}

pub fn section(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.strong(text);
}

pub fn note(ui: &mut Ui, text: &str) {
    ui.weak(text);
}

pub fn error(ui: &mut Ui, text: &str) {
    ui.colored_label(ERROR, text);
}
