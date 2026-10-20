//! Debug-panel controls: warps, stage progress, golden coins and the purse.
//!
//! The widgets only collect an action; [`DesktopApp::apply_debug`] runs it after
//! the window closes, so a click never mutates the session while egui still
//! borrows the app.

use super::DesktopApp;
use eframe::egui::{self, Ui};
use sml2_core::castle_sequence::{GOLDEN_COIN_OFFSETS, LEVEL_PROGRESS_OFFSETS};
use sml2_runtime::session::SessionState;

/// `(label, screen, node)` for the nine overworld screens.
///
/// Each zone's node is the one its door lands on (`$3C60B`'s `NODE_DOORS` in
/// `sml2_core::map_sim`), so the warp puts the cursor where a played entry would.
/// The hub uses node 0: once stage 1 is cleared the map moves the cursor to
/// `$14` by itself (`$3C316`).
const ZONES: [(&str, u8, u8); 9] = [
    ("Hub", 0, 0x00),
    ("Pumpkin", 1, 0x28),
    ("Tree", 2, 0x23),
    ("Macro", 3, 0x32),
    ("Space", 4, 0x3C),
    ("Turtle", 5, 0x4B),
    ("Mario", 6, 0x50),
    ("Wario", 7, 0x5A),
    ("Castle", 8, 0x72),
];

/// One debug edit, applied after the panel's frame.
pub(super) enum DebugAction {
    WarpMap { screen: u8, node: u8 },
    /// Walk into a zone through its hub door, as a played entry does.
    EnterZone(u8),
    WarpLevel(u8),
    ClearAllLevels,
    UnlockRoutes,
    WipeProgress,
    SetGoldenCoin { index: usize, held: bool },
    AddCoins(i32),
    AddLives(i32),
    /// Write the live record into the active pipe and the slot image.
    Save,
}

impl DesktopApp {
    pub(super) fn debug_controls(&mut self, ui: &mut Ui) -> Option<DebugAction> {
        let mut action = None;
        let (progress, coins, lives, state) = match &self.game {
            Some(game) => (
                game.session.debug_progress(),
                game.session.debug_coins(),
                game.session.debug_lives(),
                state_label(&game.session.state),
            ),
            None => ([0; 42], 0, 0, "no game loaded".to_string()),
        };

        ui.separator();
        ui.monospace(format!("{state}  ·  coins {coins:04}  ·  lives {lives:02}"));
        egui::CollapsingHeader::new("Warp")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (label, screen, node) in ZONES {
                        if ui.button(label).clicked() {
                            self.debug_screen = screen;
                            self.debug_node = node;
                            action = Some(if screen == 0 {
                                DebugAction::WarpMap { screen, node }
                            } else {
                                DebugAction::EnterZone(screen)
                            });
                        }
                    }
                });
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("debug-warp-screen")
                        .width(96.0)
                        .selected_text(
                            ZONES
                                .iter()
                                .find(|(_, id, _)| *id == self.debug_screen)
                                .map(|(name, _, _)| *name)
                                .unwrap_or("?"),
                        )
                        .show_ui(ui, |ui| {
                            for (name, screen, node) in ZONES {
                                // A new screen starts from its own landing node,
                                // not the node picked for the previous screen.
                                if ui
                                    .selectable_value(&mut self.debug_screen, screen, name)
                                    .clicked()
                                {
                                    self.debug_node = node;
                                }
                            }
                        });
                    ui.label("node");
                    ui.add(
                        egui::DragValue::new(&mut self.debug_node)
                            .range(0..=0x7F)
                            .hexadecimal(2, false, true),
                    );
                    if let Some(hint) = self.node_hint(self.debug_node) {
                        ui.label(hint);
                    }
                    if ui.button("Teleport").clicked() {
                        action = Some(DebugAction::WarpMap {
                            screen: self.debug_screen,
                            node: self.debug_node,
                        });
                    }
                });
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("debug-warp-level")
                        .width(150.0)
                        .selected_text(
                            self.levels
                                .iter()
                                .find(|(id, _)| *id == self.debug_level)
                                .map(|(_, label)| label.as_str())
                                .unwrap_or("?"),
                        )
                        .show_ui(ui, |ui| {
                            for (id, label) in &self.levels {
                                ui.selectable_value(&mut self.debug_level, *id, label);
                            }
                        });
                    if ui.button("Enter level").clicked() {
                        action = Some(DebugAction::WarpLevel(self.debug_level));
                    }
                });
                ui.label("Zone buttons walk in through the hub door; Teleport places the cursor directly.");
            });

        ui.separator();
        egui::CollapsingHeader::new("Progress")
            .default_open(true)
            .show(ui, |ui| {
                let slots = self
                    .game
                    .as_ref()
                    .map(|game| game.session.debug_level_slots())
                    .unwrap_or_default();
                let cleared = slots
                    .iter()
                    .filter(|slot| progress[**slot as usize] & 0x80 != 0)
                    .count();
                ui.monospace(format!("{cleared}/{} stages cleared", slots.len()));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Clear all stages").clicked() {
                        action = Some(DebugAction::ClearAllLevels);
                    }
                    if ui.button("Unlock routes").clicked() {
                        action = Some(DebugAction::UnlockRoutes);
                    }
                    if ui.button("Wipe progress").clicked() {
                        action = Some(DebugAction::WipeProgress);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    for index in 0..GOLDEN_COIN_OFFSETS.len() {
                        let mut held = progress[GOLDEN_COIN_OFFSETS[index]] & 0x80 != 0;
                        let label = match coin_level(index) {
                            Some(level) => format!("Coin {} · L{level}", index + 1),
                            None => format!("Coin {}", index + 1),
                        };
                        if ui.checkbox(&mut held, label).changed() {
                            action = Some(DebugAction::SetGoldenCoin { index, held });
                        }
                    }
                });
                ui.label(
                    "A coin is its boss stage's clear bit: removing one un-clears that stage.",
                );
            });

        ui.separator();
        egui::CollapsingHeader::new("Purse")
            .default_open(true)
            .show(ui, |ui| {
                let deltas = [("+10", 10), ("+100", 100), ("−10", -10), ("−100", -100)];
                ui.horizontal_wrapped(|ui| {
                    for (label, delta) in deltas {
                        if ui.button(label).clicked() {
                            action = Some(DebugAction::AddCoins(delta));
                        }
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    if ui.button("+1 life").clicked() {
                        action = Some(DebugAction::AddLives(1));
                    }
                    if ui.button("−1 life").clicked() {
                        action = Some(DebugAction::AddLives(-1));
                    }
                });
            });

        ui.separator();
        ui.horizontal(|ui| {
            let pipe = self.game.as_ref().and_then(|game| game.session.active_slot);
            let label = match pipe {
                Some(slot) => format!("Save to pipe {}", (b'A' + slot) as char),
                None => "Save slots".to_string(),
            };
            if ui.button(label).clicked() {
                action = Some(DebugAction::Save);
            }
            ui.label("Edits stay in memory until saved.");
        });

        if let Some(note) = &self.debug_note {
            ui.separator();
            ui.colored_label(
                if self.debug_note_error {
                    egui::Color32::LIGHT_RED
                } else {
                    egui::Color32::LIGHT_GREEN
                },
                note,
            );
        }
        action
    }

    pub(super) fn apply_debug(&mut self, action: DebugAction) {
        let warped = matches!(
            &action,
            DebugAction::WarpMap { .. } | DebugAction::EnterZone(_) | DebugAction::WarpLevel(_)
        );
        let Some(game) = &mut self.game else {
            return;
        };
        let session = &mut game.session;
        let (note, error) = match action {
            DebugAction::WarpMap { screen, node } => match session.debug_warp_map(screen, node) {
                Ok(()) => (
                    format!("warped to {} · node ${node:02X}", screen_name(screen)),
                    false,
                ),
                Err(error) => (error, true),
            },
            DebugAction::EnterZone(screen) => match session.debug_enter_zone(screen) {
                Ok(()) => (format!("entering {}", screen_name(screen)), false),
                Err(error) => (error, true),
            },
            DebugAction::WarpLevel(id) => match session.debug_warp_level(id) {
                Ok(()) => (format!("level {id} live"), false),
                Err(error) => (error, true),
            },
            DebugAction::ClearAllLevels => {
                let count = session.debug_clear_all_levels();
                (format!("{count} stages marked cleared"), false)
            }
            DebugAction::UnlockRoutes => {
                session.debug_unlock_routes();
                ("secret paths opened".to_string(), false)
            }
            DebugAction::WipeProgress => {
                session.debug_wipe_progress();
                ("progress wiped".to_string(), false)
            }
            DebugAction::SetGoldenCoin { index, held } => {
                session.debug_set_golden_coin(index, held);
                (
                    format!(
                        "golden coin {} {}",
                        index + 1,
                        if held { "collected" } else { "removed" }
                    ),
                    false,
                )
            }
            DebugAction::AddCoins(delta) => {
                session.debug_add_coins(delta);
                (format!("coins {:04}", session.debug_coins()), false)
            }
            DebugAction::AddLives(delta) => {
                session.debug_add_lives(delta);
                (format!("lives {:02}", session.debug_lives()), false)
            }
            DebugAction::Save => match game.save_now() {
                Ok(()) => ("saved".to_string(), false),
                Err(error) => (format!("save failed: {error}"), true),
            },
        };
        self.debug_note = Some(note);
        self.debug_note_error = error;
        // A warp replaces the frame's state; a button held across the click must
        // not read as a fresh press on the landing (which would open a door or
        // enter a level by itself).
        if warped && !error {
            self.input.suppress_until_release();
        }
    }

    /// Cache the level picker's labels: the extracted tileset name is the only
    /// per-level zone label the assets carry (`treezone5`, `pumpkinzone1`, ...).
    pub(super) fn level_choices(&mut self) {
        self.levels.clear();
        let Some(game) = &self.game else {
            return;
        };
        for id in 0..32u8 {
            let Ok(content) = game.session.assets.level(id) else {
                continue;
            };
            let zone = zone_name(&content.manifest.graphics.file);
            self.levels.push((id, format!("L{id:02} {zone}")));
        }
    }

    /// What the node picker's current id is: the stage it enters, or a path
    /// point with no stage.
    fn node_hint(&self, node: u8) -> Option<String> {
        let game = self.game.as_ref()?;
        let level = game.session.assets.nodes.level_byte(node)?;
        if level >= 0xFE {
            return Some("path point".into());
        }
        let zone = game
            .session
            .assets
            .level(level)
            .map(|content| zone_name(&content.manifest.graphics.file))
            .unwrap_or("");
        Some(if zone.is_empty() {
            format!("→ L{level}")
        } else {
            format!("→ L{level} {zone}")
        })
    }
}

/// The tileset name out of `gfx/tilesets/treezone5.2bpp`: the assets' only
/// per-level zone label.
fn zone_name(file: &str) -> &str {
    file.rsplit('/')
        .next()
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("")
}

/// The game level whose clear bit is golden coin `index`, from the reveal's own
/// `sCurLevel` -> `sLevelProgress` table.
fn coin_level(index: usize) -> Option<u8> {
    let offset = *GOLDEN_COIN_OFFSETS.get(index)? as u8;
    LEVEL_PROGRESS_OFFSETS
        .iter()
        .position(|&level_offset| level_offset == offset)
        .map(|level| level as u8)
}

fn state_label(state: &SessionState) -> String {
    match state {
        SessionState::Overworld(map) => {
            format!("{} · node ${:02X}", screen_name(map.screen_id), map.node)
        }
        SessionState::InLevel(ctx) => format!("level {}", ctx.manifest.level_id),
        SessionState::Title(_) => "title".into(),
        SessionState::FileSelectLoading(_) => "loading file select".into(),
        SessionState::FileSelect(_) => "file select".into(),
        SessionState::Demo(demo) => format!("demo {}", demo.index),
        SessionState::Castle(seq) => format!("castle reveal {:02X}", seq.state),
        SessionState::GameOver => "game over".into(),
    }
}

/// The map screen's name, for the status line and the warp note.
fn screen_name(screen: u8) -> String {
    match ZONES.iter().find(|(_, id, _)| *id == screen) {
        Some((name, _, _)) => (*name).to_string(),
        None => match screen {
            9..=12 => format!("coin room {screen}"),
            13 | 14 => format!("bell room {screen}"),
            _ => format!("screen {screen}"),
        },
    }
}
