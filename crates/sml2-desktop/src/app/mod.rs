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
use sml2_core::video::PalettePixel;
use sml2_runtime::content::{ExtractedContent, SessionAssets};
use sml2_runtime::render::motion::{pick_scale, Smooth};
#[cfg(test)]
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod backdrop;
mod controls;
mod debug;
mod dx;
mod editor;
mod file_picker;
mod filter_preview;
mod panels;
mod startup;
pub(crate) mod theme;
pub use startup::{run, Options};
#[cfg(test)]
mod dx_tests;
mod screenshot;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Game,
    Settings,
    Editor,
}

use panels::Page;

pub struct DesktopApp {
    options: Options,
    settings: Settings,
    saved_settings: Settings,
    allow_save: bool,
    status: Option<String>,
    game: Option<Game>,
    dx_bundle: Option<sml2_dx::Presentation>,
    dx_worker: Option<crate::dx_import::Worker>,
    dx_picker: Option<file_picker::FilePicker>,
    dx_message: String,
    texture: TextureHandle,
    screen: Screen,
    return_screen: Screen,
    /// The settings tab last shown.
    page: Page,
    /// "Reset all settings" was pressed once and waits for confirmation.
    reset_armed: bool,
    /// Level 1 behind the launcher; `None` until the game data loads.
    backdrop: Option<backdrop::Backdrop>,
    /// The Display page's sample frame; `None` until the game data loads.
    filter_preview: Option<filter_preview::FilterPreview>,
    input: InputState,
    controllers: Controllers,
    menu: Menu,
    capture: Option<Action>,
    debug: bool,
    paused: bool,
    step: bool,
    /// Debug-panel pickers: overworld screen, map node, level id.
    debug_screen: u8,
    debug_node: u8,
    debug_level: u8,
    /// What the last debug edit did, shown under the controls.
    debug_note: Option<String>,
    /// Whether that note is a failure (drawn red instead of green).
    debug_note_error: bool,
    /// Debug overlays painted over the game image.
    show_hitboxes: bool,
    show_native_border: bool,
    /// Level picker entries, cached when the assets load.
    levels: Vec<(u8, String)>,
    editor: editor::Editor,
    focused: bool,
    last_update: Instant,
    last_save: Instant,
    budget: TickBudget,
    effective_scale: u32,
    redraws: u64,
    screenshot_pending: bool,
    screenshot_check: Option<(ColorImage, [u32; 4])>,
    game_rect: [u32; 4],
    validation_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl DesktopApp {
    fn new(ctx: &Context, options: Options, settings: Settings, error: Option<String>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(0x2B, 0x2D, 0x38);
        visuals.selection.bg_fill = theme::MAGENTA;
        ctx.set_visuals(visuals);
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let texture = ctx.load_texture(
            "sml2-frame",
            ColorImage::from_gray([160, 144], &[0; 160 * 144]),
            TextureOptions::NEAREST,
        );
        let screen = if options.play || options.boot_title || options.boot_select {
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
            dx_bundle: None,
            dx_worker: None,
            dx_picker: None,
            dx_message: String::new(),
            texture,
            screen,
            return_screen: Screen::Home,
            page: Page::Display,
            reset_armed: false,
            backdrop: None,
            filter_preview: None,
            input: InputState::default(),
            controllers: Controllers::new(),
            menu: Menu::default(),
            capture: None,
            debug: false,
            paused: false,
            step: false,
            debug_screen: 0,
            debug_node: 0x14,
            debug_level: 0,
            debug_note: None,
            debug_note_error: false,
            show_hitboxes: false,
            show_native_border: false,
            levels: Vec::new(),
            editor: editor::Editor::default(),
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
        app.restore_dx();
        if app.options.boot_select {
            if let Some(game) = &mut app.game {
                game.session.enter_file_select();
            }
        } else if app.options.boot_title {
            if let Some(game) = &mut app.game {
                game.session.enter_title();
            }
        }
        if let Some(room) = app.options.room {
            if let Some(game) = &mut app.game {
                if room >= 13 {
                    game.session.begin_bell_room(room, 0, 0);
                } else {
                    game.session.begin_coin_room(room);
                }
            }
        } else if app.options.reveal {
            if let Some(game) = &mut app.game {
                let mut seq = sml2_core::castle_sequence::CastleSequence::default();
                seq.coins = [0x80; 6];
                game.session.begin_castle_sequence(seq, 8, 0, 0);
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
                match backdrop::Backdrop::new(&assets) {
                    Ok(backdrop) => self.backdrop = Some(backdrop),
                    Err(e) => eprintln!("Launcher backdrop unavailable: {e}"),
                }
                let (assets, preview) = filter_preview::FilterPreview::new(assets);
                match preview {
                    Ok(preview) => self.filter_preview = Some(preview),
                    Err(e) => eprintln!("Filter preview unavailable: {e}"),
                }
                self.game = Some(Game::new(
                    assets,
                    self.options.paths.save.clone(),
                    self.options.audio,
                ));
                self.level_choices();
            }
            Err(e) => self.status = Some(format!("Game data could not be loaded: {e}")),
        }
    }

    fn navigate(&mut self, ctx: &Context, screen: Screen) {
        if self.screen == Screen::Editor && screen != Screen::Editor {
            self.finish_editor_gesture();
        }
        self.screen = screen;
        self.capture = None;
        self.reset_armed = false;
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
                let (w, h) = self.settings.view();
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(
                    (w as f32 * f32::from(scale) / pixels_per_point).max(480.0),
                    (h as f32 * f32::from(scale) / pixels_per_point + 48.0).max(400.0),
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
        self.poll_dx();
        if self.screen != Screen::Game {
            if self.capture.is_some() || self.dx_picker.is_some() {
                self.menu.freeze();
            } else {
                self.menu.start(&mut self.input);
            }
        }

        if self.screen == Screen::Game || self.screen == Screen::Editor {
            self.toolbar(ctx);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::BLACK))
            .show(ctx, |ui| {
                if self.screen == Screen::Game {
                    self.game_view(ui);
                } else if self.screen == Screen::Editor {
                    self.editor_view(ctx, ui);
                } else {
                    self.launcher(ctx, ui);
                }
            });
        self.dx_picker_window(ctx);
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
        let stepped = self.step && focused;
        if stepped {
            ticks = 1;
        }
        self.step = false;
        if self.paused {
            self.input.suppress_until_release();
        }
        let mut submitted_frame = None;
        if let Some(game) = &mut self.game {
            game.session.set_view(self.settings.view());
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
        }
        // A screenshot is checked against this CPU frame, which must be a tick.
        let smooth = if self.settings.motion_smoothing
            && running
            && !stepped
            && self.options.screenshot.is_none()
        {
            Smooth {
                alpha: self.budget.fraction(),
                scale: pick_scale(self.settings.view(), self.effective_scale),
            }
        } else {
            Smooth::OFF
        };
        if let Some(image) = self.frame_image(smooth) {
            self.texture.set(image.clone(), TextureOptions::NEAREST);
            submitted_frame = Some(image);
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
                        self.screenshot_check =
                            submitted_frame.map(|frame| (frame, self.game_rect));
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
                } else {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
        if self.settings.motion_smoothing && running {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(8));
        }
    }

    /// The current game frame: DX Mode's colors on screens the installed DX
    /// data covers, the display filter everywhere else and on any DX failure.
    fn frame_image(&mut self, smooth: Smooth) -> Option<ColorImage> {
        let game = self.game.as_ref()?;
        let view = self.settings.view();
        let dx = self
            .dx_bundle
            .as_ref()
            .filter(|_| self.settings.dx_mode && game.session.dx_supported());
        if let Some(dx) = dx {
            match game
                .session
                .render_dx_frame_smooth(dx, self.settings.character, view, smooth)
            {
                Ok((pixels, scale)) => {
                    return Some(ColorImage::new(
                        [view.0 * scale, view.1 * scale],
                        pixels
                            .into_iter()
                            .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                            .collect(),
                    ))
                }
                Err(error) => self.dx_message = error,
            }
        }
        let (pixels, scale) = game.session.render_palette_frame_smooth(view, smooth);
        Some(colorized(
            &pixels,
            self.settings.filter,
            &game.session.assets.cartridge_header,
            (view.0 * scale, view.1 * scale),
        ))
    }

    fn toolbar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("SML2");
                if ui.button("Menu · Esc").clicked() {
                    self.navigate(ctx, Screen::Home);
                }
                if self.screen == Screen::Game && self.editor.playtesting
                    && ui.button("Return to editor · Esc").clicked() {
                    self.open_editor(ctx);
                }
                if ui.button("Settings").clicked() {
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
    }

    /// Home and Settings: the scrolling level behind a shell-plastic card.
    fn launcher(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        let area = ui.max_rect();
        let dim = if self.screen == Screen::Settings {
            90
        } else {
            30
        };
        match &mut self.backdrop {
            Some(backdrop) => backdrop.paint(ui, area, self.settings.filter, dim),
            None => {
                ui.painter().rect_filled(area, 0, theme::BEZEL);
            }
        }
        let content = egui::Rect::from_min_max(area.min, area.max - egui::vec2(0.0, 34.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    if self.screen == Screen::Settings {
                        self.settings_panel(ctx, ui);
                    } else {
                        self.home_panel(ctx, ui);
                    }
                });
                ui.add_space(16.0);
            });
        });
        use theme::Glyph;
        let back = if self.screen == Screen::Home && self.game.is_some() {
            "Resume · Esc"
        } else {
            "Back · Esc"
        };
        let mut hints = vec![
            (Glyph::Pad, "Choose"),
            (Glyph::A, "Confirm · Enter"),
            (Glyph::B, back),
        ];
        if self.screen == Screen::Settings {
            hints.push((Glyph::Shoulders, "Page · PgUp/PgDn"));
        } else {
            hints.push((Glyph::Start, "Menu"));
        }
        theme::hint_bar(ui, area, &hints);
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
            self.settings.view(),
        );
        self.effective_scale = size[0] / self.settings.view().0 as u32;
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
        self.paint_overlays(ui, rect);
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

    fn paint_overlays(&self, ui: &egui::Ui, rect: egui::Rect) {
        use sml2_runtime::render::overlay::OverlayKind;
        let Some(game) = &self.game else {
            return;
        };
        let view = self.settings.view();
        let painter = ui.painter().with_clip_rect(rect);
        let outline = |r: (i32, i32, i32, i32), color| {
            painter.rect_stroke(
                overlay_rect(rect, view, r),
                0.0,
                egui::Stroke::new(1.0_f32, color),
                egui::StrokeKind::Inside,
            );
        };
        if self.show_hitboxes {
            for b in game.session.overlay_boxes(view) {
                let color = match b.kind {
                    OverlayKind::Mario => Color32::GREEN,
                    OverlayKind::Object => Color32::YELLOW,
                    OverlayKind::Excluded => Color32::GRAY,
                };
                outline((b.x, b.y, b.w, b.h), color);
            }
        }
        if self.show_native_border {
            if let Some(r) = game.session.native_border(view) {
                outline(r, Color32::from_rgb(255, 0, 255));
            }
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

/// View pixels `(x, y, w, h)` to the screen rect of a game image drawn into `rect`.
fn overlay_rect(
    rect: egui::Rect,
    view: (usize, usize),
    (x, y, w, h): (i32, i32, i32, i32),
) -> egui::Rect {
    let scale = rect.width() / view.0 as f32;
    egui::Rect::from_min_size(
        rect.min + egui::vec2(x as f32, y as f32) * scale,
        egui::vec2(w as f32, h as f32) * scale,
    )
}

/// Keep the winning BG/OBJ source until its mapped shade reaches presentation.
fn colorized(
    frame: &[PalettePixel],
    filter: Filter,
    header: &[u8; 0x18],
    view: (usize, usize),
) -> ColorImage {
    let palette = filter.palette(header);
    let pixels = frame
        .iter()
        .map(|&pixel| {
            let [r, g, b] = palette.rgb(pixel);
            Color32::from_rgb(r, g, b)
        })
        .collect();
    ColorImage::new([view.0, view.1], pixels)
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        self.ui(ctx);
    }
    fn raw_input_hook(&mut self, _: &Context, input: &mut RawInput) {
        self.receive_input(input);
    }
    fn on_exit(&mut self) {
        self.finish_editor_gesture();
        self.persist();
    }
}
