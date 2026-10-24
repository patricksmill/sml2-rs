//! Settings → Game owns the original ROM and optional DX installation.
use super::theme::{self, Value};
use super::{DesktopApp, LibraryPick};
use crate::library::Row;
use eframe::egui::Ui;

impl DesktopApp {
    pub(super) fn can_import_dx(&self) -> bool {
        (self.library.vanilla_ready || self.game.is_some())
            && !self.library.checking
            && !self.library.importing
    }

    pub(super) fn installation_page(&mut self, ui: &mut Ui) {
        theme::section(ui, "ORIGINAL GAME");
        let replace = self.settings.vanilla.is_some() || self.library.vanilla_ready;
        let label = if replace {
            "Replace ROM…"
        } else {
            "Import ROM…"
        };
        if self.menu.row(ui, label, Value::None)
            && self.library.row_enabled(Row::Rom)
            && !self.library.checking
        {
            self.start_library_import(LibraryPick::Rom);
        }
        if self.library.rom_error.is_empty() {
            theme::note(ui, &self.library.row_status(Row::Rom));
        } else {
            theme::error(ui, &self.library.rom_error);
        }
        theme::note(
            ui,
            "Use the original Super Mario Land 2 UE v1.0 ROM (.gb or .gbc).",
        );

        theme::section(ui, "SML2DX · OPTIONAL");
        if self.settings.dx_installation.is_some()
            && self
                .menu
                .row(ui, "DX Mode", Value::Toggle(self.settings.dx_mode))
            && self.can_import_dx()
        {
            self.settings.dx_mode = !self.settings.dx_mode;
            self.library.dx_mode = self.settings.dx_mode;
        }
        let label = if self.settings.dx_installation.is_some() {
            "Replace DX ROM / IPS…"
        } else {
            "Add DX ROM / IPS…"
        };
        if self.menu.row(ui, label, Value::None) && self.can_import_dx() {
            self.start_library_import(LibraryPick::Dx);
        }
        theme::note(
            ui,
            if self.library.vanilla_ready || self.game.is_some() {
                "SML2DX v1.8.1: a patched ROM or IPS patch."
            } else {
                "Import the original ROM before adding DX."
            },
        );
        if self.game.is_some() {
            theme::note(
                ui,
                "ROM replacement applies the next time you launch the game.",
            );
        }
        if self.dx_worker.is_some() && self.menu.row(ui, "Cancel import", Value::None) {
            if let Some(worker) = &self.dx_worker {
                worker.cancel();
            }
            self.dx_message = "Cancelling import…".into();
        }
        if !self.dx_message.is_empty() {
            theme::note(ui, &self.dx_message);
        }
        if !self.library.dx_error.is_empty() && self.library.dx_error != self.dx_message {
            theme::error(ui, &self.library.dx_error);
        }
    }
}
