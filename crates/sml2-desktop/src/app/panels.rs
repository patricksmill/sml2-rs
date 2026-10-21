use super::{DesktopApp, Screen};
use crate::{
    input::{Action, Bindings},
    menu::release_focus,
    settings::Zoom,
};
use eframe::egui::{self, Context, RichText, Ui, ViewportCommand};
use sml2_runtime::session::SessionState;

impl DesktopApp {
    pub(super) fn home_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.label(RichText::new("SUPER MARIO LAND 2").size(30.0).strong());
        ui.label(RichText::new("Native Rust edition").color(egui::Color32::LIGHT_GREEN));
        ui.add_space(24.0);
        if self.menu.button(
            ui,
            if self.game.is_some() {
                "Play / Resume"
            } else {
                "Retry loading game"
            },
        ) {
            if self.game.is_none() {
                self.load_game();
            }
            if self.game.is_some() {
                self.navigate(ctx, Screen::Game);
            }
        }
        if self.menu.button(ui, "Title / File Select") {
            if self.game.is_none() {
                self.load_game();
            }
            if let Some(game) = &mut self.game {
                // `$00`: the original boot flow; START opens the three pipes.
                game.session.enter_title();
            }
            if self.game.is_some() {
                self.navigate(ctx, Screen::Game);
            }
        }
        if self.menu.button(ui, "Settings") {
            self.return_screen = Screen::Home;
            self.navigate(ctx, Screen::Settings);
        }
        if self.menu.button(ui, "Quit") {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if let Some(game) = &self.game {
            ui.add_space(14.0);
            ui.label("Save pipes");
            for (slot, label) in ["A", "B", "C"].iter().enumerate() {
                let text = if game.session.slots.valid(slot) {
                    let tally = game
                        .session
                        .slots
                        .load_slot(slot)
                        .map(|r| (r[0x42] >> 4) as u16 * 10 + (r[0x42] & 0x0F) as u16)
                        .unwrap_or(0);
                    format!("Pipe {label}: {tally} stages")
                } else {
                    format!("Pipe {label}: empty")
                };
                ui.label(text);
            }
        }
        ui.add_space(20.0);
        ui.label("Arrows / D-pad: navigate · Enter / South: confirm");
        ui.label("Esc / Controller Start: menu · East: back");
        if let Some(message) = &self.status {
            ui.add_space(12.0);
            ui.colored_label(egui::Color32::LIGHT_RED, message);
        }
        if !self.allow_save && ui.button("Reset unreadable settings to defaults").clicked() {
            self.settings = Default::default();
            self.allow_save = true;
            // Force replacement even when the current values already match defaults.
            self.saved_settings.version = 0;
            self.persist();
            self.status = None;
        }
    }

    pub(super) fn settings_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.heading("Settings");
        ui.label("Changes apply immediately and save automatically.");
        ui.add_space(12.0);
        ui.strong("Display");
        let adjust = self.menu.row_adjustment();
        let label = match self.settings.zoom {
            Zoom::Auto => "Zoom: auto integer fit".into(),
            Zoom::Integer(n) => format!("Zoom: {n}×"),
        };
        if self.menu.button(ui, label) || adjust != 0 {
            let current = match self.settings.zoom {
                Zoom::Auto => 0,
                Zoom::Integer(n) => n as i32,
            };
            let next = (current + if adjust != 0 { adjust } else { 1 }).rem_euclid(9);
            self.zoom(ctx, if next == 0 { None } else { Some(next as u8) });
        }
        if self.menu.button(
            ui,
            format!(
                "Fullscreen: {}",
                if self.settings.fullscreen {
                    "on"
                } else {
                    "off"
                }
            ),
        ) {
            self.fullscreen(ctx);
        }
        let adjust = self.menu.row_adjustment();
        if self.menu.button(ui, format!("Filter: {}", self.settings.filter.label()))
            || adjust != 0
        {
            self.cycle_filter(if adjust != 0 { adjust } else { 1 });
        }
        ui.label("+ / − zoom · F11 fullscreen · F4 cycles filters · game view stays 160 × 144");
        ui.add_space(10.0);
        ui.strong("Audio");
        let adjust = self.menu.row_adjustment();
        if self.menu.button(
            ui,
            format!("Volume: {:.0}%  ← / →", self.settings.volume * 100.0),
        ) || adjust != 0
        {
            self.settings.volume = ((self.settings.volume * 10.0).round()
                + if adjust != 0 { adjust as f32 } else { 1.0 })
            .clamp(0.0, 10.0)
                / 10.0;
        }
        if self.menu.button(
            ui,
            format!("Mute: {}", if self.settings.muted { "on" } else { "off" }),
        ) {
            self.settings.muted = !self.settings.muted;
        }
        ui.add_space(10.0);
        ui.strong("Input");
        if self
            .menu
            .button(ui, format!("Controller: {}", self.controllers.name()))
        {
            self.settings.preferred_controller = self.controllers.cycle(&mut self.input);
            self.input.suppress_until_release();
        }
        let adjust = self.menu.row_adjustment();
        if self.menu.button(
            ui,
            format!(
                "Stick dead zone: {:.0}%  ← / →",
                self.settings.dead_zone * 100.0
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
            ui.label(error);
        }
        ui.label("Choose an action, then press a key or controller button. Esc cancels.");
        ui.label("Rebinding keeps the other device's controls. Controller Start opens the menu.");
        for action in Action::ALL {
            let buttons = self.settings.bindings.0[action as usize]
                .iter()
                .map(|b| b.label())
                .collect::<Vec<_>>()
                .join(" / ");
            let label = if self.capture == Some(action) {
                format!("{}: press a button…", action.label())
            } else {
                format!("{}: {}", action.label(), buttons)
            };
            if self.menu.button(ui, label) && self.capture.is_none() {
                self.capture = Some(action);
                self.input.suppress_until_release();
            }
        }
        if self.menu.button(ui, "Reset controls") && self.capture.is_none() {
            self.settings.bindings = Bindings::default();
            self.input.suppress_until_release();
        }
        if self.menu.button(ui, "Back") && self.capture.is_none() {
            self.persist();
            self.navigate(ctx, self.return_screen);
        }
        if let Some(message) = &self.status {
            ui.colored_label(egui::Color32::LIGHT_RED, message);
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
