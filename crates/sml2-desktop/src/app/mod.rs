use crate::{
    controllers::{Controllers, DeviceChange},
    game::Game,
    input::{Action, Bindings, Button, ButtonClass, Controls, InputState, PadButton},
    library::{Library, Row},
    menu::{Menu, pressed, release_focus},
    players::{Connect, Device, Disconnect, DropOut, JoinSpot, Players},
    settings::{Settings, VanillaInstall, Zoom},
    timing::{TickBudget, game_rectangle},
    update, vanilla_import,
};
#[cfg(test)]
use eframe::egui::RawInput;
use eframe::egui::{
    self, Color32, ColorImage, Context, TextureHandle, TextureOptions, ViewportCommand,
};
use sml2_core::player_sim::PlayerKeys;
use sml2_runtime::content::{ExtractedContent, SessionAssets};
use sml2_runtime::render::motion::{Smooth, pick_scale};
use sml2_runtime::session::SessionState;
use std::collections::BTreeMap;
#[cfg(test)]
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod castle_backdrop;
mod controls;
mod debug;
mod dx;
mod file_picker;
mod filter_preview;
mod installation;
mod launcher;
mod panels;
mod pause_backdrop;
mod pointer;
mod startup;
mod update_ui;
pub(crate) use crate::ui::{colorized, theme};
pub use startup::{Options, run};
#[cfg(test)]
mod dx_tests;
#[cfg(test)]
mod library_tests;
pub(crate) use crate::ui::screenshot;
#[cfg(test)]
mod settings_layout_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod widescreen_tests;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Home,
    Game,
    Settings,
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
    dx_bundle: Option<std::sync::Arc<sml2_dx::Presentation>>,
    dx_worker: Option<crate::dx_import::Worker>,
    dx_picker: Option<file_picker::FilePicker>,
    dx_message: String,
    texture: TextureHandle,
    last_game_view: Option<pause_backdrop::GameView>,
    menu_from_game: bool,
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
    /// One input state per connected gamepad. `input` carries their aggregate
    /// for host shortcuts; gameplay samples each pad on its own.
    pads: BTreeMap<u64, InputState>,
    /// Menus and pause read only the devices that own a player (or every
    /// device before Player 1 is claimed), rebuilt each frame.
    ui_input: InputState,
    /// Who owns which input device this launch.
    players: Players,
    /// Debug-only fixed WASD Player 2, as the prototype always had.
    debug_wasd: bool,
    /// The device whose Start/Enter opened the pause menu; `None` when the
    /// menu is system-owned (toolbar, reconnect prompt).
    menu_owner: Option<Device>,
    /// The reconnect prompt pause menu is open.
    reconnect_pause: bool,
    /// A short "Player 2 joined/left" banner.
    banner: Option<(String, Instant)>,
    /// Player 2's fixed keyboard layout; only used by the debug WASD option.
    second_bindings: Bindings,
    /// Player 1 actions whose loaded buttons are reserved for debug Player 2.
    binding_conflicts: Vec<(Action, Vec<Button>)>,
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
    focused: bool,
    last_update: Instant,
    last_save: Instant,
    budget: TickBudget,
    effective_scale: u32,
    redraws: u64,
    screenshot_pending: bool,
    screenshot_check: Option<(ColorImage, [u32; 4])>,
    game_rect: [u32; 4],
    /// Live drawable game area; fullscreen and the toolbar can change its aspect.
    viewport_size: Option<[u32; 2]>,
    aim: Option<pointer::Aim>,
    start_click: bool,
    validation_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    library: Library,
    vanilla_check: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    library_dx_import: bool,
    /// The active worker is restoring a saved install. Import failures must not use this path.
    dx_restore: bool,
    /// Which library row is waiting on the system dialog or the in-app browser.
    library_pick: Option<LibraryPick>,
    /// Background release lookup, download and staged install.
    updater: update::Updater,
    /// The About window is open.
    about_open: bool,
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
            last_game_view: None,
            menu_from_game: false,
            castle_backdrop: None,
            screen: Screen::Home,
            return_screen: Screen::Home,
            page: Page::Display,
            reset_armed: false,
            filter_preview: None,
            input: InputState::default(),
            pads: BTreeMap::new(),
            ui_input: InputState::default(),
            players: Players::default(),
            debug_wasd: false,
            menu_owner: None,
            reconnect_pause: false,
            banner: None,
            second_bindings: Bindings::player_two(),
            binding_conflicts: Vec::new(),
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
            focused: false,
            last_update: Instant::now(),
            last_save: Instant::now(),
            budget: TickBudget::default(),
            effective_scale: 1,
            redraws: 0,
            screenshot_pending: false,
            screenshot_check: None,
            game_rect: [0; 4],
            viewport_size: None,
            aim: None,
            start_click: false,
            validation_error: Default::default(),
            library: Library::default(),
            vanilla_check: None,
            library_dx_import: false,
            dx_restore: false,
            library_pick: None,
            updater: update::Updater::new(),
            about_open: false,
        };
        app.library.dx_installed = app.settings.dx_installation.is_some();
        app.library.dx_mode = app.settings.dx_mode;
        if !app.options.launch_library {
            if app.settings.vanilla.is_some() {
                if let Ok(dir) = vanilla_import::vanilla_cache() {
                    if let Some(expected) = &app.settings.vanilla {
                        if vanilla_import::check_install(&dir, expected).is_ok() {
                            app.options.assets = dir;
                            app.library.vanilla_ready = true;
                        }
                    }
                }
            }
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
        } else if !app.options.launch_library {
            app.screen = Screen::Settings;
            app.page = Page::Game;
        }
        if let Some(scale) = app.options.scale {
            app.zoom(ctx, Some(scale));
        }
        app.refresh_binding_conflicts();
        if app.game.is_some() {
            app.show_binding_conflicts();
        }
        update::remove_previous_executable();
        app.check_updates(false);
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
                let game = Game::new(assets, self.options.paths.save.clone(), self.options.audio);
                // Every launch is single player until a second device joins.
                self.game = Some(game);
                self.level_choices();
                self.refresh_binding_conflicts();
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
                } else if self.game.is_none() {
                    self.options.assets = dest;
                    self.load_game();
                    self.restore_dx();
                    if let Some(game) = self.game.as_mut() {
                        game.session.enter_title();
                    }
                    self.screen = Screen::Game;
                    self.show_binding_conflicts();
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
        if let Some(game) = self.game.as_mut() {
            game.session.enter_title();
        }
        self.screen = Screen::Game;
        self.show_binding_conflicts();
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
        if self.dx_picker.is_some() || self.capture.is_some() || self.menu_from_game {
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
        if self.screen == Screen::Game && matches!(screen, Screen::Home | Screen::Settings) {
            self.menu_from_game = true;
        } else if screen == Screen::Game {
            self.menu_from_game = false;
        }
        self.screen = screen;
        self.capture = None;
        self.reset_armed = false;
        self.menu = Menu::default();
        self.input.suppress_until_release();
        for state in self.pads.values_mut() {
            state.suppress_until_release();
        }
        self.budget.reset();
        release_focus(ctx);
        self.aim = None;
        self.start_click = false;
    }

    /// Keep the loaded Player 1 bindings away from Player 2's reserved keys.
    /// Settings are never rewritten here; conflicts stay visible until fixed.
    fn refresh_binding_conflicts(&mut self) {
        self.binding_conflicts = if self.debug_wasd {
            self.settings.bindings.conflicts_with(&self.second_bindings)
        } else {
            Vec::new()
        };
    }

    /// Open the Controls page instead of play when loaded bindings collide.
    fn show_binding_conflicts(&mut self) {
        if self.binding_conflicts.is_empty() {
            return;
        }
        self.return_screen = Screen::Game;
        self.page = Page::Controls;
        self.screen = Screen::Settings;
        self.menu_from_game = self.game.is_some();
        self.menu = Menu::default();
        self.input.suppress_until_release();
        self.budget.reset();
        self.aim = None;
        self.start_click = false;
    }

    fn zoom(&mut self, ctx: &Context, scale: Option<u8>) {
        self.settings.zoom = scale.map_or(Zoom::Auto, Zoom::Integer);
        if let Some(scale) = scale {
            if !self.settings.fullscreen {
                let pixels_per_point = ctx.pixels_per_point();
                let (w, h) = self.view();
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

    fn show_banner(&mut self, text: impl Into<String>) {
        self.banner = Some((text.into(), Instant::now() + Duration::from_secs(2)));
    }

    fn connected_devices(&self) -> Vec<Device> {
        let mut devices = vec![Device::Keyboard];
        devices.extend(self.pads.keys().map(|id| Device::Pad(*id)));
        devices
    }

    /// Where a second device may join: the title screen or an overworld map.
    fn join_spot(&self) -> Option<JoinSpot> {
        if self.screen != Screen::Game {
            return None;
        }
        match &self.game.as_ref()?.session.state {
            SessionState::Title(_) => Some(JoinSpot::Title),
            // Bonus and coin rooms are map states with a room script; they
            // are not the overworld map.
            SessionState::Overworld(map) if map.room.is_none() => Some(JoinSpot::Map),
            _ => None,
        }
    }

    fn in_level(&self) -> bool {
        self.game
            .as_ref()
            .is_some_and(|game| matches!(game.session.state, SessionState::InLevel(_)))
    }

    fn on_title_screen(&self) -> bool {
        self.screen == Screen::Game
            && self
                .game
                .as_ref()
                .is_some_and(|game| matches!(game.session.state, SessionState::Title(_)))
    }

    fn apply_device_change(&mut self, ctx: &Context, change: DeviceChange) {
        match change {
            DeviceChange::Disconnected { id } => {
                let in_level = self.in_level();
                match self.players.disconnect(Device::Pad(id), in_level) {
                    Disconnect::PlayerOneNeedsReconnect => self.open_reconnect_pause(ctx),
                    Disconnect::PlayerTwoBubbled => {
                        if let Some(game) = &mut self.game {
                            game.session.bubble_second_player();
                        }
                    }
                    Disconnect::PlayerTwoLeft => {
                        if let Some(game) = &mut self.game {
                            game.session.set_local_multiplayer(false);
                        }
                        self.show_banner("Player 2 left");
                    }
                    _ => {}
                }
            }
            DeviceChange::Connected { id, .. } => match self.players.connect(Device::Pad(id)) {
                Connect::PlayerOne => {
                    if self.reconnect_pause {
                        self.reconnect_pause = false;
                        self.menu_owner = None;
                        self.navigate(ctx, Screen::Game);
                    }
                }
                Connect::PlayerTwo => {
                    if let Some(game) = &mut self.game {
                        game.session.resume_second_player();
                    }
                }
                Connect::None => {}
            },
        }
    }

    fn open_reconnect_pause(&mut self, ctx: &Context) {
        if self.reconnect_pause {
            return;
        }
        self.reconnect_pause = true;
        if self.game.is_none() {
            return;
        }
        if self.screen != Screen::Home {
            self.navigate(ctx, Screen::Home);
        }
        self.menu_owner = None;
    }

    /// The input stream menus and pause may read this frame. Before Player 1
    /// is claimed every device may act on the title; afterwards only the
    /// devices that own a player can reach a menu.
    fn build_ui_input(&self) -> InputState {
        if self.players.player_one_device().is_none() {
            return self.input.clone();
        }
        let mut view = InputState::default();
        for device in [self.players.player_one(), self.players.player_two()]
            .into_iter()
            .flatten()
        {
            match device {
                Device::Keyboard => view.merge_class(&self.input, ButtonClass::Keyboard),
                Device::Pad(id) => {
                    if let Some(state) = self.pads.get(&id) {
                        view.merge_class(state, ButtonClass::Pad);
                    }
                }
            }
        }
        view
    }

    /// Drop presses the filtered stream consumed, so they cannot reach the
    /// game tick or fire again next frame.
    fn commit_ui_input(&mut self) {
        self.input.retain_pressed(&self.ui_input);
        for state in self.pads.values_mut() {
            state.retain_pressed(&self.ui_input);
        }
    }

    /// Claim, join, leave and pause ownership for this frame.
    fn handle_players(&mut self, ctx: &Context) {
        let on_title = self.on_title_screen();
        let spot = self.join_spot();

        // The press that acts on the title claims Player 1, so a map opened
        // by that press (or by the menu confirm it produced) belongs to it.
        if on_title && self.players.player_one_device().is_none() {
            for device in self.connected_devices() {
                if pending_claim(&self.input, &self.pads, device) {
                    self.players.claim_player_one(device);
                    break;
                }
            }
        }
        // A launch with no title screen. On a file-select screen the device
        // that first acts is the one opening the map, so it claims Player 1;
        // when the map was opened by the pointer there is no device press and
        // the fallback takes the first pad in connection order, else the
        // keyboard.
        if self.screen == Screen::Game && !on_title && self.players.player_one_device().is_none() {
            let file_select = self.game.as_ref().is_some_and(|game| {
                matches!(
                    game.session.state,
                    SessionState::FileSelect(_) | SessionState::FileSelectLoading(_)
                )
            });
            if !file_select {
                self.players.resolve_initial();
            } else {
                for device in self.connected_devices() {
                    if pending_action(&self.input, &self.pads, device) {
                        self.players.claim_player_one(device);
                        break;
                    }
                }
            }
        }

        // A device that owns nothing may only join, and only on the title or
        // a map. Its join press is consumed everywhere else, so a press made
        // in a level cannot queue up and join on the next map.
        if let Some(player_one) = self.players.player_one_device() {
            let can_join = spot.is_some()
                && !self.debug_wasd
                && !self.players.two_players()
                && self.players.player_one().is_some();
            for device in self.connected_devices() {
                if device == player_one || Some(device) == self.players.player_two_device() {
                    continue;
                }
                if take_join_press(&mut self.input, &mut self.pads, device) && can_join {
                    if self.players.join(device) {
                        self.show_banner("Player 2 joined");
                    }
                    break;
                }
            }
        }

        // Player 2 can leave with Select on the title or an overworld map.
        if spot.is_some() && !self.debug_wasd && self.players.two_players() {
            if let Some(device) = self.players.player_two() {
                if take_leave_press(
                    &mut self.input,
                    &mut self.pads,
                    device,
                    &self.settings.bindings,
                ) {
                    if let Some(game) = &mut self.game {
                        game.session.set_local_multiplayer(false);
                    }
                    self.players.drop_out(Some(device));
                    self.show_banner("Player 2 left");
                    return;
                }
            }
        }

        // Either player's Start/Enter opens the pause menu anywhere.
        if self.screen == Screen::Game
            && !on_title
            && self.capture.is_none()
            && self.dx_picker.is_none()
        {
            let mut devices: Vec<Device> = [self.players.player_one(), self.players.player_two()]
                .into_iter()
                .flatten()
                .collect();
            if devices.is_empty() && self.debug_wasd {
                devices.push(Device::Keyboard);
            }
            let mut opener = None;
            for device in devices {
                if take_pause_press(
                    &mut self.input,
                    &mut self.pads,
                    device,
                    &self.settings.bindings,
                ) {
                    opener = Some(device);
                    break;
                }
            }
            if opener.is_none() && self.debug_wasd {
                // The debug keyboard Player 2 keeps its Num1 pause key.
                if take_device_action(
                    &mut self.input,
                    &mut self.pads,
                    Device::Keyboard,
                    &self.second_bindings,
                    Action::Start,
                ) {
                    opener = Some(Device::Keyboard);
                }
            }
            if let Some(device) = opener {
                self.menu_owner = Some(device);
                self.reconnect_pause = false;
                self.navigate(ctx, Screen::Home);
            }
        }
    }

    /// Apply the pause menu's Drop out to the menu's owner.
    fn drop_out(&mut self, ctx: &Context) {
        let owner = self.menu_owner.or_else(|| self.players.player_one_device());
        let Some(outcome) = self.players.drop_out(owner) else {
            return;
        };
        match outcome {
            DropOut::PlayerTwo => {
                if let Some(game) = &mut self.game {
                    game.session.set_local_multiplayer(false);
                }
                self.show_banner("Player 2 left");
            }
            DropOut::PlayerOnePromoted => {
                if let Some(game) = &mut self.game {
                    game.session.drop_out_player_one();
                    self.settings.character = game.session.character;
                }
                self.show_banner("Player 1 left");
            }
        }
        self.menu_owner = None;
        if self.players.player_one_needs_reconnect() {
            self.open_reconnect_pause(ctx);
        } else {
            self.reconnect_pause = false;
            self.navigate(ctx, Screen::Game);
            self.show_binding_conflicts();
        }
    }

    fn ui(&mut self, ctx: &Context) {
        // Collect the events from the frame that owns the menu. A discarded
        // layout pass must not feed the same keyboard press into it again.
        if ctx.current_pass_index() == 0 {
            self.receive_input(&ctx.input(|input| input.raw.clone()));
        }
        let focused = ctx.input(|i| i.focused && i.viewport().minimized != Some(true));
        let changes =
            self.controllers
                .poll_devices(&mut self.input, &mut self.pads, self.settings.dead_zone);
        for change in changes {
            self.apply_device_change(ctx, change);
        }
        if !focused {
            self.input.clear();
            for state in self.pads.values_mut() {
                state.clear();
            }
            self.budget.reset();
            self.last_update = Instant::now();
        } else if !self.focused {
            self.input.suppress_until_release();
            for state in self.pads.values_mut() {
                state.suppress_until_release();
            }
            self.budget.reset();
            self.last_update = Instant::now();
        }
        self.focused = focused;
        self.handle_players(ctx);
        self.quit_library_on_escape(ctx);
        self.ui_input = self.build_ui_input();
        self.shortcuts(ctx);
        self.commit_ui_input();
        self.ui_input = self.build_ui_input();
        self.poll_imports();
        let on_title = self.screen == Screen::Game
            && self.game.as_ref().is_some_and(|g| {
                matches!(
                    g.session.state,
                    sml2_runtime::session::SessionState::Title(_)
                )
            });
        if self.screen != Screen::Game || on_title {
            if self.capture.is_some() || self.dx_picker.is_some() {
                self.menu.freeze();
            } else if on_title
                || (self.options.launch_library
                    && !self.menu_from_game
                    && self.screen == Screen::Home)
            {
                self.menu.start_horizontal(&mut self.ui_input);
            } else {
                self.menu.start(&mut self.ui_input);
            }
        }

        if self.settings.show_toolbar {
            // The cold launcher is the title scene itself; a help bar would
            // shrink it and repeat actions the grass already shows.
            let cold_launcher = self.game.is_none()
                && self.options.launch_library
                && !self.menu_from_game
                && self.screen == Screen::Home;
            if self.game.is_some()
                && (!self.options.launch_library
                    || self.menu_from_game
                    || self.screen == Screen::Game)
            {
                self.toolbar(ctx);
            } else if !cold_launcher {
                egui::TopBottomPanel::top("help-bar").show(ctx, |ui| self.help_menu(ui));
            }
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::BLACK))
            .show(ctx, |ui| {
                let available = ui.available_size() * ctx.pixels_per_point();
                let size = [available.x.floor() as u32, available.y.floor() as u32];
                if !size.contains(&0) {
                    self.viewport_size = Some(size);
                }
                if self.menu_from_game && matches!(self.screen, Screen::Home | Screen::Settings) {
                    self.paused_game_view(ui);
                } else if self.options.launch_library
                    && matches!(self.screen, Screen::Home | Screen::Settings)
                {
                    self.launcher_view(ui);
                    if self.screen == Screen::Settings {
                        ui.painter()
                            .rect_filled(ui.max_rect(), 0, Color32::from_black_alpha(150));
                    }
                } else if self.game.is_some() {
                    if self.screen == Screen::Game {
                        self.remember_game_view(ui);
                    } else {
                        self.paused_game_view(ui);
                    }
                }
            });
        if self.screen == Screen::Settings
            || self.screen == Screen::Home && (!self.options.launch_library || self.menu_from_game)
        {
            self.pause_menu(ctx);
        }
        self.dx_picker_window(ctx);
        let on_launcher =
            self.screen == Screen::Home && self.options.launch_library && !self.menu_from_game;
        let in_play = self.screen == Screen::Game && !on_title;
        self.update_notice(ctx, on_title || on_launcher, in_play);
        self.about_window(ctx);
        if self.screen != Screen::Game || on_title {
            self.menu.finish();
        }
        self.commit_ui_input();
        if self.debug {
            self.debug_panel(ctx);
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;
        let running = focused
            && self.screen == Screen::Game
            && self.binding_conflicts.is_empty()
            && !self.paused
            && self.capture.is_none()
            && self.dx_picker.is_none()
            && !self.screenshot_pending;
        let mut ticks = self.budget.advance(elapsed, running);
        let stepped = self.step
            && focused
            && self.screen == Screen::Game
            && self.binding_conflicts.is_empty();
        if stepped {
            ticks = 1;
        }
        self.step = false;
        if self.paused {
            self.input.suppress_until_release();
        }
        let mut submitted_frame = None;
        let view = self.view();
        if let Some(game) = &mut self.game {
            game.session.set_view(view);
            game.audio_running(running && !self.settings.muted);
        }
        let volume = self.settings.audio_volume();
        let two_players = self.players.two_players() || self.debug_wasd;
        if let Some(game) = &mut self.game {
            // Level entry builds Player 2 only when two players are present.
            if game.session.local_multiplayer() != two_players {
                game.session.set_local_multiplayer(two_players);
            }
            if self.players.player_two_waiting() {
                game.session.bubble_second_player();
            }
        }
        for _ in 0..ticks {
            let snap = self.front_snap();
            let mut clear_aim = false;
            let mut clear_start = false;
            if self.game.is_some() {
                if let Some(game) = self.game.as_mut() {
                    game.session.set_dx(
                        self.dx_bundle.clone().filter(|_| self.settings.dx_mode),
                        self.settings.character,
                    );
                    game.session.expanded_levels = self.settings.expanded_levels;
                    dx::prepare_tick(&mut game.session, &self.settings, self.dx_bundle.is_some());
                }
                let (mut first, mut second) = if running {
                    let (one, two) = if self.debug_wasd {
                        self.input
                            .sample_two_players(&self.settings.bindings, &self.second_bindings)
                    } else {
                        let p1 = self.players.player_one();
                        let p2 = self.players.player_two();
                        let one = sample_device(
                            &mut self.input,
                            &mut self.pads,
                            p1,
                            &self.settings.bindings,
                        );
                        let two = sample_device(
                            &mut self.input,
                            &mut self.pads,
                            p2,
                            &self.settings.bindings,
                        );
                        (one, two)
                    };
                    let (one_airborne, two_airborne) =
                        self.game.as_ref().map_or((false, false), |g| {
                            (g.airborne(), g.second_player_airborne())
                        });
                    let mut first = one.keys;
                    let mut second = two.keys;
                    // Spin is Down held in the air ($126F); on the ground it
                    // would duck or pan, so the button only reaches the
                    // joypad once that player has left the floor.
                    if one.spin && one_airborne {
                        first.down = true;
                        first.up = false;
                    }
                    if two.spin && two_airborne {
                        second.down = true;
                        second.up = false;
                    }
                    (first, second)
                } else {
                    (PlayerKeys::default(), PlayerKeys::default())
                };
                if on_title {
                    // The launcher owns title input. Held gameplay bindings
                    // must neither bypass Play nor cancel its queued start.
                    first = PlayerKeys::default();
                    second = PlayerKeys::default();
                }
                let merged = pointer::merge_pointer(&snap, &first, self.aim, self.start_click);
                let first = merged.keys;
                clear_aim = merged.clear_aim;
                clear_start = merged.clear_start;
                if let Some(game) = self.game.as_mut() {
                    game.tick_two_players(first, second, volume);
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
        // A pad that vanished mid-level keeps Player 2's slot until the map.
        let on_map = self
            .game
            .as_ref()
            .is_some_and(|g| matches!(g.session.state, SessionState::Overworld(_)));
        if on_map
            && self.players.player_two_waiting()
            && self.players.remove_absent_player_two().is_some()
        {
            if let Some(game) = &mut self.game {
                game.session.set_local_multiplayer(false);
            }
            self.show_banner("Player 2 left");
        }
        // A screenshot is checked against this CPU frame, which must be a tick.
        let smooth = if self.settings.motion_smoothing
            && running
            && !stepped
            && self.options.screenshot.is_none()
        {
            Smooth {
                alpha: self.budget.fraction(),
                scale: pick_scale(view, self.effective_scale),
            }
        } else {
            Smooth::OFF
        };
        if self.screen == Screen::Game {
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
        } else {
            ctx.request_repaint_after(Duration::from_millis(8));
        }
    }

    /// The current game frame: DX Mode's colors on screens the installed DX
    /// data covers, the display filter everywhere else and on any DX failure.
    fn frame_image(&mut self, smooth: Smooth) -> Option<ColorImage> {
        self.frame_image_with_view(self.view(), smooth)
    }

    fn view(&self) -> (usize, usize) {
        self.viewport_size.map_or_else(
            || self.settings.view(),
            |size| self.settings.view_for_window(size),
        )
    }

    fn frame_image_with_view(
        &mut self,
        view: (usize, usize),
        smooth: Smooth,
    ) -> Option<ColorImage> {
        // Applied at draw time so a change from the pause menu shows at once.
        self.game.as_mut()?.session.hud_style = self.settings.hud_style;
        let game = self.game.as_ref()?;
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
                    ));
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
                    self.menu_owner = None;
                    self.navigate(ctx, Screen::Home);
                }
                if self.screen != Screen::Settings {
                    for page in Page::ALL {
                        if ui.selectable_label(false, page.label()).clicked() {
                            self.return_screen = self.screen;
                            self.page = page;
                            self.navigate(ctx, Screen::Settings);
                        }
                    }
                }
                self.help_menu(ui);
            });
        });
    }

    /// Pause and settings popups share the actual game's paused frame.
    fn pause_menu(&mut self, ctx: &Context) {
        let settings = self.screen == Screen::Settings;
        let title = if settings { "Settings" } else { "Menu" };
        let window = egui::Window::new(title)
            .id(egui::Id::new(if settings {
                "settings-popup"
            } else {
                "pause-menu"
            }))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(if settings { theme::ROW_SIZE.x } else { 260.0 })
            .collapsible(false)
            .resizable(false)
            .movable(false);
        let window = if settings {
            // Leave room for the title bar and margins even at 480×400.
            let available = ctx.available_rect();
            window.constrain_to(available).fixed_size(egui::vec2(
                theme::ROW_SIZE.x,
                (available.height() - 72.0).clamp(160.0, 600.0),
            ))
        } else {
            window
        };
        window.show(ctx, |ui| {
            if settings {
                self.settings_panel(ctx, ui);
            } else {
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 100.0).max(160.0))
                    .show(ui, |ui| self.home_panel(ctx, ui));
                ui.add_space(6.0);
                theme::note(ui, "Esc / Start: resume");
            }
        });
    }

    fn game_view(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_rect_before_wrap();
        let ppp = ui.ctx().pixels_per_point();
        let view = self.view();
        let (offset, size) = game_rectangle(
            [
                (available.width() * ppp).floor() as u32,
                (available.height() * ppp).floor() as u32,
            ],
            self.settings.zoom,
            view,
            self.settings.integer_scaling,
        );
        self.effective_scale = size[0] / view.0 as u32;
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
        let on_title = self.game.as_ref().is_some_and(|g| {
            matches!(
                g.session.state,
                sml2_runtime::session::SessionState::Title(_)
            )
        });
        if on_title {
            let scale = rect.height() / view.1 as f32;
            let native_y = sml2_runtime::render::view::margin(view).1 as f32;
            let grass_top = rect.top() + (native_y + 144.0) * scale;
            let grass =
                egui::Rect::from_min_max(egui::pos2(available.left(), grass_top), available.max);
            self.launcher_bar(ui.ctx(), grass);
        }
        let picture = ui.interact(
            rect,
            egui::Id::new("game-picture"),
            if on_title {
                egui::Sense::hover()
            } else {
                egui::Sense::click()
            },
        );
        if self.screen == Screen::Game && self.dx_picker.is_none() && picture.clicked() {
            if let Some(pos) = picture.interact_pointer_pos() {
                self.on_picture_click(pos, rect);
            }
        }
        if !self.binding_conflicts.is_empty() {
            ui.painter().text(
                available.min + egui::vec2(12.0, 12.0),
                egui::Align2::LEFT_TOP,
                "PLAYER 1 BINDING CONFLICT · Settings > Controls",
                egui::FontId::proportional(15.0),
                Color32::LIGHT_RED,
            );
        }
        if self.paused {
            ui.painter().text(
                available.min + egui::vec2(12.0, 32.0),
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
        let view = self.view();
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
        let corner = rect.right_bottom() - egui::vec2(8.0, 8.0);
        let banner_active = self
            .banner
            .as_ref()
            .is_some_and(|(_, until)| Instant::now() < *until);
        if banner_active {
            if let Some((text, _)) = &self.banner {
                painter.text(
                    corner,
                    egui::Align2::RIGHT_BOTTOM,
                    text,
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );
            }
        } else if let Some(prompt) = self
            .players
            .join_prompt(self.join_spot().is_some() && !self.debug_wasd)
        {
            painter.text(
                corner,
                egui::Align2::RIGHT_BOTTOM,
                prompt,
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
        }
        if self.players.player_one_needs_reconnect() {
            painter.text(
                rect.center_bottom() - egui::vec2(0.0, 8.0),
                egui::Align2::CENTER_BOTTOM,
                "Reconnect Player 1's controller",
                egui::FontId::proportional(13.0),
                Color32::LIGHT_RED,
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

fn button_belongs_to(button: &Button, device: Device) -> bool {
    match device {
        Device::Keyboard => matches!(button, Button::Key(_)),
        Device::Pad(_) => matches!(button, Button::Pad(_)),
    }
}

/// Whether a press of `button` on `device` is pending, without consuming it.
fn pending_button(
    merged: &InputState,
    pads: &BTreeMap<u64, InputState>,
    device: Device,
    button: &Button,
) -> bool {
    if !button_belongs_to(button, device) {
        return false;
    }
    match device {
        Device::Keyboard => merged.press_pending(button),
        Device::Pad(id) => pads
            .get(&id)
            .is_some_and(|state| state.press_pending(button)),
    }
}

/// Consume a press of `button` on `device`. Gamepad events also leave the
/// merged host stream so they cannot confirm a desktop menu.
fn take_button(
    merged: &mut InputState,
    pads: &mut BTreeMap<u64, InputState>,
    device: Device,
    button: &Button,
) -> bool {
    if !button_belongs_to(button, device) {
        return false;
    }
    match device {
        Device::Keyboard => merged.take_press(button),
        Device::Pad(id) => {
            let own = pads
                .get_mut(&id)
                .is_some_and(|state| state.take_press(button));
            merged.take_press(button);
            own
        }
    }
}

/// Whether a device has any pending press. Used on a title-less file select,
/// where the first device to move or confirm is the one opening the map.
fn pending_action(merged: &InputState, pads: &BTreeMap<u64, InputState>, device: Device) -> bool {
    match device {
        Device::Keyboard => merged.any_press(ButtonClass::Keyboard),
        Device::Pad(id) => pads
            .get(&id)
            .is_some_and(|state| state.any_press(ButtonClass::Pad)),
    }
}

/// A title-screen action press claims Player 1: Start on a pad, Enter or
/// Space on the keyboard. The menu's confirm buttons are included so the
/// device that opens a map (or the title's Play action) becomes Player 1
/// instead of an arbitrary pad.
fn pending_claim(merged: &InputState, pads: &BTreeMap<u64, InputState>, device: Device) -> bool {
    let buttons = match device {
        Device::Keyboard => [Button::key("Enter"), Button::key("Space")],
        Device::Pad(_) => [Button::Pad(PadButton::Start), Button::Pad(PadButton::South)],
    };
    buttons
        .iter()
        .any(|button| pending_button(merged, pads, device, button))
}

/// Start/Enter on a device that is not Player 1's joins Player 2.
fn take_join_press(
    merged: &mut InputState,
    pads: &mut BTreeMap<u64, InputState>,
    device: Device,
) -> bool {
    let button = match device {
        Device::Keyboard => Button::key("Enter"),
        Device::Pad(_) => Button::Pad(PadButton::Start),
    };
    take_button(merged, pads, device, &button)
}

fn take_device_action(
    merged: &mut InputState,
    pads: &mut BTreeMap<u64, InputState>,
    device: Device,
    bindings: &Bindings,
    action: Action,
) -> bool {
    let mut found = false;
    for button in &bindings.0[action as usize] {
        found |= take_button(merged, pads, device, button);
    }
    found
}

/// Player 2 leaves with Select/− or Backspace even when the Select action
/// was rebound to another button.
fn take_leave_press(
    merged: &mut InputState,
    pads: &mut BTreeMap<u64, InputState>,
    device: Device,
    bindings: &Bindings,
) -> bool {
    let mut found = take_device_action(merged, pads, device, bindings, Action::Select);
    found |= take_button(merged, pads, device, &Button::key("Backspace"));
    found |= take_button(merged, pads, device, &Button::Pad(PadButton::Select));
    found
}

/// Start (gamepad) or Enter (keyboard) also reaches a rebound Start action.
fn take_pause_press(
    merged: &mut InputState,
    pads: &mut BTreeMap<u64, InputState>,
    device: Device,
    bindings: &Bindings,
) -> bool {
    let mut found = take_device_action(merged, pads, device, bindings, Action::Start);
    found |= take_join_press(merged, pads, device);
    found
}

fn sample_device(
    merged: &mut InputState,
    pads: &mut BTreeMap<u64, InputState>,
    device: Option<Device>,
    bindings: &Bindings,
) -> Controls {
    match device {
        Some(Device::Keyboard) => merged.sample_keyboard(bindings),
        Some(Device::Pad(id)) => pads
            .get_mut(&id)
            .map_or_else(Controls::default, |state| state.sample_pad(bindings)),
        None => Controls::default(),
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

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        self.ui(ctx);
    }
    fn on_exit(&mut self) {
        self.persist();
        self.updater.apply_on_exit();
    }
}
