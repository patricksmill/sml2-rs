use super::theme::{self, Value};
use super::{DesktopApp, Screen};
use crate::library::Row;
use crate::{
    input::{Action, Bindings, Button, PadButton},
    menu::{Preview, pressed, release_focus},
    settings::{HudStyle, Zoom},
};
use eframe::egui::{self, Context, Ui, ViewportCommand};
use sml2_runtime::session::SessionState;

/// Settings pages, in tab order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(super) enum Page {
    Display,
    Audio,
    Controls,
    Mods,
    Game,
    Debug,
}

impl Page {
    pub(super) const ALL: [Page; 6] = [
        Page::Display,
        Page::Audio,
        Page::Controls,
        Page::Mods,
        Page::Game,
        Page::Debug,
    ];
    pub(super) fn label(self) -> &'static str {
        match self {
            Page::Display => "DISPLAY",
            Page::Audio => "AUDIO",
            Page::Controls => "CONTROLS",
            Page::Mods => "MODS",
            Page::Game => "GAME",
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

    /// Cold-start menu. Installation controls live in Settings → Game.
    pub(super) fn library_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        if self.menu.compact_button(ui, "Play") {
            if self.library.play_enabled() {
                self.play_library();
            } else {
                self.return_screen = Screen::Home;
                self.page = Page::Game;
                self.navigate(ctx, Screen::Settings);
            }
        }
        if self.menu.compact_button(ui, "Settings") {
            self.return_screen = Screen::Home;
            self.navigate(ctx, Screen::Settings);
        }
        if self.menu.compact_button(ui, "Quit") && self.library.row_enabled(Row::Exit) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if let Some(message) = &self.status {
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
        let previous_page = self.page;
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
        ui.set_width(ui.available_width());
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

        // Reserve the footer before allocating the page, including its two gaps.
        // Short pages fill the same space; long pages scroll below the fixed tabs.
        let footer_height = theme::ROW_SIZE.y
            + ui.text_style_height(&egui::TextStyle::Body)
            + 2.0 * ui.spacing().item_spacing.y;
        let body = egui::vec2(
            ui.available_width(),
            (ui.available_height() - footer_height).max(0.0),
        );
        // Narrow windows stack the explanation under the list instead of beside it.
        let two_pane = body.x >= 600.0;
        let list_width = if two_pane {
            (body.x * 0.55).floor()
        } else {
            body.x
        };
        ui.allocate_ui_with_layout(body, egui::Layout::left_to_right(egui::Align::Min), |ui| {
            let mut page = egui::ScrollArea::vertical()
                .id_salt(("settings-page", self.page))
                .auto_shrink([false, false])
                .min_scrolled_height(0.0)
                .max_width(list_width)
                .max_height(body.y);
            if self.page != previous_page {
                page = page.vertical_scroll_offset(0.0);
            }
            page.show(ui, |ui| {
                // The scroll area inherits the row layout; pages stack top-down.
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(ui.available_width());
                    match self.page {
                        Page::Display => self.display_page(ctx, ui),
                        Page::Audio => self.audio_page(ui),
                        Page::Controls => self.controls_page(ui),
                        Page::Mods => {
                            self.view_controls(ui);
                            self.dx_controls(ui);
                        }
                        Page::Game => self.installation_page(ui),
                        Page::Debug => self.debug_page(ui),
                    }
                    if let Some(message) = &self.status {
                        theme::error(ui, message);
                    }
                    if !two_pane {
                        self.detail_pane(ui);
                    }
                });
            });
            if two_pane {
                ui.separator();
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(ui.available_width());
                        self.detail_pane(ui);
                    },
                );
            }
        });

        if self.menu.row(ui, "Back", Value::None) && self.capture.is_none() {
            self.persist();
            self.navigate(ctx, self.return_screen);
        }
        theme::note(ui, "Esc: back · PgUp/PgDn or LB/RB: page");
    }

    /// Explains the hovered or selected setting, with a live sample when it applies.
    fn detail_pane(&mut self, ui: &mut Ui) {
        let Some(detail) = self.menu.detail().cloned() else {
            theme::note(ui, "Select a setting to see what it does.");
            return;
        };
        ui.add_space(6.0);
        ui.strong(&detail.title);
        ui.add_space(4.0);
        ui.label(&detail.text);
        let view = self.view();
        let caption = match detail.preview {
            _ if self.settings.dx_mode && self.dx_bundle.is_some() => {
                "DX Mode colors the levels; the sample shows the filter"
            }
            Some(Preview::Pixels) => "Confirm on this row to compare",
            _ => "Left / right on this row to compare",
        };
        if let (Some(kind), Some(preview)) = (detail.preview, &mut self.settings_preview) {
            ui.add_space(8.0);
            preview.show(ui, kind, &self.settings, view, caption);
        }
    }

    fn display_page(&mut self, ctx: &Context, ui: &mut Ui) {
        theme::section(ui, "SCREEN");
        let fit = self.settings.zoom == Zoom::Auto;
        if self.menu.row(ui, "Fit to window", Value::Toggle(fit)) {
            self.fit_to_window(!fit);
        }
        self.menu.describe(
            "Scales the game to fill the window as you resize it. Turn it off to pick a fixed \
             window scale instead. + / − change the zoom at any time.",
        );
        if let Zoom::Integer(scale) = self.settings.zoom {
            let adjust = self.menu.choice(ui, "Window scale", format!("{scale}×"));
            if adjust != 0 {
                let next = (i32::from(scale) - 1 + adjust).rem_euclid(8) + 1;
                self.zoom(ctx, Some(next as u8));
            }
            self.menu.describe(
                "How many screen pixels each game pixel uses. The window resizes to match.",
            );
        }
        if self.menu.row(
            ui,
            "Integer scaling",
            Value::Toggle(self.settings.integer_scaling),
        ) {
            self.settings.integer_scaling = !self.settings.integer_scaling;
        }
        self.menu.describe_with_preview(
            "Only scales the picture by whole numbers (2×, 3×, …) so every game pixel is the \
             same size and sharp. Off fills more of the window, but some pixels end up a \
             little wider or taller than others, which can look uneven when scrolling.",
            Preview::Pixels,
        );
        if self
            .menu
            .row(ui, "Fullscreen", Value::Toggle(self.settings.fullscreen))
        {
            self.fullscreen(ctx);
        }
        self.menu
            .describe("Fills the whole screen without window borders. F11 toggles it in game.");
        theme::section(ui, "HUD");
        let modern = self.settings.hud_style == HudStyle::Modern;
        let label = if modern { "Modern" } else { "Original" };
        if self.menu.choice(ui, "Status bar", label.into()) != 0 {
            self.settings.hud_style = if modern {
                HudStyle::Classic
            } else {
                HudStyle::Modern
            };
        }
        self.menu.describe_with_preview(
            "Original draws the status bar as a strip along the bottom, like the Game Boy. \
             Modern floats the same counters in the corners of the view: lives and coins top \
             left, time top right. Gameplay is identical.",
            Preview::Frame,
        );
        theme::section(ui, "COLOR");
        let adjust = self
            .menu
            .choice(ui, "Palette filter", self.settings.filter.label());
        if adjust != 0 {
            self.cycle_filter(adjust);
        }
        self.menu.describe_with_preview(
            "Colors the four Game Boy shades, from the original screen tints to Super Game \
             Boy palettes. F4 cycles filters in game.",
            Preview::Frame,
        );
    }

    /// Settings → Mods: the enlarged view.
    fn view_controls(&mut self, ui: &mut Ui) {
        theme::section(ui, "VIEW");
        let widescreen = self.settings.widescreen;
        let adjust = self
            .menu
            .choice(ui, "Widescreen", widescreen.label().into());
        if adjust != 0 {
            self.settings.widescreen = widescreen.cycle(adjust);
        }
        self.menu.describe_with_preview(
            "Shows more of each level to the sides: 16:9, 21:9, or whatever shape the window \
             is. A wider view spawns enemies and items earlier, so play differs from the \
             original. Menus and cutscenes keep their original size.",
            Preview::Frame,
        );
        let zoom = self.settings.camera_zoom;
        let adjust = self.menu.choice(ui, "Camera zoom", zoom.label().into());
        if adjust != 0 {
            self.settings.camera_zoom = zoom.cycle(adjust);
        }
        self.menu.describe_with_preview(
            "Pulls the camera back to show more of the level above and below as well. Like \
             widescreen, a larger view spawns enemies and items earlier.",
            Preview::Frame,
        );
        if self.menu.row(
            ui,
            "Motion smoothing",
            Value::Toggle(self.settings.motion_smoothing),
        ) {
            self.settings.motion_smoothing = !self.settings.motion_smoothing;
        }
        self.menu.describe(
            "Draws levels between game frames on displays faster than 60 Hz for smoother \
             scrolling. The picture runs one frame behind.",
        );
        theme::section(ui, "LEVELS");
        if self.menu.row(
            ui,
            "Expanded levels",
            Value::Toggle(self.settings.expanded_levels),
        ) {
            self.settings.expanded_levels = !self.settings.expanded_levels;
        }
        self.menu.describe(
            "Plays levels whose sections the original joined with pipe jumps as one \
             connected map. Off restores the original layouts. Takes effect when a level \
             starts.",
        );
    }

    fn audio_page(&mut self, ui: &mut Ui) {
        theme::section(ui, "SOUND");
        let levels = [
            ("Volume", &mut self.settings.volume),
            ("Music", &mut self.settings.music_volume),
            ("Sound effects", &mut self.settings.effects_volume),
        ];
        let hints = [
            "Overall loudness. Music and sound effects scale within it.",
            "Background music, relative to the overall volume.",
            "Jumps, coins, enemies and other sounds, relative to the overall volume.",
        ];
        for ((label, level), hint) in levels.into_iter().zip(hints) {
            let adjust = self.menu.row_adjustment();
            if self.menu.row(
                ui,
                label,
                Value::Meter(*level, format!("{:.0}%", *level * 100.0)),
            ) || adjust != 0
            {
                *level = ((*level * 10.0).round() + if adjust != 0 { adjust as f32 } else { 1.0 })
                    .clamp(0.0, 10.0)
                    / 10.0;
            }
            self.menu
                .describe(&format!("{hint} Left / right adjust it in 10% steps."));
        }
        if self
            .menu
            .row(ui, "Mute", Value::Toggle(self.settings.muted))
        {
            self.settings.muted = !self.settings.muted;
        }
        self.menu
            .describe("Silences all sound without changing the levels above.");
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
        self.menu.describe(
            "The controller the game listens to. Confirm to switch between connected \
             controllers. The keyboard always works.",
        );
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
        self.menu.describe(
            "How far the analog stick must move before it counts as a direction. Raise it if \
             Mario drifts on his own; lower it if the stick feels unresponsive.",
        );
        if let Some(error) = &self.controllers.error {
            theme::error(ui, error);
        }
        theme::section(ui, "BUTTONS");
        for action in Action::ALL {
            let value = if self.capture == Some(action) {
                "press a button…".to_string()
            } else {
                let row = &self.settings.bindings.0[action as usize];
                if row.is_empty() {
                    "unbound".to_string()
                } else {
                    row.iter()
                        .map(|b| b.label())
                        .collect::<Vec<_>>()
                        .join(" / ")
                }
            };
            if self.menu.row(ui, action.label(), Value::Text(value)) && self.capture.is_none() {
                self.capture = Some(action);
                self.input.suppress_until_release();
            }
            self.menu.describe(
                "Confirm, then press a key or controller button; Esc cancels. Rebinding keeps \
                 the other device's controls.",
            );
        }
        if self.menu.row(ui, "Reset controls", Value::None) && self.capture.is_none() {
            self.settings.bindings = Bindings::default();
            self.input.suppress_until_release();
        }
        self.menu.describe(
            "Restores the default keyboard and controller bindings. Controller Start always \
             opens the menu.",
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
        self.menu.describe(
            "A window that warps between zones and levels and edits progress, coins and \
             power-ups.",
        );
        if self
            .menu
            .row(ui, "Pause game · F2", Value::Toggle(self.paused))
        {
            self.paused = !self.paused;
        }
        self.menu
            .describe("Freezes the game. F3 steps one tick at a time while paused.");
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
        self.menu.describe(&format!(
            "Restores every setting to its default. Confirm twice. Settings are stored in {}.",
            self.options.paths.settings.display()
        ));
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
