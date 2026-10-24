use crate::{
    controllers::Controllers,
    game::Game,
    input::{Action, Button, InputState},
    library::{Library, Row},
    menu::{pressed, release_focus, Menu},
    settings::{Filter, Settings, VanillaInstall, Zoom},
    timing::{game_rectangle, TickBudget},
    vanilla_import,
};
use eframe::egui::{
    self, Color32, ColorImage, Context, RawInput, TextureHandle, TextureOptions, ViewportCommand,
};
use sml2_core::player_sim::PlayerKeys;
use sml2_core::video::{OutputMode, PalettePixel};
use sml2_runtime::content::{ExtractedContent, SessionAssets};
use sml2_runtime::render::motion::{pick_scale, Smooth};
#[cfg(test)]
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod castle_backdrop;
mod catalog;
mod controls;
mod debug;
mod dx;
mod editor;
mod file_picker;
mod filter_preview;
mod hall;
mod installation;
mod launcher;
mod panels;
mod pointer;
mod startup;
pub(crate) mod theme;
pub use startup::{run, Options};
#[cfg(test)]
mod dx_tests;
#[cfg(test)]
mod library_tests;
mod screenshot;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    launcher: Option<launcher::Launcher>,
    dx_bundle: Option<sml2_dx::Presentation>,
    dx_worker: Option<crate::dx_import::Worker>,
    dx_picker: Option<file_picker::FilePicker>,
    dx_message: String,
    texture: TextureHandle,
    castle_backdrop: Option<castle_backdrop::CastleBackdrop>,
    screen: Screen,
    return_screen: Screen,
    /// The settings tab last shown.
    page: Page,
    /// "Reset all settings" was pressed once and waits for confirmation.
    reset_armed: bool,
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
    aim: Option<pointer::Aim>,
    start_click: bool,
    hall: Option<hall::Hall>,
    prev_hall_a: bool,
    prev_hall_left: bool,
    validation_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    library: Library,
    vanilla_check: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    library_dx_import: bool,
    /// The active worker is restoring a saved install. Import failures must not use this path.
    dx_restore: bool,
    /// Which library row is waiting on the system dialog or the in-app browser.
    library_pick: Option<LibraryPick>,
}

enum LibraryPick {
    Rom,
    Dx,
}

impl DesktopApp {
    fn new(ctx: &Context, options: Options, settings: Settings, error: Option<String>) -> Self {
        ctx.set_visuals(egui::Visuals::dark());
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let texture = ctx.load_texture(
            "sml2-frame",
            ColorImage::from_gray([160, 144], &[0; 160 * 144]),
            TextureOptions::NEAREST,
        );
        let mut app = Self {
            options,
            saved_settings: settings.clone(),
            settings,
            allow_save: error.is_none(),
            status: error,
            game: None,
            launcher: None,
            dx_bundle: None,
            dx_worker: None,
            dx_picker: None,
            dx_message: String::new(),
            texture,
            castle_backdrop: None,
            screen: Screen::Home,
            return_screen: Screen::Home,
            page: Page::Display,
            reset_armed: false,
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
            aim: None,
            start_click: false,
            hall: None,
            prev_hall_a: false,
            prev_hall_left: false,
            validation_error: Default::default(),
            library: Library::default(),
            vanilla_check: None,
            library_dx_import: false,
            dx_restore: false,
            library_pick: None,
        };
        app.library.dx_installed = app.settings.dx_installation.is_some();
        app.library.dx_mode = app.settings.dx_mode;
        if !app.options.launch_library {
            app.load_game();
            app.restore_dx();
        } else if app.settings.vanilla.is_some() {
            app.start_vanilla_check();
        } else {
            app.load_launcher();
        }
        if app.game.is_some() {
            app.screen = Screen::Game;
            if app.options.boot_select {
                app.game.as_mut().unwrap().session.enter_file_select();
            } else if app.options.boot_title || !app.options.play {
                app.game.as_mut().unwrap().session.enter_title();
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
        }
        if let Some(scale) = app.options.scale {
            app.zoom(ctx, Some(scale));
        }
        app
    }

    fn load_game(&mut self) {
        match SessionAssets::load_from(&ExtractedContent::new(&self.options.assets)) {
            Ok(assets) => {
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

    fn start_vanilla_check(&mut self) {
        let Some(expected) = self.settings.vanilla.clone() else {
            return;
        };
        let dir = match vanilla_import::vanilla_cache() {
            Ok(dir) => dir,
            Err(error) => {
                self.library.checking = false;
                self.library.vanilla_ready = false;
                self.library.rom_error = error;
                return;
            }
        };
        self.library.checking = true;
        self.library.vanilla_ready = false;
        self.library.rom_error.clear();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = vanilla_import::check_install(&dir, &expected).and_then(|()| {
                sml2_extract::vanilla::REVISIONS
                    .iter()
                    .find(|revision| revision.id == expected.revision)
                    .map(|revision| revision.name.to_string())
                    .ok_or_else(|| "The vanilla install does not match the saved identity.".into())
            });
            let _ = sender.send(result);
        });
        self.vanilla_check = Some(receiver);
    }

    fn poll_imports(&mut self) {
        let watching = self.library_dx_import && self.dx_worker.is_some();
        let dx_mode = self.settings.dx_mode;
        let dx_installation = self.settings.dx_installation.clone();
        self.poll_dx();
        self.poll_vanilla_check();
        if watching && self.dx_worker.is_none() {
            self.finish_library_dx(dx_mode, dx_installation);
        }
    }

    fn poll_vanilla_check(&mut self) {
        let ready = match self.vanilla_check.as_ref() {
            Some(receiver) => match receiver.try_recv() {
                Ok(result) => Some(Ok(result)),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(Err(())),
            },
            None => None,
        };
        let Some(ready) = ready else {
            return;
        };
        self.vanilla_check = None;
        self.library.checking = false;
        match ready {
            Ok(Ok(name)) => {
                self.library.vanilla_ready = true;
                self.library.revision_name = name;
                self.library.rom_error.clear();
                if self.options.launch_library {
                    self.load_launcher();
                    self.restore_dx();
                }
            }
            Ok(Err(error)) => {
                self.library.vanilla_ready = false;
                self.library.rom_error = error;
            }
            Err(()) => {
                self.library.vanilla_ready = false;
                self.library.rom_error = "The vanilla check stopped before it finished.".into();
            }
        }
    }

    fn finish_library_dx(
        &mut self,
        dx_mode: bool,
        dx_installation: Option<sml2_dx::cache::CacheKey>,
    ) {
        self.library.importing = false;
        self.library_dx_import = false;
        if self.dx_message.starts_with("Installed ") {
            self.settings.dx_mode = true;
            self.library.dx_mode = true;
            self.library.dx_installed = self.settings.dx_installation.is_some();
            self.library.dx_error.clear();
            match self.settings.save(&self.options.paths.settings) {
                Ok(()) => {
                    self.saved_settings = self.settings.clone();
                    self.allow_save = true;
                }
                Err(error) => {
                    self.status = Some(format!("Settings could not be saved: {error}"));
                }
            }
        } else {
            self.settings.dx_mode = dx_mode;
            self.settings.dx_installation = dx_installation.clone();
            self.library.dx_mode = dx_mode;
            self.library.dx_installed = dx_installation.is_some();
            if !self.dx_message.is_empty() {
                self.library.dx_error = self.dx_message.clone();
            }
        }
    }

    fn install_vanilla_rom(&mut self, rom: &[u8]) {
        if self.library.importing || self.library.checking {
            return;
        }
        self.library.importing = true;
        let dest = match vanilla_import::vanilla_cache() {
            Ok(dest) => dest,
            Err(error) => {
                self.library.importing = false;
                self.library.rom_error = error;
                return;
            }
        };
        let commit = match vanilla_import::install_rom(&dest, rom) {
            Ok(commit) => commit,
            Err(error) => {
                self.library.importing = false;
                self.library.rom_error = error;
                return;
            }
        };
        let revision = match sml2_extract::vanilla::identify(rom) {
            Ok(revision) => revision,
            Err(error) => {
                if let Err(rollback_error) = sml2_extract::vanilla::rollback(&dest, &commit) {
                    self.status = Some(vanilla_import::publish_error(rollback_error));
                }
                self.library.importing = false;
                self.library.rom_error = vanilla_import::identify_error(error);
                return;
            }
        };
        let previous_settings = self.settings.clone();
        let previous_bundle = self.dx_bundle.clone();
        let previous_dx_error = self.library.dx_error.clone();
        let previous_dx_message = self.dx_message.clone();
        self.settings.vanilla = Some(VanillaInstall {
            revision: revision.id.to_string(),
            sha256: sml2_extract::vanilla::sha256(rom),
        });
        let remove_dx_after_save = self.mark_dx_for_vanilla(&dest);
        match self.settings.save(&self.options.paths.settings) {
            Ok(()) => {
                sml2_extract::vanilla::discard_backup(&commit);
                self.saved_settings = self.settings.clone();
                self.allow_save = true;
                if remove_dx_after_save {
                    self.delete_retired_dx();
                }
                self.library.importing = false;
                self.library.checking = false;
                self.library.vanilla_ready = true;
                self.library.revision_name = revision.name.to_string();
                self.library.rom_error.clear();
                self.library.dx_installed = self.settings.dx_installation.is_some();
                self.library.dx_mode = self.settings.dx_mode;
                if self.options.launch_library {
                    self.load_launcher();
                }
            }
            Err(error) => {
                let mut message = format!("Settings could not be saved: {error}");
                if let Err(rollback_error) = sml2_extract::vanilla::rollback(&dest, &commit) {
                    message = format!(
                        "{message} {}",
                        vanilla_import::publish_error(rollback_error)
                    );
                }
                self.settings = previous_settings;
                self.dx_bundle = previous_bundle;
                self.library.dx_error = previous_dx_error;
                self.dx_message = previous_dx_message;
                self.library.dx_installed = self.settings.dx_installation.is_some();
                self.library.dx_mode = self.settings.dx_mode;
                self.library.importing = false;
                self.library.rom_error = message.clone();
                self.status = Some(message);
            }
        }
    }

    /// Record how the published vanilla tree relates to the installed DX key.
    ///
    /// Returns whether that DX directory should be deleted after settings save.
    /// This never deletes files: a failed save must still be able to roll vanilla back.
    fn mark_dx_for_vanilla(&mut self, dest: &std::path::Path) -> bool {
        let Some(base_sha256) = self
            .settings
            .dx_installation
            .as_ref()
            .map(|key| key.base_sha256.clone())
        else {
            return false;
        };
        let fingerprint = match sml2_dx::base_fingerprint(dest) {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                self.settings.dx_mode = false;
                self.library.dx_mode = false;
                let message = error.to_string();
                self.library.dx_error = message.clone();
                self.dx_message = message;
                return false;
            }
        };
        if fingerprint == base_sha256 {
            return false;
        }
        self.settings.dx_mode = false;
        self.library.dx_mode = false;
        true
    }

    fn delete_retired_dx(&mut self) {
        let Some(key) = self.settings.dx_installation.clone() else {
            return;
        };
        match crate::dx_import::cache_root().and_then(|root| {
            let dir = root.join(key.directory_name());
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.to_string()),
            }
        }) {
            Ok(()) => {
                self.settings.dx_installation = None;
                self.dx_bundle = None;
                match self.settings.save(&self.options.paths.settings) {
                    Ok(()) => self.saved_settings = self.settings.clone(),
                    Err(error) => {
                        self.status = Some(format!("Settings could not be saved: {error}"));
                    }
                }
            }
            Err(error) => {
                let message = format!("The old DX install could not be removed: {error}");
                self.library.dx_error = message.clone();
                self.dx_message = message;
            }
        }
    }

    fn play_library(&mut self) {
        if !self.load_library_game() {
            return;
        }
        self.options.launch_library = false;
        self.editor.playtesting = false;
        if let Some(game) = self.game.as_mut() {
            game.session.enter_title();
        }
        self.screen = Screen::Game;
    }

    fn load_library_game(&mut self) -> bool {
        if !self.library.play_enabled() {
            return false;
        }
        if self.game.is_some() {
            return true;
        }
        let assets = match vanilla_import::vanilla_cache() {
            Ok(path) => path,
            Err(error) => {
                self.library.rom_error = error;
                return false;
            }
        };
        self.options.assets = assets;
        self.load_game();
        if self.game.is_none() {
            if let Some(error) = self.status.clone() {
                self.library.rom_error = error;
            }
            return false;
        }
        if self.settings.dx_mode {
            self.restore_dx();
        }
        true
    }

    fn quit_library_on_escape(&mut self, ctx: &Context) {
        if self.dx_picker.is_some() || self.capture.is_some() {
            return;
        }
        if self.screen != Screen::Home || !self.options.launch_library {
            return;
        }
        // `pressed` consumes Esc. Exit and Esc share one quit, including while import disables it.
        if pressed(&mut self.input, &[Button::key("Escape")]) && self.library.row_enabled(Row::Exit)
        {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn accept_library_file(&mut self, path: std::path::PathBuf) {
        let kind = self.library_pick.take();
        self.dx_picker = None;
        self.input.suppress_until_release();
        self.menu = Default::default();
        match kind {
            Some(LibraryPick::Rom) => self.import_library_rom(path),
            Some(LibraryPick::Dx) => self.import_library_dx(path),
            None => self.import_dx_path(path),
        }
    }

    fn import_library_rom(&mut self, path: std::path::PathBuf) {
        if self.library.importing {
            return;
        }
        match vanilla_import::read_rom(&path) {
            Ok(rom) => self.install_vanilla_rom(&rom),
            Err(error) => self.library.rom_error = error,
        }
    }

    fn import_library_dx(&mut self, path: std::path::PathBuf) {
        if !self.can_import_dx() || self.dx_worker.is_some() {
            return;
        }
        let assets = match if self.game.is_some() {
            Ok(self.options.assets.clone())
        } else {
            vanilla_import::vanilla_cache()
        } {
            Ok(assets) => assets,
            Err(error) => {
                self.dx_message = error.clone();
                self.library.dx_error = error;
                return;
            }
        };
        self.library.importing = true;
        self.library_dx_import = true;
        self.dx_restore = false;
        self.dx_worker = Some(crate::dx_import::Worker::start(move |cancel, progress| {
            crate::dx_import::presentation(crate::dx_import::import_path(
                &crate::dx_import::cache_root()?,
                &assets,
                &path,
                cancel,
                progress,
            ))
        }));
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
        self.aim = None;
        self.start_click = false;
        if screen == Screen::Editor
            || !self.game.as_ref().is_some_and(|game| {
                matches!(
                    game.session.state,
                    sml2_runtime::session::SessionState::FileSelect(_)
                )
            })
        {
            self.hall = None;
        }
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

    /// Fit follows window resizes; switching it off selects the nearest manual scale
    /// without changing the window dimensions or the logical game view.
    fn fit_to_window(&mut self, enabled: bool) {
        self.settings.zoom = if enabled {
            Zoom::Auto
        } else {
            Zoom::Integer(self.effective_scale.clamp(1, 8) as u8)
        };
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
        self.quit_library_on_escape(ctx);
        self.shortcuts(ctx);
        self.poll_imports();
        if self.screen != Screen::Game {
            if self.capture.is_some() || self.dx_picker.is_some() {
                self.menu.freeze();
            } else if self.options.launch_library && self.screen == Screen::Home {
                self.menu.start_horizontal(&mut self.input);
            } else {
                self.menu.start(&mut self.input);
            }
        }

        if self.settings.show_toolbar
            && self.game.is_some()
            && (!self.options.launch_library
                || matches!(self.screen, Screen::Editor | Screen::Game))
        {
            self.toolbar(ctx);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::BLACK))
            .show(ctx, |ui| {
                if self.screen == Screen::Editor {
                    self.editor_view(ctx, ui);
                } else if self.options.launch_library
                    && matches!(self.screen, Screen::Home | Screen::Settings)
                {
                    self.launcher_view(ui);
                } else if self.game.is_some() {
                    self.game_view(ui);
                }
            });
        if self.screen == Screen::Settings
            || self.screen == Screen::Home && !self.options.launch_library
        {
            self.pause_menu(ctx);
        }
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
            && self.dx_picker.is_none()
            && !self.screenshot_pending;
        let mut ticks = self.budget.advance(elapsed, running);
        let stepped = self.step && focused && self.screen == Screen::Game;
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
        }
        let volume = if self.settings.muted {
            0.0
        } else {
            self.settings.volume
        };
        for _ in 0..ticks {
            let snap = self.front_snap();
            let mut clear_aim = false;
            let mut clear_start = false;
            if self.game.is_some() {
                if let Some(game) = self.game.as_mut() {
                    dx::prepare_tick(&mut game.session, &self.settings, self.dx_bundle.is_some());
                }
                let mut keys = if running {
                    self.input.sample_player(&self.settings.bindings)
                } else {
                    PlayerKeys::default()
                };
                let merged = pointer::merge_pointer(&snap, &keys, self.aim, self.start_click);
                keys = merged.keys;
                clear_aim = merged.clear_aim;
                clear_start = merged.clear_start;
                if let Some(mut hall) = self.hall.take() {
                    let jump = keys.a && !self.prev_hall_a;
                    self.prev_hall_a = keys.a;
                    let fullscreen_was = self.settings.fullscreen;
                    let zoom_was = self.settings.zoom;
                    let leave = hall::step(
                        &mut hall,
                        keys.left,
                        keys.right,
                        jump,
                        &mut self.settings,
                        &mut self.debug,
                    );
                    if self.settings.fullscreen != fullscreen_was {
                        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(
                            self.settings.fullscreen,
                        ));
                    }
                    if self.settings.zoom != zoom_was {
                        let scale = match self.settings.zoom {
                            Zoom::Auto => None,
                            Zoom::Integer(scale) => Some(scale),
                        };
                        self.zoom(ctx, scale);
                    }
                    if leave {
                        self.persist();
                    } else {
                        self.hall = Some(hall);
                    }
                    keys = PlayerKeys::default();
                } else if let Some(game) = self.game.as_ref() {
                    if let sml2_runtime::session::SessionState::FileSelect(file_select) =
                        &game.session.state
                    {
                        if file_select.menu_state() == 1
                            && file_select.cursor().0 == 0x28
                            && keys.left
                            && !self.prev_hall_left
                        {
                            self.hall = Some(hall::Hall::enter());
                            keys.left = false;
                        }
                        self.prev_hall_left = keys.left;
                    }
                }
                if let Some(game) = self.game.as_mut() {
                    // The hall may have changed Character after prepare_tick.
                    // Apply that, then let the file-select door win and save it.
                    game.session.character = self.settings.character;
                    game.tick(keys, volume);
                    self.settings.character = game.session.character;
                }
            }
            if clear_aim {
                self.aim = None;
            }
            if clear_start {
                self.start_click = false;
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
        if self.screen != Screen::Editor {
            if let Some(image) = self.frame_image(smooth) {
                self.texture.set(image.clone(), TextureOptions::NEAREST);
                submitted_frame = Some(image);
            }
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
        } else if self.screen == Screen::Editor && self.dx_worker.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        } else if self.screen != Screen::Editor {
            ctx.request_repaint_after(Duration::from_millis(8));
        }
    }

    /// The current game frame: DX Mode's colors on screens the installed DX
    /// data covers, the display filter everywhere else and on any DX failure.
    fn frame_image(&mut self, smooth: Smooth) -> Option<ColorImage> {
        self.frame_image_with_view(self.settings.view(), smooth)
    }

    fn frame_image_with_view(
        &mut self,
        view: (usize, usize),
        smooth: Smooth,
    ) -> Option<ColorImage> {
        let game = self.game.as_ref()?;
        if let sml2_runtime::session::SessionState::FileSelect(file_select) = &game.session.state {
            let mut doorway = hall::Hall::enter();
            doorway.camera = i32::from(file_select.cursor().0.saturating_sub(152));
            let hall = self.hall.as_ref().unwrap_or(&doorway);
            let on_door = hall.mario == hall::DOOR && hall.walk == 0 && hall.hop == 0;
            let dx = self
                .dx_bundle
                .as_ref()
                .filter(|_| self.settings.dx_mode && game.session.dx_supported());
            if let Some(dx) = dx {
                let frame = if on_door {
                    game.session
                        .render_dx_file_select_room(dx, self.settings.character, view)
                } else {
                    game.session.render_dx_hall_view(
                        dx,
                        self.settings.character,
                        view,
                        hall.camera,
                        hall::presentation(hall),
                    )
                };
                match frame {
                    Ok(mut rgb) => {
                        let palette = &dx.file_select.bg_palettes;
                        let ink = sml2_core::hardware_palettes::rgb555(u16::from_le_bytes([
                            palette[6], palette[7],
                        ]));
                        hall::stamp_signs(
                            &mut rgb,
                            hall,
                            &game.session.assets.front_end,
                            hall::alphabet(&game.session.assets),
                            &self.settings,
                            self.debug,
                            ink,
                        );
                        return Some(ColorImage::new(
                            [view.0, view.1],
                            rgb.into_iter()
                                .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                                .collect(),
                        ));
                    }
                    Err(error) => self.dx_message = error,
                }
            }
            let mut indices = if on_door {
                sml2_core::front_end::render::render_file_select_room_output(
                    file_select,
                    &game.session.slots,
                    &game.session.assets.front_end,
                    &game.session.assets.object_tables,
                    view,
                    OutputMode::PaletteIndices,
                )
            } else {
                hall::pixels(
                    hall,
                    &game.session.assets.front_end,
                    &game.session.assets.object_tables,
                    hall::alphabet(&game.session.assets),
                    &self.settings,
                    self.debug,
                )
            };
            if on_door {
                hall::stamp_signs(
                    &mut indices,
                    hall,
                    &game.session.assets.front_end,
                    hall::alphabet(&game.session.assets),
                    &self.settings,
                    self.debug,
                    3,
                );
            }
            let pixels: Vec<_> = indices.into_iter().map(PalettePixel::from_token).collect();
            return Some(colorized(
                &pixels,
                self.settings.filter,
                &game.session.assets.cartridge_header,
                view,
            ));
        }
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
            ui.horizontal_wrapped(|ui| {
                ui.strong("SML2");
                if ui.button("Menu · Esc").clicked() {
                    self.navigate(ctx, Screen::Home);
                }
                if self.screen == Screen::Game
                    && self.editor.playtesting
                    && ui.button("Return to editor · Esc").clicked()
                {
                    self.open_editor(ctx);
                }
                for page in Page::ALL {
                    let active = self.screen == Screen::Settings && self.page == page;
                    if ui.selectable_label(active, page.label()).clicked() {
                        if self.screen != Screen::Settings {
                            self.return_screen = self.screen;
                        }
                        self.page = page;
                        self.navigate(ctx, Screen::Settings);
                    }
                }
            });
        });
    }

    /// Pause and settings popups share the actual game's paused frame.
    fn pause_menu(&mut self, ctx: &Context) {
        let settings = self.screen == Screen::Settings;
        let title = if settings { "Settings" } else { "Menu" };
        egui::Window::new(title)
            .id(egui::Id::new(if settings {
                "settings-popup"
            } else {
                "pause-menu"
            }))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(if settings { theme::ROW_SIZE.x } else { 260.0 })
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 100.0).max(160.0))
                    .show(ui, |ui| {
                        if settings {
                            self.settings_panel(ctx, ui);
                        } else {
                            self.home_panel(ctx, ui);
                        }
                    });
                ui.add_space(6.0);
                theme::note(
                    ui,
                    if settings {
                        "Esc: back · L / R: page"
                    } else {
                        "Esc / Start: resume"
                    },
                );
            });
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
            self.settings.integer_scaling,
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
        self.paint_castle_backdrop(ui, rect, available);
        ui.painter().image(
            self.texture.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        self.paint_overlays(ui, rect);
        let picture = ui.interact(rect, egui::Id::new("game-picture"), egui::Sense::click());
        if self.screen == Screen::Game && self.dx_picker.is_none() && picture.clicked() {
            if let Some(pos) = picture.interact_pointer_pos() {
                if let Some(hall) = &mut self.hall {
                    let view = self.settings.view();
                    if let Some(pixel) = pointer::view_pixel(rect, view, pos) {
                        if let Some(index) =
                            hall::pedestal_at(pixel, hall, &self.settings, self.debug)
                        {
                            hall.goal = Some(index);
                        }
                    }
                } else {
                    self.on_picture_click(pos, rect);
                }
            }
        }
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
