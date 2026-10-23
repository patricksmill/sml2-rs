use super::file_picker::{FilePicker, PickerResult};
use super::theme::{self, Value};
use super::DesktopApp;
use crate::dx_import::{self, Event, Worker};
use eframe::egui::{Context, Ui};

impl DesktopApp {
    pub(super) fn restore_dx(&mut self) {
        let Some(key) = self.settings.dx_installation.clone() else {
            return;
        };
        let assets = self.options.assets.clone();
        self.dx_message = "Checking installed DX graphics…".into();
        self.dx_worker = Some(Worker::start(move |cancel, _| {
            dx_import::presentation(dx_import::restore(
                &dx_import::cache_root()?,
                &assets,
                key,
                cancel,
            ))
        }));
    }

    pub(super) fn poll_dx(&mut self) {
        let events: Vec<_> = self
            .dx_worker
            .as_ref()
            .map(|worker| worker.events.try_iter().collect())
            .unwrap_or_default();
        for event in events {
            match event {
                Event::FileChosen(path) => {
                    self.input.suppress_until_release();
                    self.menu = Default::default();
                    let cancelled = self
                        .dx_worker
                        .as_ref()
                        .is_some_and(|w| w.cancelled.load(std::sync::atomic::Ordering::Acquire));
                    self.dx_worker = None;
                    if cancelled {
                        self.dx_message =
                            "DX import cancelled; previous installation retained".into();
                    } else if let Some(path) = path {
                        self.import_dx_path(path);
                    } else {
                        self.dx_message =
                            "No file selected in the system dialog. Choose a ROM in the file browser or cancel."
                                .into();
                        self.dx_picker = Some(FilePicker::new(
                            std::env::current_dir().unwrap_or_else(|_| self.options.assets.clone()),
                        ));
                    }
                }
                Event::Progress(text) => self.dx_message = text.into(),
                Event::Finished(result) => {
                    let cancelled = self
                        .dx_worker
                        .as_ref()
                        .is_some_and(|w| w.cancelled.load(std::sync::atomic::Ordering::Acquire));
                    self.dx_worker = None;
                    if cancelled {
                        self.dx_message =
                            "DX import cancelled; previous installation retained".into();
                    } else {
                        match result {
                            Ok((key, bundle)) => {
                                self.settings.dx_installation = Some(key);
                                self.dx_bundle = Some(bundle);
                                self.dx_message = format!("Installed {}", sml2_dx::REVISION);
                            }
                            Err(error) => self.dx_message = error,
                        }
                    }
                }
            }
        }
    }

    pub(super) fn import_dx_path(&mut self, path: std::path::PathBuf) {
        let assets = self.options.assets.clone();
        self.dx_picker = None;
        // Enter/Space used by the picker must not activate a Home menu item.
        self.input.suppress_until_release();
        self.menu = Default::default();
        self.dx_worker = Some(Worker::start(move |cancel, progress| {
            dx_import::presentation(dx_import::import_path(
                &dx_import::cache_root()?,
                &assets,
                &path,
                cancel,
                progress,
            ))
        }));
    }

    /// Settings → Mods: DX Mode, its character and the ROM import.
    pub(super) fn dx_controls(&mut self, ui: &mut Ui) {
        theme::section(ui, "SML2DX");
        let installed = self.dx_bundle.is_some();
        if self
            .menu
            .row(ui, "DX Mode", Value::Toggle(self.settings.dx_mode))
        {
            self.settings.dx_mode = !self.settings.dx_mode;
            // Turning DX Mode on without graphics goes straight to the import.
            if self.settings.dx_mode && !installed && self.dx_worker.is_none() {
                self.choose_dx_rom();
            }
        }
        let adjust = self.menu.row_adjustment();
        let character = self.settings.character;
        let shown = Value::Choice(character.label().into());
        if self.menu.row(ui, "Character", shown) || adjust != 0 {
            self.settings.character = character.next();
        }
        theme::section(ui, "GRAPHICS");
        if self.dx_worker.is_none() {
            let (label, status) = if installed {
                ("Re-import SML2DX ROM…", "v1.8.1 installed")
            } else {
                ("Import SML2DX ROM…", "not installed")
            };
            if self.dx_picker.is_none() && self.menu.row(ui, label, Value::Text(status.into())) {
                self.choose_dx_rom();
            }
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                theme::note(ui, "Working…");
            });
            if self.menu.row(ui, "Cancel DX import", Value::None) {
                if let Some(worker) = &self.dx_worker {
                    worker.cancel();
                }
                self.dx_message = "Cancelling DX import…".into();
            }
        }
        if !self.dx_message.is_empty() {
            theme::note(ui, &self.dx_message);
        }
        theme::note(
            ui,
            "Supply your own SML2DX v1.8.1 ROM. DX Mode redraws every level in DX graphics and \
             color, as Mario or Luigi, in the normal game with your normal saves. The title and \
             file select are DX too; boss arenas and the map keep the display filter for now, and DX gameplay changes are \
             not included.",
        );
    }

    fn choose_dx_rom(&mut self) {
        self.dx_message = "Choose the patched SML2DX v1.8.1 ROM…".into();
        self.dx_worker = Some(Worker::choose_file(|| {
            // Unit tests drive this path; they must never open a native dialog.
            if cfg!(test) {
                return None;
            }
            rfd::FileDialog::new()
                .set_title("Import SML2DX v1.8.1")
                .add_filter("Game Boy Color ROM", &["gbc", "gb"])
                .pick_file()
        }));
    }

    /// The in-app ROM browser, drawn over whichever screen is showing.
    pub(super) fn dx_picker_window(&mut self, ctx: &Context) {
        let choice = self.dx_picker.as_mut().and_then(|picker| picker.show(ctx));
        match choice {
            Some(PickerResult::Selected(path)) => self.import_dx_path(path),
            Some(PickerResult::Cancelled) => {
                self.dx_picker = None;
                self.input.suppress_until_release();
                self.menu = Default::default();
                self.dx_message = "DX import cancelled; previous installation retained".into();
            }
            None => {}
        }
    }
}
