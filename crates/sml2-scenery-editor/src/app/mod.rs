//! Standalone authoring host. Its game session never opens a progress save.

use crate::input::{Button, InputState, PadButton};
use crate::settings::{Filter, Settings};
use eframe::egui::{self, Color32, ColorImage, Context, RawInput, TextureHandle, TextureOptions};
#[cfg(test)]
pub(super) use sml2_desktop::ui::screenshot;
pub(super) use sml2_desktop::ui::{colorized, theme};
use sml2_desktop::{controllers::Controllers, game::Game, timing::TickBudget};
use sml2_runtime::{
    content::{ExtractedContent, SessionAssets},
    render::motion::Smooth,
};
use std::time::{Duration, Instant};

mod editor;
mod imports;
mod startup;
pub use startup::{run, Options};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Home,
    Editor,
    Game,
}

pub struct EditorApp {
    options: Options,
    settings: Settings,
    status: Option<String>,
    allow_save: bool,
    game: Option<Game>,
    dx_bundle: Option<sml2_dx::Presentation>,
    dx_worker: Option<crate::dx_import::Worker>,
    rom_worker: Option<std::sync::mpsc::Receiver<imports::RomResult>>,
    dx_restore: bool,
    import_kind: imports::ImportKind,
    input_path: String,
    screen: Screen,
    editor: editor::Editor,
    levels: Vec<(u8, String)>,
    texture: TextureHandle,
    input: InputState,
    controllers: Controllers,
    debug: bool,
    paused: bool,
    budget: TickBudget,
    last_update: Instant,
    redraws: u64,
}

impl EditorApp {
    fn new(ctx: &Context, options: Options, settings: Settings, error: Option<String>) -> Self {
        ctx.set_visuals(egui::Visuals::dark());
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let mut app = Self {
            options,
            settings,
            allow_save: error.is_none(),
            status: error,
            game: None,
            dx_bundle: None,
            dx_worker: None,
            rom_worker: None,
            dx_restore: false,
            import_kind: imports::ImportKind::Rom,
            input_path: String::new(),
            screen: Screen::Home,
            editor: Default::default(),
            levels: Vec::new(),
            texture: ctx.load_texture(
                "scenery-playtest",
                ColorImage::filled([160, 144], Color32::BLACK),
                TextureOptions::NEAREST,
            ),
            input: Default::default(),
            controllers: if cfg!(test) {
                Controllers::disconnected()
            } else {
                Controllers::new()
            },
            debug: false,
            paused: false,
            budget: Default::default(),
            last_update: Instant::now(),
            redraws: 0,
        };
        app.load_game();
        if app.game.is_some() {
            app.open_editor(ctx);
        }
        app
    }

    fn load_game(&mut self) {
        if !self.options.explicit_assets {
            if let Some(expected) = &self.settings.vanilla {
                match crate::vanilla_import::vanilla_cache().and_then(|path| {
                    crate::vanilla_import::check_install(&path, expected)?;
                    Ok(path)
                }) {
                    Ok(path) => self.options.assets = path,
                    Err(error) => {
                        self.status = Some(error);
                        return;
                    }
                }
            }
        }
        match SessionAssets::load_from(&ExtractedContent::new(&self.options.assets)) {
            Ok(assets) => {
                self.levels = (0..32)
                    .filter_map(|id| {
                        assets.level(id).ok().map(|content| {
                            (id, format!("{id:02} · {}", content.manifest.graphics.file))
                        })
                    })
                    .collect();
                self.game = Some(Game::preview(assets, self.options.audio));
            }
            Err(error) => self.status = Some(format!("Game data could not be loaded: {error}")),
        }
    }

    fn navigate(&mut self, ctx: &Context, screen: Screen) {
        if self.screen == Screen::Editor && screen != Screen::Editor {
            self.finish_editor_gesture();
        }
        self.screen = screen;
        self.input.suppress_until_release();
        self.budget.reset();
        if let Some(game) = &mut self.game {
            game.audio_running(false);
        }
        sml2_desktop::ui::release_focus(ctx);
    }

    fn cycle_filter(&mut self, delta: i32) {
        self.settings.filter = self.settings.filter.cycle(delta);
    }

    fn receive_input(&mut self, raw: &RawInput) {
        for event in &raw.events {
            match event {
                egui::Event::Key {
                    key,
                    physical_key,
                    pressed,
                    repeat,
                    ..
                } if !repeat || !pressed => {
                    self.input.event(
                        Button::key(&format!("{:?}", physical_key.unwrap_or(*key))),
                        *pressed,
                    );
                }
                egui::Event::WindowFocused(false) => self.input.clear(),
                _ => {}
            }
        }
    }

    fn shortcuts(&mut self, ctx: &Context) {
        if self.editor_export_open() {
            return;
        }
        let back = self.input.take_press(&Button::key("Escape"))
            | self.input.take_press(&Button::Pad(PadButton::East));
        if self.screen == Screen::Editor {
            if back {
                if !self.cancel_editor_gesture() {
                    self.navigate(ctx, Screen::Home);
                }
                return;
            }
            self.editor_shortcuts(ctx);
        } else if self.screen == Screen::Game
            && (back || self.input.take_press(&Button::Pad(PadButton::Start)))
        {
            self.open_editor(ctx);
        }
        if self.input.take_press(&Button::key("F4")) {
            self.cycle_filter(1);
        }
        if self.input.take_press(&Button::key("F11")) {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }
    }

    fn ui(&mut self, ctx: &Context) {
        self.poll_imports(ctx);
        let focused = ctx.input(|i| i.focused);
        self.controllers.poll(
            &mut self.input,
            self.settings.preferred_controller.as_deref(),
            self.settings.dead_zone,
        );
        if !focused {
            self.input.clear();
            self.budget.reset();
        }
        self.shortcuts(ctx);
        egui::TopBottomPanel::top("editor-presentation").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Game data…").clicked() {
                    self.navigate(ctx, Screen::Home);
                }
                egui::ComboBox::from_id_salt("editor-filter")
                    .selected_text(self.settings.filter.label())
                    .show_ui(ui, |ui| {
                        for filter in Filter::ALL {
                            ui.selectable_value(&mut self.settings.filter, filter, filter.label());
                        }
                    });
                let dx_before = self.settings.dx_mode;
                ui.checkbox(&mut self.settings.dx_mode, "DX graphics");
                if self.settings.dx_mode
                    && !dx_before
                    && self.dx_bundle.is_none()
                    && self.dx_worker.is_none()
                {
                    self.restore_dx();
                }
                if self.screen == Screen::Game {
                    if ui.button("Return to editor · Esc").clicked() {
                        self.open_editor(ctx);
                    }
                    if ui.button("Restart").clicked() {
                        self.play_editor(ctx);
                    }
                    ui.checkbox(&mut self.paused, "Pause");
                }
            });
        });
        let elapsed = self.last_update.elapsed();
        self.last_update = Instant::now();
        let running = self.screen == Screen::Game && focused && !self.paused;
        if let Some(game) = &mut self.game {
            game.session.set_view(self.settings.view());
            game.audio_running(running && !self.settings.muted);
            for _ in 0..self.budget.advance(elapsed, running) {
                game.session.character = self.settings.character;
                let keys = self.input.sample_player(&self.settings.bindings);
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
        egui::CentralPanel::default().show(ctx, |ui| match self.screen {
            Screen::Editor => self.editor_view(ctx, ui),
            Screen::Home => self.data_view(ctx, ui),
            Screen::Game => {
                if let Some(image) = self.frame_image(Smooth::OFF) {
                    let size = egui::vec2(image.size[0] as f32, image.size[1] as f32);
                    self.texture.set(image, TextureOptions::NEAREST);
                    let scale = (ui.available_width() / size.x)
                        .min(ui.available_height() / size.y)
                        .max(0.01);
                    ui.centered_and_justified(|ui| {
                        ui.image((self.texture.id(), size * scale));
                    });
                }
            }
        });
        self.export_window(ctx);
        if let Some(message) = &self.status {
            egui::TopBottomPanel::bottom("editor-message").show(ctx, |ui| {
                ui.label(message);
            });
        }
        self.redraws += 1;
        if self
            .options
            .smoke_frames
            .is_some_and(|count| self.redraws >= count)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if running {
            ctx.request_repaint_after(Duration::from_millis(8));
        } else if self.dx_worker.is_some() || self.rom_worker.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn frame_image(&mut self, smooth: Smooth) -> Option<ColorImage> {
        let game = self.game.as_ref()?;
        let view = self.settings.view();
        if let Some(bundle) = self.dx_bundle.as_ref().filter(|_| self.settings.dx_mode) {
            match game
                .session
                .render_dx_frame_smooth(bundle, self.settings.character, view, smooth)
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
                Err(error) => self.status = Some(format!("DX preview unavailable: {error}")),
            }
        }
        Some(colorized(
            &game.session.render_palette_frame(view),
            self.settings.filter,
            &game.session.assets.cartridge_header,
            view,
        ))
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        self.ui(ctx);
    }
    fn raw_input_hook(&mut self, _: &Context, raw: &mut RawInput) {
        self.receive_input(raw);
    }
    fn on_exit(&mut self) {
        self.finish_editor_gesture();
    }
}
