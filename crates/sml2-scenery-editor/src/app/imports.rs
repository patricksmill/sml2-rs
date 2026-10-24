//! Reuse the game's import/cache boundary without involving player saves.

use super::EditorApp;
use crate::{dx_import::{self, Event, Worker}, settings::{Settings, VanillaInstall}, vanilla_import};
use eframe::egui::{self, Context, Ui};
use std::{path::PathBuf, sync::mpsc};

pub(super) type RomResult = Result<(VanillaInstall, PathBuf), String>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ImportKind { Rom, Dx }

impl EditorApp {
    pub(super) fn restore_dx(&mut self) {
        let Some(key) = self.settings.dx_installation.clone() else {
            self.status = Some("Import DX graphics in Game data to preview them.".into());
            self.settings.dx_mode = false;
            return;
        };
        let assets = self.options.assets.clone();
        self.dx_restore = true;
        self.dx_worker = Some(Worker::start(move |cancel, _| {
            dx_import::presentation(dx_import::restore(&dx_import::cache_root()?, &assets, key, cancel))
        }));
    }

    fn choose_import(&mut self, kind: ImportKind) {
        if self.dx_worker.is_some() || self.rom_worker.is_some() { return; }
        self.import_kind = kind;
        self.dx_worker = Some(Worker::choose_file(move || {
            if cfg!(test) { return None; }
            match kind {
                ImportKind::Rom => rfd::FileDialog::new().set_title("Import original ROM").add_filter("Game Boy ROM", &["gb", "gbc"]).pick_file(),
                ImportKind::Dx => rfd::FileDialog::new().set_title("Import DX ROM or IPS").add_filter("DX ROM or patch", &["gb", "gbc", "ips"]).pick_file(),
            }
        }));
    }

    fn import_path(&mut self, kind: ImportKind, path: PathBuf) {
        if self.dx_worker.is_some() || self.rom_worker.is_some() { return; }
        self.finish_editor_gesture();
        match kind {
            ImportKind::Rom => {
                let (sender, receiver) = mpsc::channel();
                self.rom_worker = Some(receiver);
                std::thread::spawn(move || {
                    let result = (|| {
                        let rom = vanilla_import::read_rom(&path)?;
                        let destination = vanilla_import::vanilla_cache()?;
                        vanilla_import::install_rom(&destination, &rom)?;
                        let identity = serde_json::from_slice(&std::fs::read(destination.join("manifest.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                        Ok((identity, destination))
                    })();
                    let _ = sender.send(result);
                });
            }
            ImportKind::Dx => {
                let assets = self.options.assets.clone();
                self.dx_restore = false;
                self.dx_worker = Some(Worker::start(move |cancel, progress| {
                    dx_import::presentation(dx_import::import_path(&dx_import::cache_root()?, &assets, &path, cancel, progress))
                }));
            }
        }
        self.status = Some("Importing game graphics…".into());
    }

    /// Merge installation identities only; authoring never saves game display
    /// preferences or progress, and never replaces unreadable settings.
    fn persist_installation(&mut self) {
        if !self.allow_save { return; }
        let result = Settings::load(&self.options.paths.settings).and_then(|mut current| {
            current.vanilla = self.settings.vanilla.clone();
            current.dx_installation = self.settings.dx_installation.clone();
            current.save(&self.options.paths.settings)
        });
        if let Err(error) = result { self.status = Some(format!("Installation settings could not be saved: {error}")); }
    }

    pub(super) fn poll_imports(&mut self, ctx: &Context) {
        let rom = self.rom_worker.as_ref().and_then(|worker| match worker.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("ROM importer stopped unexpectedly".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = rom {
            self.rom_worker = None;
            match result {
                Ok((identity, assets)) => {
                    self.settings.vanilla = Some(identity);
                    self.options.assets = assets;
                    self.options.explicit_assets = false;
                    self.persist_installation();
                    self.game = None;
                    self.dx_bundle = None;
                    self.editor = Default::default();
                    self.load_game();
                    if self.game.is_some() { self.open_editor(ctx); }
                }
                Err(error) => self.status = Some(error),
            }
        }
        let events: Vec<_> = self.dx_worker.as_ref().map(|w| w.events.try_iter().collect()).unwrap_or_default();
        for event in events {
            match event {
                Event::FileChosen(path) => {
                    self.dx_worker = None;
                    self.input.suppress_until_release();
                    if let Some(path) = path { self.import_path(self.import_kind, path); }
                    else { self.status = Some("No file selected. You can enter its path below.".into()); }
                }
                Event::Progress(text) => self.status = Some(text.into()),
                Event::Finished(result) => {
                    self.dx_worker = None;
                    let restoring = std::mem::take(&mut self.dx_restore);
                    match result {
                        Ok((key, bundle)) => {
                            self.settings.dx_installation = Some(key);
                            self.dx_bundle = Some(bundle);
                            self.status = Some("DX graphics ready".into());
                            if !restoring { self.persist_installation(); }
                        }
                        Err(error) => { self.settings.dx_mode = false; self.status = Some(error); }
                    }
                }
            }
        }
    }

    pub(super) fn data_view(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.heading("Scenery Editor");
        ui.label("Use your game's imported ROM, or import a supported original ROM here.");
        let idle = self.dx_worker.is_none() && self.rom_worker.is_none();
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(idle, egui::Button::new("Import original ROM…")).clicked() { self.choose_import(ImportKind::Rom); }
            if ui.add_enabled(idle && self.game.is_some(), egui::Button::new("Import DX ROM / IPS…")).clicked() { self.choose_import(ImportKind::Dx); }
            if ui.add_enabled(self.game.is_some(), egui::Button::new("Return to editor")).clicked() { self.open_editor(ctx); }
        });
        ui.separator();
        ui.label("File path (if the system picker is unavailable)");
        ui.text_edit_singleline(&mut self.input_path);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.import_kind, ImportKind::Rom, "Original ROM");
            ui.selectable_value(&mut self.import_kind, ImportKind::Dx, "DX ROM / IPS");
            if ui.add_enabled(idle && !self.input_path.is_empty() && (self.import_kind == ImportKind::Rom || self.game.is_some()), egui::Button::new("Import path")).clicked() {
                self.import_path(self.import_kind, PathBuf::from(&self.input_path));
            }
        });
        if !idle { ui.spinner(); }
        if ui.button("Quit").clicked() { self.finish_editor_gesture(); ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
    }
}
