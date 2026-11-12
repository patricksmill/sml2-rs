//! Help menu, update notice and About window.

use super::{DesktopApp, theme};
use crate::update::{self, Phase};
use eframe::egui::{self, Align2, Context, Id, RichText, ViewportCommand};

impl DesktopApp {
    /// Run a check now. Automatic checks respect the daily limit and setting;
    /// manual checks ignore both the limit and the skipped version.
    pub(super) fn check_updates(&mut self, manual: bool) {
        if self.updater.is_busy() || self.updater.is_ready() {
            return;
        }
        if !manual {
            if !update::RELEASE_BUILD || !self.settings.check_for_updates {
                return;
            }
            let now = update::now();
            if !update::auto_check_due(
                self.settings.check_for_updates,
                self.settings.last_update_check,
                now,
            ) {
                return;
            }
            self.settings.last_update_check = Some(now);
        } else {
            self.settings.last_update_check = Some(update::now());
        }
        self.updater
            .check(manual, self.settings.skipped_version.clone());
    }

    pub(super) fn help_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Help", |ui| {
            if ui.button("Getting Started").clicked() {
                update::open_url(update::GETTING_STARTED);
                ui.close();
            }
            if ui.button("What's New").clicked() {
                update::open_url(&update::release_notes_url());
                ui.close();
            }
            if ui.button("Report a Bug / Request a Feature").clicked() {
                update::open_url(update::ISSUES);
                ui.close();
            }
            if ui.button("GitHub Repository").clicked() {
                update::open_url(update::REPOSITORY);
                ui.close();
            }
            ui.separator();
            if ui.button("Check for Updates…").clicked() {
                self.check_updates(true);
                ui.close();
            }
            if ui.button("About Six Golden Coins").clicked() {
                self.about_open = true;
                ui.close();
            }
        });
    }

    /// Release offers belong on the title screen or launcher; a manual check
    /// may also report wherever the player opened Help, except during play.
    pub(super) fn update_notice(&mut self, ctx: &Context, on_title_screen: bool, in_play: bool) {
        let phase = self.updater.phase();
        let manual = self.updater.is_manual() && !in_play;
        let visible = match &phase {
            Phase::Idle => false,
            Phase::Checking | Phase::UpToDate | Phase::SourceBuild => manual,
            Phase::Available | Phase::Downloading { .. } | Phase::Ready | Phase::Failed(_) => {
                manual || on_title_screen
            }
        };
        if !visible {
            return;
        }
        let release = self.updater.release();
        egui::Window::new("Updates")
            .id(Id::new("update-notice"))
            .anchor(Align2::RIGHT_TOP, egui::vec2(-16.0, 56.0))
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .show(ctx, |ui| {
                ui.set_max_width(320.0);
                match &phase {
                    Phase::Idle => {}
                    Phase::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Checking for updates…");
                        });
                    }
                    Phase::Available => {
                        let Some(release) = &release else {
                            return;
                        };
                        ui.label(
                            RichText::new(format!("Version {} is available", release.tag)).strong(),
                        );
                        ui.label(&release.summary);
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Update now").clicked() {
                                self.updater.download();
                            }
                            if ui.button("What's new").clicked() {
                                update::open_url(&release.url);
                            }
                            if ui.button("Skip this version").clicked() {
                                if let Some(tag) = self.updater.skip() {
                                    self.settings.skipped_version = Some(tag);
                                }
                            }
                        });
                    }
                    Phase::Downloading { received, total } => {
                        let total = *total;
                        ui.label("Downloading the update…");
                        let fraction = total
                            .filter(|total| *total > 0)
                            .map_or(0.0, |total| *received as f32 / total as f32);
                        let bar = egui::ProgressBar::new(fraction)
                            .show_percentage()
                            .animate(total.is_none());
                        ui.add(bar);
                        ui.label(format!("{:.1} MiB", *received as f32 / (1024.0 * 1024.0)));
                    }
                    Phase::Ready => {
                        ui.label("The update is downloaded and verified.");
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Restart to update").clicked()
                                && self.updater.install_and_restart()
                            {
                                ctx.send_viewport_cmd(ViewportCommand::Close);
                            }
                            if ui.button("Later").clicked() {
                                self.updater.dismiss();
                            }
                            if let Some(url) = release.as_ref().map(|release| release.url.clone()) {
                                if ui.button("What's new").clicked() {
                                    update::open_url(&url);
                                }
                            }
                        });
                    }
                    Phase::UpToDate => {
                        ui.label("Six Golden Coins is up to date.");
                        if ui.button("Dismiss").clicked() {
                            self.updater.dismiss();
                        }
                    }
                    Phase::SourceBuild => {
                        ui.label("Updates are only available in official release builds.");
                        ui.label("This copy was built from source.");
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Open releases page").clicked() {
                                update::open_url(update::RELEASES_PAGE);
                            }
                            if ui.button("Dismiss").clicked() {
                                self.updater.dismiss();
                            }
                        });
                    }
                    Phase::Failed(message) => {
                        ui.colored_label(theme::ERROR, message);
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Open download page").clicked() {
                                let url = release
                                    .as_ref()
                                    .map_or(update::RELEASES_PAGE, |release| release.url.as_str());
                                update::open_url(url);
                            }
                            if ui.button("Dismiss").clicked() {
                                self.updater.dismiss();
                            }
                        });
                    }
                }
            });
    }

    pub(super) fn about_window(&mut self, ctx: &Context) {
        if !self.about_open {
            return;
        }
        let mut open = self.about_open;
        egui::Window::new("About Six Golden Coins")
            .id(Id::new("about-window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_max_width(360.0);
                ui.label(RichText::new(format!("Six Golden Coins {}", update::VERSION)).strong());
                ui.label("A native Rust desktop edition of Super Mario Land 2: 6 Golden Coins.");
                ui.add_space(6.0);
                ui.label("MIT License · Copyright (c) Patrick Mill");
                ui.label("This project ships no game data; supply your own cartridge dump.");
                ui.label("Not affiliated with Nintendo.");
                ui.label("Built on froggestspirit's marioland2 decompilation.");
                ui.add_space(6.0);
                if ui.button("GitHub Repository").clicked() {
                    update::open_url(update::REPOSITORY);
                }
            });
        self.about_open = open;
    }
}
