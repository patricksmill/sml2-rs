//! A release export is explicit and independent from draft autosaving.

use super::EditorApp;
use crate::settings::atomic_write;
use eframe::egui::{self, Context};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

pub(super) struct ExportDialog {
    path: String,
    bytes: Vec<u8>,
    picker: Option<Receiver<Option<PathBuf>>>,
    error: Option<String>,
}

impl EditorApp {
    pub(in crate::app) fn editor_export_open(&self) -> bool {
        self.editor.export.is_some()
    }

    pub(in crate::app) fn editor_export_path(&self) -> Option<&str> {
        self.editor
            .export
            .as_ref()
            .map(|dialog| dialog.path.as_str())
    }

    pub(super) fn begin_export(&mut self) {
        self.finish_editor_gesture();
        let Some(work) = &self.editor.work else {
            self.editor.export = None;
            return;
        };
        if work.load_error.is_some() {
            self.editor.export = None;
            return;
        }
        let bytes = match serde_json::to_vec_pretty(&work.scenery) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = Some(error.to_string());
                return;
            }
        };
        let filename = self.editor.scene.map_or_else(
            || format!("scenery-{:02}.json", self.editor.level),
            |scene| format!("scenery-{}.json", scene.file_stem()),
        );
        self.editor.export = Some(ExportDialog {
            path: filename,
            bytes,
            picker: None,
            error: None,
        });
    }

    pub(super) fn export_to(&mut self, path: &Path) -> Result<(), String> {
        let dialog = self
            .editor
            .export
            .as_ref()
            .ok_or("No scenery selected for export")?;
        atomic_write(path, &dialog.bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub(in crate::app) fn export_window(&mut self, ctx: &Context) {
        let Some(dialog) = &mut self.editor.export else {
            return;
        };
        if let Some(picker) = &dialog.picker {
            match picker.try_recv() {
                Ok(path) => {
                    if let Some(path) = path {
                        dialog.path = path.to_string_lossy().into_owned();
                    }
                    dialog.picker = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => dialog.picker = None,
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                }
            }
        }
        let mut export = false;
        let mut open = true;
        egui::Window::new("Export scenery").open(&mut open).collapsible(false).show(ctx, |ui| {
            ui.label("Choose a destination for this scenery file. Exported scenery can be included in a future port release.");
            ui.label("File path");
            ui.text_edit_singleline(&mut dialog.path);
            if let Some(error) = &dialog.error { ui.colored_label(super::theme::ERROR, error); }
            ui.horizontal(|ui| {
                if ui.add_enabled(dialog.picker.is_none(), egui::Button::new("Browse…")).clicked() {
                    let filename = Path::new(&dialog.path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "scenery.json".into());
                    let (sender, receiver) = mpsc::channel();
                    dialog.picker = Some(receiver);
                    std::thread::spawn(move || {
                        let path = if cfg!(test) { None } else { rfd::FileDialog::new().set_title("Export scenery for release").set_file_name(filename).add_filter("Scenery JSON", &["json"]).save_file() };
                        let _ = sender.send(path);
                    });
                }
                export = ui.add_enabled(dialog.picker.is_none() && !dialog.path.trim().is_empty(), egui::Button::new("Export")).clicked();
            });
        });
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            open = false;
        }
        if !open {
            self.editor.export = None;
            return;
        }
        if export {
            let path = PathBuf::from(&self.editor.export.as_ref().unwrap().path);
            match self.export_to(&path) {
                Ok(()) => {
                    self.editor.export = None;
                    self.status = Some(format!("Exported {}", path.display()));
                }
                Err(error) => self.editor.export.as_mut().unwrap().error = Some(error),
            }
        }
    }
}
