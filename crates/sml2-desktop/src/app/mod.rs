use crate::{
    controllers::Controllers,
    game::Game,
    input::{Action, InputState},
    menu::{release_focus, Menu},
    settings::{Filter, Settings, Zoom},
    timing::{game_rectangle, TickBudget},
};
use eframe::egui::{
    self, Color32, ColorImage, Context, RawInput, TextureHandle, TextureOptions, ViewportCommand,
};
use sml2_runtime::content::{ExtractedContent, SessionAssets};
#[cfg(test)]
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod controls;
mod panels;
mod startup;
pub use startup::{run, Options};
mod screenshot;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Game,
    Settings,
}

pub struct DesktopApp {
    options: Options,
    settings: Settings,
    saved_settings: Settings,
    allow_save: bool,
    status: Option<String>,
    game: Option<Game>,
    texture: TextureHandle,
    screen: Screen,
    return_screen: Screen,
    input: InputState,
    controllers: Controllers,
    menu: Menu,
    capture: Option<Action>,
    debug: bool,
    paused: bool,
    step: bool,
    focused: bool,
    last_update: Instant,
    last_save: Instant,
    budget: TickBudget,
    effective_scale: u32,
    redraws: u64,
    screenshot_pending: bool,
    screenshot_check: Option<(Vec<u8>, [u32; 4])>,
    game_rect: [u32; 4],
    validation_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl DesktopApp {
    fn new(ctx: &Context, options: Options, settings: Settings, error: Option<String>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(19, 24, 23);
        visuals.selection.bg_fill = Color32::from_rgb(54, 106, 72);
        ctx.set_visuals(visuals);
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let texture = ctx.load_texture(
            "sml2-frame",
            ColorImage::from_gray([160, 144], &[0; 160 * 144]),
            TextureOptions::NEAREST,
        );
        let screen = if options.play || options.boot_title {
            Screen::Game
        } else {
            Screen::Home
        };
        let mut app = Self {
            options,
            saved_settings: settings.clone(),
            settings,
            allow_save: error.is_none(),
            status: error,
            game: None,
            texture,
            screen,
            return_screen: Screen::Home,
            input: InputState::default(),
            controllers: Controllers::new(),
            menu: Menu::default(),
            capture: None,
            debug: false,
            paused: false,
            step: false,
            focused: false,
            last_update: Instant::now(),
            last_save: Instant::now(),
            budget: TickBudget::default(),
            effective_scale: 1,
            redraws: 0,
            screenshot_pending: false,
            screenshot_check: None,
            game_rect: [0; 4],
            validation_error: Default::default(),
        };
        app.load_game();
        if app.options.boot_title {
            if let Some(game) = &mut app.game {
                game.session.enter_title();
            }
        }
        if let Some(scale) = app.options.scale {
            app.zoom(ctx, Some(scale));
        }
        app
    }

    fn load_game(&mut self) {
        match SessionAssets::load_from(&ExtractedContent::new(&self.options.assets)) {
            Ok(assets) => {
                self.game = Some(Game::new(
                    assets,
                    self.options.paths.save.clone(),
                    self.options.audio,
                ))
            }
            Err(e) => self.status = Some(format!("Game data could not be loaded: {e}")),
        }
    }

    fn navigate(&mut self, ctx: &Context, screen: Screen) {
        self.screen = screen;
        self.capture = None;
        self.menu = Menu::default();
        self.input.suppress_until_release();
        self.budget.reset();
        release_focus(ctx);
    }

    fn zoom(&mut self, ctx: &Context, scale: Option<u8>) {
        self.settings.zoom = scale.map_or(Zoom::Auto, Zoom::Integer);
        if let Some(scale) = scale {
            if !self.settings.fullscreen {
                let pixels_per_point = ctx.pixels_per_point();
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(
                    (160.0 * f32::from(scale) / pixels_per_point).max(480.0),
                    (144.0 * f32::from(scale) / pixels_per_point + 48.0).max(400.0),
                )));
            }
        }
    }

    fn fullscreen(&mut self, ctx: &Context) {
        self.settings.fullscreen = !self.settings.fullscreen;
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.settings.fullscreen));
    }

    fn ui(&mut self, ctx: &Context) {
        let focused = ctx.input(|i| i.focused && i.viewport().minimized != Some(true));
        let disconnected = self.controllers.poll(
            &mut self.input,
            self.settings.preferred_controller.as_deref(),
            self.settings.dead_zone,
        );
        if disconnected && self.screen == Screen::Game {
            self.navigate(ctx, Screen::Home);
        }
        if !focused {
            self.input.clear();
            self.budget.reset();
            self.last_update = Instant::now();
        } else if !self.focused {
            self.input.suppress_until_release();
            self.budget.reset();
            self.last_update = Instant::now();
        }
        self.focused = focused;
        self.shortcuts(ctx);
        if self.screen != Screen::Game {
            if self.capture.is_some() {
                self.menu.freeze();
            } else {
                self.menu.start(&mut self.input);
            }
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("SML2");
                if ui.button("Menu · Esc").clicked() {
                    self.navigate(ctx, Screen::Home);
                }
                if ui.button("Settings").clicked() && self.screen != Screen::Settings {
                    self.return_screen = self.screen;
                    self.navigate(ctx, Screen::Settings);
                }
                if ui.button("Debug · F1").clicked() {
                    self.debug = !self.debug;
                    release_focus(ctx);
                }
                if ui
                    .button(format!("Filter: {}", self.settings.filter.label()))
                    .clicked()
                {
                    self.cycle_filter(1);
                    release_focus(ctx);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Fullscreen · F11").clicked() {
                        self.fullscreen(ctx);
                        release_focus(ctx);
                    }
                });
            });
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::BLACK))
            .show(ctx, |ui| {
                if self.screen == Screen::Game {
                    self.game_view(ui);
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            if self.screen == Screen::Settings {
                                self.settings_panel(ctx, ui);
                            } else {
                                self.home_panel(ctx, ui);
                            }
                        });
                    });
                }
            });
        if self.screen != Screen::Game {
            self.menu.finish();
        }
        if self.debug {
            self.debug_panel(ctx);
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;
        let running = focused
            && self.screen == Screen::Game
            && !self.paused
            && self.capture.is_none()
            && !self.screenshot_pending;
        let mut ticks = self.budget.advance(elapsed, running);
        if self.step && focused {
            ticks = 1;
        }
        self.step = false;
        if self.paused {
            self.input.suppress_until_release();
        }
        if let Some(game) = &mut self.game {
            game.audio_running(running && !self.settings.muted);
            for _ in 0..ticks {
                let keys = if running {
                    self.input.sample_player(&self.settings.bindings)
                } else {
                    Default::default()
                };
                game.tick(
                    keys,
                    if self.settings.muted {
                        0.0
                    } else {
                        self.settings.volume
                    },
                );
            }
            let pixels = game.session.render_frame((160, 144));
            self.texture.set(
                colorized(&pixels, self.settings.filter),
                TextureOptions::NEAREST,
            );
        }
        if !self.settings.fullscreen && ctx.input(|i| i.viewport().maximized != Some(true)) {
            let size = ctx.input(|i| {
                i.viewport()
                    .inner_rect
                    .unwrap_or_else(|| i.content_rect())
                    .size()
            });
            self.settings.window_size = [size.x, size.y];
        }
        if now.duration_since(self.last_save) > Duration::from_millis(500) {
            self.persist();
            self.last_save = now;
        }
        self.redraws += 1;
        if let Some(frames) = self.options.smoke_frames {
            if self.redraws >= frames && !self.screenshot_pending {
                if self.options.screenshot.is_some() && self.redraws == frames {
                    self.screenshot_pending = true;
                    if self.screen == Screen::Game && !self.debug && !self.paused {
                        if let Some(game) = &self.game {
                            self.screenshot_check =
                                Some((game.session.render_frame((160, 144)), self.game_rect));
                        }
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
                } else {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
        ctx.request_repaint_after(Duration::from_millis(8));
    }

    fn game_view(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_rect_before_wrap();
        let ppp = ui.ctx().pixels_per_point();
        let (offset, size) = game_rectangle(
            [
                (available.width() * ppp).floor() as u32,
                (available.height() * ppp).floor() as u32,
            ],
            self.settings.zoom,
        );
        self.effective_scale = size[0] / 160;
        let origin = egui::pos2(
            (available.min.x * ppp).round() / ppp + offset[0] as f32 / ppp,
            (available.min.y * ppp).round() / ppp + offset[1] as f32 / ppp,
        );
        let rect = egui::Rect::from_min_size(
            origin,
            egui::vec2(size[0] as f32 / ppp, size[1] as f32 / ppp),
        );
        self.game_rect = [
            (rect.min.x * ppp).round() as u32,
            (rect.min.y * ppp).round() as u32,
            size[0],
            size[1],
        ];
        ui.painter().image(
            self.texture.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        if self.paused {
            ui.painter().text(
                available.min + egui::vec2(12.0, 12.0),
                egui::Align2::LEFT_TOP,
                "DEBUG PAUSED · F3 to step",
                egui::FontId::proportional(15.0),
                Color32::LIGHT_GREEN,
            );
        }
    }

    fn persist(&mut self) {
        if self.allow_save && self.settings != self.saved_settings {
            match self.settings.save(&self.options.paths.settings) {
                Ok(()) => self.saved_settings = self.settings.clone(),
                Err(e) => self.status = Some(format!("Settings could not be saved: {e}")),
            }
        }
    }
}

/// Map the simulation's four DMG shades into RGB for presentation.
fn colorized(frame: &[u8], filter: Filter) -> ColorImage {
    let pixels = frame
        .iter()
        .map(|&gray| {
            let [r, g, b] = filter.rgb(gray);
            Color32::from_rgb(r, g, b)
        })
        .collect();
    ColorImage::new([160, 144], pixels)
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        self.ui(ctx);
    }
    fn raw_input_hook(&mut self, _: &Context, input: &mut RawInput) {
        self.receive_input(input);
    }
    fn on_exit(&mut self) {
        self.persist();
    }
}
