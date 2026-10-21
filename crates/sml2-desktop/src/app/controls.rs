use super::{screenshot, DesktopApp, Screen};
use crate::{
    input::{Button, PadButton},
    menu::pressed,
};
use eframe::egui::{self, Context, RawInput};

impl DesktopApp {
    pub(super) fn receive_input(&mut self, raw: &RawInput) {
        for event in &raw.events {
            match event {
                egui::Event::Key {
                    key,
                    physical_key,
                    pressed,
                    repeat,
                    ..
                } => {
                    if !repeat || !pressed {
                        self.input.event(
                            Button::key(&format!("{:?}", physical_key.unwrap_or(*key))),
                            *pressed,
                        );
                    }
                }
                egui::Event::WindowFocused(false) => self.input.clear(),
                egui::Event::Screenshot { image, .. } => {
                    if let Some(path) = &self.options.screenshot {
                        match screenshot::save(path, image) {
                            Ok(()) => eprintln!("Screenshot: {}", path.display()),
                            Err(e) => {
                                self.status = Some(e.clone());
                                *self.validation_error.lock().unwrap() = Some(e);
                            }
                        }
                    }
                    if let Some((frame, rect)) = self.screenshot_check.take() {
                        match screenshot::verify(image, &frame, rect) {
                            Ok(()) => eprintln!("GPU game pixels match the CPU frame exactly"),
                            Err(e) => {
                                eprintln!("{e}");
                                *self.validation_error.lock().unwrap() = Some(e);
                            }
                        }
                    }
                    self.screenshot_pending = false;
                }
                _ => {}
            }
        }
        self.input.event(Button::key("Shift"), raw.modifiers.shift);
        self.input.event(Button::key("Control"), raw.modifiers.ctrl);
        self.input.event(Button::key("Alt"), raw.modifiers.alt);
    }

    pub(super) fn cycle_filter(&mut self, delta: i32) {
        self.settings.filter = self.settings.filter.cycle(delta);
    }

    pub(super) fn shortcuts(&mut self, ctx: &Context) {
        let back = pressed(
            &mut self.input,
            &[Button::key("Escape"), Button::Pad(PadButton::Start)],
        );
        if self.capture.is_some() {
            if back {
                self.capture = None;
                self.input.suppress_until_release();
            } else if let Some(button) = self.input.first_press() {
                match self.settings.bindings.rebind(self.capture.unwrap(), button) {
                    Ok(()) => {
                        self.capture = None;
                        self.status = None;
                    }
                    Err(e) => self.status = Some(e),
                }
                self.input.suppress_until_release();
            }
            return;
        }
        if self.screen != Screen::Game && self.input.take_press(&Button::Pad(PadButton::East))
            || back
        {
            let target = match self.screen {
                Screen::Settings => self.return_screen,
                Screen::Home if self.game.is_some() => Screen::Game,
                _ => Screen::Home,
            };
            self.navigate(ctx, target);
        }
        if self.input.take_press(&Button::key("F11")) {
            self.fullscreen(ctx);
        }
        if self.input.take_press(&Button::key("F4")) {
            self.cycle_filter(1);
        }
        if self.input.take_press(&Button::key("F1")) {
            self.debug = !self.debug;
        }
        if self.input.take_press(&Button::key("F2")) {
            self.paused = !self.paused;
            self.input.suppress_until_release();
        }
        if self.input.take_press(&Button::key("F3")) {
            self.paused = true;
            self.step = true;
        }
        let plus = pressed(
            &mut self.input,
            &[Button::key("Plus"), Button::key("Equals")],
        );
        let minus = self.input.take_press(&Button::key("Minus"));
        if plus || minus {
            let next = if plus {
                self.effective_scale.saturating_add(1)
            } else {
                self.effective_scale.saturating_sub(1)
            };
            self.zoom(ctx, Some(next.clamp(1, 8) as u8));
        }
    }
}
