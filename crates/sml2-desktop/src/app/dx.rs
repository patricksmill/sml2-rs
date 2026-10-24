use super::file_picker::{FilePicker, PickerResult};
use super::theme::{self, Value};
use super::DesktopApp;
use crate::dx_import::{self, Event, Worker};
use eframe::egui::{Context, Ui};

/// Settings owns the persisted choice; a door action can update it during this tick.
pub(super) fn prepare_tick(
    session: &mut sml2_runtime::Session,
    settings: &crate::settings::Settings,
    installed: bool,
) {
    session.character = settings.character;
    if let sml2_runtime::session::SessionState::FileSelect(file_select) = &mut session.state {
        file_select.set_character_door_enabled(settings.dx_mode && installed);
    }
}

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
                        let pick = self.library_pick.take();
                        self.dx_message = match pick {
                            Some(super::LibraryPick::Rom) => "ROM import cancelled.".into(),
                            _ => "DX import cancelled; previous installation retained".into(),
                        };
                    } else if let Some(path) = path {
                        if self.library_pick.is_some() {
                            self.accept_library_file(path);
                        } else {
                            self.import_dx_path(path);
                        }
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

    /// Settings → Mods keeps the character. Import and the DX switch live on the library.
    pub(super) fn dx_controls(&mut self, ui: &mut Ui) {
        theme::section(ui, "SML2DX");
        let adjust = self.menu.row_adjustment();
        let character = self.settings.character;
        let shown = Value::Choice(character.label().into());
        if self.menu.row(ui, "Character", shown) || adjust != 0 {
            self.settings.character = character.next();
        }
        if !self.dx_message.is_empty() {
            theme::note(ui, &self.dx_message);
        }
    }

    /// One chooser at a time. A second request is ignored while an import is running.
    pub(super) fn start_library_import(&mut self, kind: super::LibraryPick) {
        if self.library.importing || self.dx_worker.is_some() || self.library_pick.is_some() {
            return;
        }
        let title = match kind {
            super::LibraryPick::Rom => "Import Super Mario Land 2",
            super::LibraryPick::Dx => "Import SML2DX v1.8.1",
        };
        self.library_pick = Some(kind);
        self.dx_worker = Some(Worker::choose_file(move || {
            // Unit tests drive this path; they must never open a native dialog.
            if cfg!(test) {
                return None;
            }
            rfd::FileDialog::new()
                .set_title(title)
                .add_filter("Game Boy ROM", &["gb", "gbc"])
                .pick_file()
        }));
    }

    /// The in-app ROM browser, drawn over whichever screen is showing.
    pub(super) fn dx_picker_window(&mut self, ctx: &Context) {
        let choice = self.dx_picker.as_mut().and_then(|picker| picker.show(ctx));
        match choice {
            Some(PickerResult::Selected(path)) => {
                if self.library_pick.is_some() {
                    self.accept_library_file(path);
                } else {
                    self.import_dx_path(path);
                }
            }
            Some(PickerResult::Cancelled) => {
                self.dx_picker = None;
                self.input.suppress_until_release();
                self.menu = Default::default();
                let pick = self.library_pick.take();
                self.dx_message = match pick {
                    Some(super::LibraryPick::Rom) => "ROM import cancelled.".into(),
                    _ => "DX import cancelled; previous installation retained".into(),
                };
            }
            None => {}
        }
    }
}
