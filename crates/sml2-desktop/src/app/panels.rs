use super::catalog;
use super::theme::{self, Value};
use super::{DesktopApp, LibraryPick, Screen};
use crate::library::Row;
use crate::{
    input::{Action, Bindings, Button, PadButton},
    menu::{pressed, release_focus},
    settings::Zoom,
};
use eframe::egui::{self, Context, Ui, ViewportCommand};
use sml2_runtime::session::SessionState;

/// Settings pages, in tab order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Page {
    Display,
    Audio,
    Controls,
    Mods,
    Debug,
}

impl Page {
    pub(super) const ALL: [Page; 5] = [
        Page::Display,
        Page::Audio,
        Page::Controls,
        Page::Mods,
        Page::Debug,
    ];
    fn label(self) -> &'static str {
        match self {
            Page::Display => "DISPLAY",
            Page::Audio => "AUDIO",
            Page::Controls => "CONTROLS",
            Page::Mods => "MODS",
            Page::Debug => "DEBUG",
        }
    }
}

fn on_front_end(state: &SessionState) -> bool {
    matches!(
        state,
        SessionState::Title(_) | SessionState::FileSelect(_) | SessionState::FileSelectLoading(_)
    )
}

impl DesktopApp {
    pub(super) fn home_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.set_width(260.0);
        ui.spacing_mut().item_spacing.y = 4.0;
        if self.game.is_some() && self.menu.button(ui, "Resume") {
            self.navigate(ctx, Screen::Game);
        }
        if self.game.is_none() {
            if self.menu.button(ui, "Retry loading game") {
                self.load_game();
                if self.game.is_some() {
                    self.navigate(ctx, Screen::Game);
                }
            }
        }
        if self.menu.button(ui, "Settings") {
            self.return_screen = Screen::Home;
            self.navigate(ctx, Screen::Settings);
        }
        if self.game.is_some() && self.menu.button(ui, "Scenery editor") {
            self.open_editor(ctx);
        }
        let quit_to_title = self
            .game
            .as_ref()
            .is_some_and(|game| !on_front_end(&game.session.state));
        if quit_to_title && self.menu.button(ui, "Quit to title") {
            if let Some(game) = &mut self.game {
                game.session.enter_title();
            }
            self.navigate(ctx, Screen::Game);
        }
        if self.menu.button(ui, "Quit") {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if let Some(message) = &self.status {
            ui.add_space(6.0);
            theme::error(ui, message);
        }
        if !self.allow_save && self.menu.button(ui, "Reset unreadable settings") {
            self.reset_settings();
        }
    }

    /// Cold-start library. A loaded game keeps [`Self::home_panel`].
    pub(super) fn library_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.set_width(theme::ROW_SIZE.x);
        ui.spacing_mut().item_spacing.y = 4.0;
        theme::section(ui, "1. ORIGINAL GAME");
        theme::note(ui, "Add the original Super Mario Land 2 ROM (UE v1.0).");
        theme::note(
            ui,
            ".gb or .gbc is fine; the file must contain the original game.",
        );
        let rom_label = if self.settings.vanilla.is_some() || self.library.vanilla_ready {
            "Replace ROM…"
        } else {
            "Import ROM…"
        };
        if self.menu.row(
            ui,
            rom_label,
            Value::Text(self.library.row_status(Row::Rom)),
        ) && self.library.row_enabled(Row::Rom)
            && !self.library.checking
        {
            self.start_library_import(LibraryPick::Rom);
        }
        if self.library.vanilla_ready && !self.library.checking {
            ui.add_space(10.0);
            theme::section(ui, "2. DX MODE · OPTIONAL");
            theme::note(ui, "Add SML2DX v1.8.1 as a patched ROM or an IPS patch.");
            theme::note(ui, "You can skip this and play the original game.");
            let dx_status = self.library.row_status(Row::Dx);
            let show_switch =
                self.library.vanilla_ready && self.library.dx_installed && !self.library.checking;
            if show_switch
                && self
                    .menu
                    .row(ui, "DX Mode", Value::Toggle(self.settings.dx_mode))
                && self.library.row_enabled(Row::Dx)
            {
                self.settings.dx_mode = !self.settings.dx_mode;
                self.library.dx_mode = self.settings.dx_mode;
            }
            let dx_label = if self.library.dx_installed {
                "Replace DX ROM / IPS…"
            } else {
                "Add DX ROM / IPS…"
            };
            let dx_value = if show_switch {
                Value::None
            } else {
                Value::Text(dx_status)
            };
            if self.menu.row(ui, dx_label, dx_value) && self.library.row_enabled(Row::Dx) {
                self.start_library_import(LibraryPick::Dx);
            }
            if self.menu.button(ui, self.library.play_label())
                && self.library.row_enabled(Row::Play)
            {
                self.play_library();
            }
        }
        if self.menu.button(ui, "Exit") && self.library.row_enabled(Row::Exit) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if self.library.importing
            && self.library_dx_import
            && self.dx_worker.is_some()
            && self.menu.row(ui, "Cancel", Value::None)
        {
            if let Some(worker) = &self.dx_worker {
                worker.cancel();
            }
            self.dx_message = "Cancelling DX import…".into();
        }
        if !self.dx_message.is_empty() {
            ui.add_space(6.0);
            theme::note(ui, &self.dx_message);
        }
        if !self.library.dx_error.is_empty() {
            ui.add_space(6.0);
            theme::note(ui, &self.library.dx_error);
        }
        if let Some(message) = &self.status {
            ui.add_space(6.0);
            theme::error(ui, message);
        }
    }

    /// Replace the settings file with defaults, even when it could not be read.
    pub(super) fn reset_settings(&mut self) {
        let window_size = self.settings.window_size;
        let dx_installation = self.settings.dx_installation.clone();
        self.settings = Default::default();
        self.settings.window_size = window_size;
        self.settings.dx_installation = dx_installation;
        self.allow_save = true;
        // Force replacement even when the current values already match defaults.
        self.saved_settings.version = 0;
        self.persist();
        self.status = None;
    }

    pub(super) fn settings_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        if self.capture.is_none() && self.dx_picker.is_none() {
            let back = pressed(
                &mut self.input,
                &[Button::Pad(PadButton::LeftShoulder), Button::key("PageUp")],
            );
            let next = pressed(
                &mut self.input,
                &[
                    Button::Pad(PadButton::RightShoulder),
                    Button::key("PageDown"),
                ],
            );
            if back || next {
                let count = Page::ALL.len() as i32;
                let index = Page::ALL.iter().position(|p| *p == self.page).unwrap_or(0) as i32;
                self.page = Page::ALL
                    [(index + i32::from(next) - i32::from(back)).rem_euclid(count) as usize];
                self.menu.selected = 0;
                self.reset_armed = false;
            }
        }
        ui.set_width(theme::ROW_SIZE.x);
        ui.spacing_mut().item_spacing.y = 6.0;
        let labels = Page::ALL.map(Page::label);
        let mut index = Page::ALL.iter().position(|p| *p == self.page).unwrap_or(0);
        ui.vertical_centered(|ui| {
            if self.menu.tabs(ui, &labels, &mut index) {
                self.page = Page::ALL[index];
                self.reset_armed = false;
            }
        });
        ui.add_space(4.0);
        match self.page {
            Page::Display => self.display_page(ctx, ui),
            Page::Audio => self.audio_page(ui),
            Page::Controls => self.controls_page(ui),
            Page::Mods => {
                self.view_controls(ui);
                self.dx_controls(ui);
            }
            Page::Debug => self.debug_page(ui),
        }
        ui.add_space(6.0);
        if self.menu.row(ui, "Back", Value::None) && self.capture.is_none() {
            self.persist();
            self.navigate(ctx, self.return_screen);
        }
        if let Some(message) = &self.status {
            theme::error(ui, message);
        }
    }

    fn display_page(&mut self, ctx: &Context, ui: &mut Ui) {
        theme::section(ui, "SCREEN");
        let fit = self.settings.zoom == Zoom::Auto;
        if self.menu.row(ui, "Fit to window", Value::Toggle(fit)) {
            self.fit_to_window(!fit);
        }
        if let Zoom::Integer(scale) = self.settings.zoom {
            let adjust = self.menu.choice(ui, "Window scale", format!("{scale}×"));
            if adjust != 0 {
                let next = (i32::from(scale) - 1 + adjust).rem_euclid(8) + 1;
                self.zoom(ctx, Some(next as u8));
            }
        }
        theme::note(
            ui,
            "Fit scales the same game view as you resize the window.",
        );
        theme::note(
            ui,
            "Whole-number scaling keeps pixels crisp; borders preserve the aspect ratio.",
        );
        if self
            .menu
            .row(ui, "Fullscreen", Value::Toggle(self.settings.fullscreen))
        {
            self.fullscreen(ctx);
        }
        theme::section(ui, "COLOR");
        let adjust = self
            .menu
            .choice(ui, "Palette filter", self.settings.filter.label());
        if adjust != 0 {
            self.cycle_filter(adjust);
        }
        if let Some(preview) = &mut self.filter_preview {
            ui.add_space(4.0);
            let caption = if self.settings.dx_mode && self.dx_bundle.is_some() {
                "DX Mode colors the levels; the filter covers everything else"
            } else {
                "Left / right on the filter row to compare"
            };
            preview.show(ui, self.settings.filter, caption);
        }
        theme::note(ui, "+ / − zoom · F11 fullscreen · F4 cycles filters.");
    }

    /// Settings → Mods: the enlarged view.
    fn view_controls(&mut self, ui: &mut Ui) {
        theme::section(ui, "VIEW");
        let widescreen = self.settings.widescreen;
        if self
            .menu
            .choice(ui, "Widescreen", widescreen.label().into())
            != 0
        {
            self.settings.widescreen = widescreen.next();
        }
        let zoom = self.settings.camera_zoom;
        let adjust = self.menu.choice(ui, "Camera zoom", zoom.label().into());
        if adjust != 0 {
            self.settings.camera_zoom = zoom.cycle(adjust);
        }
        if self.menu.row(
            ui,
            "Motion smoothing",
            Value::Toggle(self.settings.motion_smoothing),
        ) {
            self.settings.motion_smoothing = !self.settings.motion_smoothing;
        }
        theme::note(
            ui,
            "A larger view also spawns enemies and items earlier, so play differs from the \
             original while it is on. Menus and cutscenes keep their original size. Motion \
             smoothing draws levels between frames on displays faster than 60 Hz, one frame \
             behind.",
        );
    }

    fn audio_page(&mut self, ui: &mut Ui) {
        theme::section(ui, "SOUND");
        let adjust = self.menu.row_adjustment();
        if self.menu.row(
            ui,
            "Volume",
            Value::Meter(
                self.settings.volume,
                format!("{:.0}%", self.settings.volume * 100.0),
            ),
        ) || adjust != 0
        {
            self.settings.volume = ((self.settings.volume * 10.0).round()
                + if adjust != 0 { adjust as f32 } else { 1.0 })
            .clamp(0.0, 10.0)
                / 10.0;
        }
        if self
            .menu
            .row(ui, "Mute", Value::Toggle(self.settings.muted))
        {
            self.settings.muted = !self.settings.muted;
        }
        theme::note(ui, "Left / right adjust the volume in 10% steps.");
    }

    fn controls_page(&mut self, ui: &mut Ui) {
        theme::section(ui, "CONTROLLER");
        if self.menu.row(
            ui,
            "Device",
            Value::Text(self.controllers.name().to_string()),
        ) {
            self.settings.preferred_controller = self.controllers.cycle(&mut self.input);
            self.input.suppress_until_release();
        }
        let adjust = self.menu.row_adjustment();
        if self.menu.row(
            ui,
            "Stick dead zone",
            Value::Meter(
                (self.settings.dead_zone - 0.1) / 0.7,
                format!("{:.0}%", self.settings.dead_zone * 100.0),
            ),
        ) || adjust != 0
        {
            self.settings.dead_zone = ((self.settings.dead_zone * 100.0).round()
                + if adjust != 0 {
                    adjust as f32 * 5.0
                } else {
                    5.0
                })
            .clamp(10.0, 80.0)
                / 100.0;
        }
        if let Some(error) = &self.controllers.error {
            theme::error(ui, error);
        }
        theme::section(ui, "BUTTONS");
        for action in Action::ALL {
            let value = if self.capture == Some(action) {
                "press a button…".to_string()
            } else {
                self.settings.bindings.0[action as usize]
                    .iter()
                    .map(|b| b.label())
                    .collect::<Vec<_>>()
                    .join(" / ")
            };
            if self.menu.row(ui, action.label(), Value::Text(value)) && self.capture.is_none() {
                self.capture = Some(action);
                self.input.suppress_until_release();
            }
        }
        if self.menu.row(ui, "Reset controls", Value::None) && self.capture.is_none() {
            self.settings.bindings = Bindings::default();
            self.input.suppress_until_release();
        }
        theme::note(
            ui,
            "Choose an action, then press a key or controller button; Esc cancels. Rebinding keeps the other device's controls. Controller Start opens the menu.",
        );
    }

    fn debug_page(&mut self, ui: &mut Ui) {
        theme::section(ui, "TOOLS");
        if self
            .menu
            .row(ui, "Debug panel · F1", Value::Toggle(self.debug))
        {
            self.debug = !self.debug;
        }
        if self
            .menu
            .row(ui, "Pause game · F2", Value::Toggle(self.paused))
        {
            self.paused = !self.paused;
        }
        theme::note(
            ui,
            "F3 steps one tick while paused. The panel warps between zones and levels, edits progress, coins and power-ups.",
        );
        theme::section(ui, "SETTINGS FILE");
        let label = if self.reset_armed {
            "Press again to reset everything"
        } else {
            "Reset all settings"
        };
        if self.menu.row(ui, label, Value::None) {
            if self.reset_armed {
                self.reset_settings();
                self.reset_armed = false;
            } else {
                self.reset_armed = true;
            }
        }
        theme::note(ui, &format!("{}", self.options.paths.settings.display()));
    }

    pub(super) fn settings_window(&mut self, ctx: &Context) {
        if !self.settings_open {
            return;
        }
        let mut open = self.settings_open;
        let mut close_for_full_settings = false;
        egui::Window::new(egui::RichText::new("Settings").color(theme::INK))
            .open(&mut open)
            .default_width(280.0)
            .collapsible(false)
            .resizable(false)
            .frame(theme::popup_frame())
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(theme::INK);
                let mut fit = self.settings.zoom == Zoom::Auto;
                if ui.checkbox(&mut fit, "Fit to window")
                    .on_hover_text("Scale the same game view to fit the window, preserving crisp pixels and aspect ratio.")
                    .changed()
                {
                    self.fit_to_window(fit);
                    self.persist();
                    release_focus(ctx);
                }
                for pedestal in catalog::PEDESTALS {
                    if pedestal == catalog::Pedestal::Zoom && self.settings.zoom == Zoom::Auto {
                        continue;
                    }
                    let text = catalog::label(pedestal, &self.settings, self.debug);
                    if ui.button(text).clicked() {
                        match pedestal {
                            catalog::Pedestal::Fullscreen => self.fullscreen(ctx),
                            catalog::Pedestal::Zoom => {
                                let current = match self.settings.zoom {
                                    Zoom::Auto => 0,
                                    Zoom::Integer(n) => n as i32,
                                };
                                let next = (current + 1).rem_euclid(9);
                                self.zoom(
                                    ctx,
                                    if next == 0 { None } else { Some(next as u8) },
                                );
                            }
                            _ => catalog::cycle(pedestal, &mut self.settings, &mut self.debug),
                        }
                        self.persist();
                        release_focus(ctx);
                    }
                }
                ui.separator();
                ui.label("F10 hides this bar. Bindings, ROM import, and reset stay on the full settings page.");
                if ui.button("All settings").clicked() {
                    self.return_screen = self.screen;
                    self.navigate(ctx, Screen::Settings);
                    close_for_full_settings = true;
                    release_focus(ctx);
                }
            });
        self.settings_open = open && !close_for_full_settings;
        if !self.settings_open {
            self.input.suppress_until_release();
            self.budget.reset();
            release_focus(ctx);
        }
    }

    pub(super) fn debug_panel(&mut self, ctx: &Context) {
        let mut open = self.debug;
        let mut action = None;
        egui::Window::new("Debug · F1")
            .open(&mut open)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(if self.paused {
                            "Resume · F2"
                        } else {
                            "Pause · F2"
                        })
                        .clicked()
                    {
                        self.paused = !self.paused;
                        release_focus(ctx);
                    }
                    if ui.button("Step · F3").clicked() {
                        self.paused = true;
                        self.step = true;
                        release_focus(ctx);
                    }
                });
                if let Some(game) = &self.game {
                    let session = &game.session;
                    ui.monospace(format!(
                        "Tick {}  |  zoom {}×",
                        session.frame_n, self.effective_scale
                    ));
                    ui.monospace(format!(
                        "Song {}  |  lives {:02X}  |  slot {:?}",
                        session.sound_driver.active_song, session.lives, session.active_slot
                    ));
                    match &session.state {
                        SessionState::Overworld(map) => {
                            ui.label("Overworld");
                            ui.monospace(format!("Screen {}  node {}", map.screen_id, map.node));
                            ui.monospace(format!(
                                "Camera {}, {}  RNG {:02X}",
                                map.cam_x, map.cam_y, map.rand_latch
                            ));
                            if let Some(transition) = &session.pending_level {
                                ui.label(format!(
                                    "Entering level {}: frame {}",
                                    transition.level_id, transition.frame
                                ));
                            }
                        }
                        SessionState::Demo(demo) => {
                            ui.label(format!("Attract demo {}", demo.index));
                            ui.monospace(format!(
                                "Level {}  tape {}  frame {}",
                                demo.level_id,
                                demo.tape.position(),
                                demo.ctx.frame_n
                            ));
                        }
                        SessionState::InLevel(level) => {
                            ui.label(format!("Level {}", level.manifest.level_id));
                            ui.monospace(format!(
                                "Player {}, {}  velocity {:02X}/{:02X}",
                                level.sim.x, level.sim.y, level.sim.hvel, level.sim.vvel
                            ));
                            ui.monospace(format!(
                                "Camera {}, {}  scene {:02X}",
                                level.cam.x, level.cam.y, level.cam.scene_id
                            ));
                            ui.monospace(format!(
                                "Fade {}  game pause {}",
                                level.sim.fade_timer, level.sim.paused
                            ));
                            egui::CollapsingHeader::new("Object slots").show(ui, |ui| {
                                for (index, slot) in level.obj.slots.iter().enumerate() {
                                    ui.monospace(format!(
                                        "{index:02}: {:?} ({}, {})",
                                        slot.state(),
                                        slot.x(),
                                        slot.y()
                                    ));
                                }
                            });
                            ui.monospace(format!("Easy mode: {}", level.obj.easy_mode));
                            if !level.obj.unported.is_empty() {
                                ui.label(format!("Unported handlers: {:?}", level.obj.unported));
                            }
                        }
                        SessionState::Title(title) => {
                            ui.label("Title");
                            ui.monospace(format!("Timer {:04X}", title.timer()));
                        }
                        SessionState::FileSelectLoading(_) => {
                            ui.label("Loading file select");
                        }
                        SessionState::FileSelect(file_select) => {
                            ui.label("File select");
                            ui.monospace(format!(
                                "State {}  cursor {},{}",
                                file_select.menu_state(),
                                file_select.cursor().0,
                                file_select.cursor().1
                            ));
                            ui.monospace(format!(
                                "Easy {}  erase {}",
                                file_select.easy_mode(),
                                file_select.clear_mode()
                            ));
                        }
                        SessionState::Castle(seq) => {
                            ui.label("Castle reveal");
                            ui.monospace(format!(
                                "State {:02X}  menu {}  fade {:02X}/{:02X}",
                                seq.state, seq.menu, seq.fade_high, seq.fade_low
                            ));
                        }
                        SessionState::GameOver(screen) => {
                            ui.label("Game over");
                            ui.monospace(format!(
                                "Fade {:03}  coins {:06b}",
                                screen.sim.fade_timer, screen.sim.golden_coin_flags
                            ));
                        }
                    }
                    if !game.diagnostics.is_empty() {
                        egui::CollapsingHeader::new("Diagnostics").show(ui, |ui| {
                            for message in &game.diagnostics {
                                ui.label(message);
                            }
                        });
                    }
                }
                ui.separator();
                action = self.debug_controls(ui);
                ui.separator();
                ui.label("Menu and focus loss suspend ticking. Step advances one outer game tick.");
            });
        self.debug = open;
        if let Some(action) = action {
            self.apply_debug(action);
        }
    }
}
