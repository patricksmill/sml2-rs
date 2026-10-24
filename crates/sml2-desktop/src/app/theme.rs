//! Translucent pause popup and DMG-inspired settings controls.
//! Painting only; selection and activation
//! stay with [`crate::menu::Menu`].

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2,
};

pub const SHELL: Color32 = Color32::from_rgb(0xD4, 0xD0, 0xC8);
pub const SHELL_LIGHT: Color32 = Color32::from_rgb(0xE6, 0xE3, 0xDC);
pub const SHELL_SHADOW: Color32 = Color32::from_rgb(0x8E, 0x8A, 0x84);
pub const BEZEL: Color32 = Color32::from_rgb(0x5B, 0x5D, 0x70);
pub const MAGENTA: Color32 = Color32::from_rgb(0xA3, 0x1B, 0x5B);
pub const MAGENTA_DARK: Color32 = Color32::from_rgb(0x66, 0x0E, 0x38);
pub const PILL: Color32 = Color32::from_rgb(0x84, 0x81, 0x8C);
pub const PILL_DARK: Color32 = Color32::from_rgb(0x52, 0x50, 0x58);
pub const NAVY: Color32 = Color32::from_rgb(0x2A, 0x2D, 0x8A);
pub const INK: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x38);
pub const GOLD: Color32 = Color32::from_rgb(0xF8, 0xD0, 0x30);
pub const LED: Color32 = Color32::from_rgb(0xE8, 0x20, 0x30);
pub const ERROR: Color32 = Color32::from_rgb(0xB0, 0x10, 0x20);

pub const ROW_SIZE: Vec2 = Vec2::new(440.0, 34.0);

/// A shell-plastic card with the DMG's big lower-right corner.
pub fn card() -> egui::Frame {
    egui::Frame::NONE
        .fill(SHELL)
        .stroke(Stroke::new(1.0_f32, SHELL_SHADOW))
        .corner_radius(CornerRadius {
            nw: 10,
            ne: 10,
            sw: 10,
            se: 42,
        })
        .inner_margin(egui::Margin::symmetric(22, 18))
        .shadow(egui::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(110),
        })
}

/// A compact translucent surface: the paused game remains visible beneath it.
pub fn popup_frame() -> egui::Frame {
    card()
        .fill(Color32::from_rgba_unmultiplied(0xD4, 0xD0, 0xC8, 200))
        .corner_radius(12)
        .inner_margin(16)
}

pub fn menu_button(ui: &mut Ui, text: &str, selected: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(260.0, 30.0), Sense::click());
    let ink = if selected { Color32::WHITE } else { INK };
    if selected {
        ui.painter()
            .rect_filled(rect, 5, MAGENTA.gamma_multiply(0.85));
        cursor(ui, rect.left_center() + Vec2::new(13.0, 0.0), ink);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 5, Color32::from_black_alpha(20));
    }
    ui.painter().text(
        rect.left_center() + Vec2::new(28.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(15.0),
        ink,
    );
    response
}

/// The menu's ▶ cursor, drawn so it never depends on the font's glyphs.
fn cursor(ui: &Ui, center: Pos2, color: Color32) {
    let h = 6.0;
    ui.painter().add(Shape::convex_polygon(
        vec![
            center + Vec2::new(-h * 0.8, -h),
            center + Vec2::new(h * 0.9, 0.0),
            center + Vec2::new(-h * 0.8, h),
        ],
        color,
        Stroke::NONE,
    ));
}

fn arrow(ui: &Ui, center: Pos2, left: bool, color: Color32) {
    let (h, w) = (5.0, if left { -5.0 } else { 5.0 });
    ui.painter().add(Shape::convex_polygon(
        if left {
            vec![
                center + Vec2::new(w, 0.0),
                center + Vec2::new(-w, h),
                center + Vec2::new(-w, -h),
            ]
        } else {
            vec![
                center + Vec2::new(-w, -h),
                center + Vec2::new(w, 0.0),
                center + Vec2::new(-w, h),
            ]
        },
        color,
        Stroke::NONE,
    ));
}

/// Raised plastic with a drop shadow that collapses while the button is held.
fn raised(
    ui: &Ui,
    rect: Rect,
    response: &Response,
    fill: Color32,
    edge: Color32,
    radius: u8,
) -> Rect {
    let pressed = response.is_pointer_button_down_on();
    let depth = if pressed { 1.0 } else { 3.0 };
    let face = rect.translate(Vec2::new(0.0, 3.0 - depth));
    let painter = ui.painter();
    painter.rect_filled(face.translate(Vec2::new(0.0, depth)), radius, edge);
    let fill = if response.hovered() && !pressed {
        fill.lerp_to_gamma(Color32::WHITE, 0.12)
    } else {
        fill
    };
    painter.rect(
        face,
        radius,
        fill,
        Stroke::new(1.0_f32, edge),
        StrokeKind::Inside,
    );
    face
}

fn interact(ui: &mut Ui, size: Vec2) -> (Rect, Response) {
    let (rect, response) = ui.allocate_exact_size(size + Vec2::new(0.0, 3.0), Sense::click());
    (Rect::from_min_size(rect.min, size), response)
}

/// What a settings row shows on its right-hand side.
pub enum Value {
    None,
    /// A ◀ value ▶ choice adjusted with left/right.
    Choice(String),
    /// A level meter, `0.0..=1.0`, with its readout.
    Meter(f32, String),
    /// The power LED: lit while on.
    Toggle(bool),
    /// Plain text, e.g. a binding list.
    Text(String),
}

/// The two arrow centers, shared by painting and pointer hit testing.
pub fn choice_arrows(ui: &Ui, rect: Rect, text: &str) -> (Pos2, Pos2) {
    let width = ui
        .painter()
        .layout_no_wrap(text.into(), FontId::proportional(15.0), NAVY)
        .size()
        .x
        .max(60.0);
    let right = rect.right_center() - Vec2::new(14.0, 0.0);
    (
        right - Vec2::new(32.0 + width, 0.0),
        right - Vec2::new(4.0, 0.0),
    )
}

/// A full-width settings row: label left, value right.
pub fn option_row(ui: &mut Ui, label: &str, value: &Value, selected: bool) -> Response {
    let (rect, response) = interact(ui, ROW_SIZE);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let (fill, edge, ink) = if selected {
        (MAGENTA, MAGENTA_DARK, Color32::WHITE)
    } else {
        (SHELL_LIGHT, SHELL_SHADOW, INK)
    };
    let face = raised(ui, rect, &response, fill, edge, 8);
    let painter = ui.painter();
    if selected {
        cursor(
            ui,
            face.left_center() + Vec2::new(14.0, 0.0),
            Color32::WHITE,
        );
    }
    let font = FontId::proportional(15.0);
    painter.text(
        face.left_center() + Vec2::new(28.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        font.clone(),
        ink,
    );
    let right = face.right_center() - Vec2::new(14.0, 0.0);
    let soft = if selected { Color32::WHITE } else { NAVY };
    match value {
        Value::None => {}
        Value::Text(text) => {
            painter.text(right, Align2::RIGHT_CENTER, text, font, soft);
        }
        Value::Choice(text) => {
            let galley = painter.layout_no_wrap(text.clone(), font.clone(), soft);
            let width = galley.size().x.max(60.0);
            let (previous, next) = choice_arrows(ui, face, text);
            arrow(ui, next, false, soft);
            painter.text(
                right - Vec2::new(18.0 + width / 2.0, 0.0),
                Align2::CENTER_CENTER,
                text,
                font,
                soft,
            );
            arrow(ui, previous, true, soft);
        }
        Value::Meter(level, text) => {
            painter.text(right, Align2::RIGHT_CENTER, text, font, soft);
            let bar = Rect::from_min_size(right - Vec2::new(170.0, 5.0), Vec2::new(110.0, 10.0));
            let back = if selected { MAGENTA_DARK } else { SHELL_SHADOW };
            painter.rect_filled(bar, 5, back);
            let segments = 10;
            let lit = (level.clamp(0.0, 1.0) * segments as f32).round() as usize;
            let step = bar.width() / segments as f32;
            for i in 0..lit {
                let seg = Rect::from_min_size(
                    bar.min + Vec2::new(i as f32 * step + 1.5, 2.0),
                    Vec2::new(step - 3.0, bar.height() - 4.0),
                );
                painter.rect_filled(seg, 2, if selected { Color32::WHITE } else { NAVY });
            }
        }
        Value::Toggle(on) => {
            let center = right - Vec2::new(8.0, 0.0);
            painter.circle_filled(center, 6.0, if *on { LED } else { PILL_DARK });
            if *on {
                painter.circle_filled(center, 10.0, LED.gamma_multiply(0.25));
            }
            painter.text(
                center - Vec2::new(16.0, 0.0),
                Align2::RIGHT_CENTER,
                if *on { "ON" } else { "OFF" },
                font,
                soft,
            );
        }
    }
    response
}

/// One START/SELECT-style pill in the settings tab strip.
pub fn tab(ui: &mut Ui, text: &str, active: bool, focused: bool) -> Response {
    let font = FontId::proportional(13.0);
    let width = ui
        .painter()
        .layout_no_wrap(text.into(), font.clone(), INK)
        .size()
        .x
        + 30.0;
    let (rect, response) = interact(ui, Vec2::new(width, 26.0));
    let (fill, edge) = if active {
        (MAGENTA, MAGENTA_DARK)
    } else {
        (PILL, PILL_DARK)
    };
    let face = raised(ui, rect, &response, fill, edge, 13);
    if active && focused {
        ui.painter().rect_stroke(
            face.expand(3.0),
            16,
            Stroke::new(2.0_f32, Color32::WHITE),
            StrokeKind::Outside,
        );
    }
    ui.painter().text(
        face.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        Color32::WHITE,
    );
    response
}

/// A small heading inside a card.
pub fn section(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).size(12.0).strong().color(BEZEL));
}

pub fn note(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.5).color(INK));
}

pub fn error(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(13.0).strong().color(ERROR));
}
