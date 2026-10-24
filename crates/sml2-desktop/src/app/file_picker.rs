//! Local fallback when the host cannot provide a native ROM picker.
use eframe::egui::{self, Context};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq)]
pub enum PickerResult {
    Selected(PathBuf),
    Cancelled,
}

#[derive(Debug)]
struct Entry {
    path: PathBuf,
    directory: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputKind {
    Original,
    Dx,
}

impl InputKind {
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Original => &["gb", "gbc"],
            Self::Dx => &["gb", "gbc", "ips"],
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Original => "Step 1: Choose original ROM",
            Self::Dx => "Step 2: Add DX ROM or IPS (optional)",
        }
    }

    fn accepts(self, path: &Path) -> bool {
        path.extension()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                self.extensions()
                    .iter()
                    .any(|ext| name.eq_ignore_ascii_case(ext))
            })
    }
}

fn directory_entries(directory: &Path, kind: InputKind) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory).map_err(|e| format!("Cannot read directory: {e}"))? {
        let entry = entry.map_err(|e| format!("Cannot read directory entry: {e}"))?;
        let path = entry.path();
        // Follow directory links so a user's usual ROM-folder shortcuts work.
        // A broken shortcut or inaccessible entry must not hide the whole folder.
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let directory = metadata.is_dir();
        if directory || metadata.is_file() && kind.accepts(&path) {
            entries.push(Entry { path, directory });
        }
    }
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| a.path.file_name().cmp(&b.path.file_name()))
    });
    Ok(entries)
}

#[derive(Debug, PartialEq, Eq)]
enum PathSelection {
    Directory,
    File,
}

fn classify(path: &Path) -> Result<PathSelection, String> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    if metadata.is_dir() {
        Ok(PathSelection::Directory)
    } else if metadata.is_file() {
        Ok(PathSelection::File)
    } else {
        Err("Choose a regular file or a directory".into())
    }
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub struct FilePicker {
    kind: InputKind,
    directory: PathBuf,
    entries: Vec<Entry>,
    /// Kept separately from display text so clicked non-UTF8 paths round-trip.
    target: PathBuf,
    path_text: String,
    error: Option<String>,
    finished: bool,
}

impl FilePicker {
    pub fn new(start: PathBuf) -> Self {
        Self::for_kind(start, InputKind::Dx)
    }

    pub fn for_kind(start: PathBuf, kind: InputKind) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let start = if start.is_absolute() {
            start
        } else {
            cwd.join(start)
        };
        let mut picker = Self {
            kind,
            directory: cwd.clone(),
            entries: Vec::new(),
            target: start.clone(),
            path_text: start.to_string_lossy().into_owned(),
            error: None,
            finished: false,
        };
        match classify(&start) {
            Ok(PathSelection::Directory) => picker.navigate(start),
            Ok(PathSelection::File) => {
                picker.navigate(start.parent().unwrap_or(&cwd).to_owned());
                picker.set_target(start);
            }
            Err(error) => {
                picker.navigate(cwd);
                picker.set_target(start);
                picker.error = Some(error);
            }
        }
        picker
    }

    fn set_target(&mut self, target: PathBuf) {
        self.path_text = target.to_string_lossy().into_owned();
        self.target = target;
    }

    fn navigate(&mut self, directory: PathBuf) {
        match directory_entries(&directory, self.kind) {
            Ok(entries) => {
                self.entries = entries;
                self.directory = directory.clone();
                self.set_target(directory);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn open_target(&mut self) -> Option<PickerResult> {
        let path = if self.target.is_absolute() {
            self.target.clone()
        } else {
            self.directory.join(&self.target)
        };
        match classify(&path) {
            Ok(PathSelection::Directory) => {
                self.navigate(path);
                None
            }
            Ok(PathSelection::File) if self.kind.accepts(&path) => {
                Some(PickerResult::Selected(path))
            }
            Ok(PathSelection::File) => {
                self.error = Some(format!(
                    "Choose a {} file",
                    self.kind.extensions().join(", ")
                ));
                None
            }
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }

    pub fn show(&mut self, ctx: &Context) -> Option<PickerResult> {
        if self.finished {
            return None;
        }
        let mut result = None;
        let modal = egui::Modal::new(egui::Id::new("dx-rom-picker")).show(ctx, |ui| {
            ui.set_width(560.0_f32.min((ctx.content_rect().width() - 40.0).max(240.0)));
            ui.horizontal(|ui| {
                ui.heading(self.kind.title());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("×").on_hover_text("Cancel").clicked() {
                        result = Some(PickerResult::Cancelled);
                    }
                });
            });
            ui.label(match self.kind {
                InputKind::Original => {
                    "Choose the original Super Mario Land 2 UE v1.0 ROM (.gb or .gbc)."
                }
                InputKind::Dx => {
                    "Choose a patched SML2DX v1.8.1 ROM (.gb or .gbc), or its .ips patch."
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Up").clicked() {
                    if let Some(parent) = self.directory.parent() {
                        self.navigate(parent.to_owned());
                    }
                }
                if ui.button("Home").clicked() {
                    if let Some(home) = home_directory() {
                        self.navigate(home);
                    } else {
                        self.error = Some("The home directory is unavailable".into());
                    }
                }
                if ui.button("Refresh").clicked() {
                    self.navigate(self.directory.clone());
                }
            });
            ui.label("File or directory path");
            let edited = ui
                .add(egui::TextEdit::singleline(&mut self.path_text).desired_width(f32::INFINITY));
            if edited.changed() {
                self.target = PathBuf::from(&self.path_text);
                self.error = None;
            }
            if ui.button("Open path").clicked()
                || edited.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                result = self.open_target();
            }
            ui.separator();
            ui.label(self.directory.to_string_lossy());
            let mut chosen = None;
            let row_height = ui.text_style_height(&egui::TextStyle::Button) + 6.0;
            egui::ScrollArea::vertical().max_height(280.0).show_rows(
                ui,
                row_height,
                self.entries.len(),
                |ui, rows| {
                    for index in rows {
                        let entry = &self.entries[index];
                        let name = entry.path.file_name().unwrap_or_default().to_string_lossy();
                        let label = if entry.directory {
                            format!("[Folder] {name}")
                        } else {
                            name.into_owned()
                        };
                        let response = ui.selectable_label(self.target == entry.path, label);
                        if response.clicked() {
                            chosen = Some((
                                entry.path.clone(),
                                entry.directory,
                                response.double_clicked(),
                            ));
                        }
                    }
                },
            );
            if let Some((path, directory, double_clicked)) = chosen {
                if directory {
                    self.navigate(path);
                } else {
                    self.set_target(path);
                    if double_clicked {
                        result = self.open_target();
                    }
                }
            }
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            ui.horizontal(|ui| {
                if ui.button("Open file").clicked() {
                    result = self.open_target();
                }
                if ui.button("Cancel").clicked() {
                    result = Some(PickerResult::Cancelled);
                }
            });
        });
        if modal.should_close() {
            result = Some(PickerResult::Cancelled);
        }
        if result.is_some() {
            self.finished = true;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directory_listing_keeps_folders_and_case_insensitive_rom_files() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("z-folder")).unwrap();
        for file in ["a.GBC", "b.gb", "c.txt", "d.gbc.bak"] {
            fs::write(temp.path().join(file), b"").unwrap();
        }
        let entries = directory_entries(temp.path(), InputKind::Original).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].directory);
        assert_eq!(entries[1].path.file_name().unwrap(), "a.GBC");
        assert_eq!(entries[2].path.file_name().unwrap(), "b.gb");
    }

    #[test]
    fn ips_files_are_available_only_for_the_optional_dx_step() {
        let temp = tempfile::tempdir().unwrap();
        let patch = temp.path().join("dx.IPS");
        fs::write(&patch, b"PATCHEOF").unwrap();
        let mut original = FilePicker::for_kind(temp.path().to_owned(), InputKind::Original);
        assert!(original.entries.is_empty());
        original.set_target(patch.clone());
        assert_eq!(original.open_target(), None);
        assert!(original.error.is_some());
        let mut dx = FilePicker::new(temp.path().to_owned());
        assert_eq!(dx.entries[0].path, patch);
        dx.set_target(patch.clone());
        assert_eq!(dx.open_target(), Some(PickerResult::Selected(patch)));
    }

    #[test]
    fn open_path_navigates_directories_and_keeps_missing_file_errors_visible() {
        let temp = tempfile::tempdir().unwrap();
        let child = temp.path().join("child");
        fs::create_dir(&child).unwrap();
        let file = child.join("dx.gbc");
        fs::write(&file, b"").unwrap();
        let mut picker = FilePicker::new(temp.path().to_owned());
        picker.set_target(child.clone());
        assert_eq!(picker.open_target(), None);
        assert_eq!(picker.directory, child);
        picker.set_target(PathBuf::from("missing.gbc"));
        assert_eq!(picker.open_target(), None);
        assert!(picker.error.is_some());
        assert_eq!(picker.directory, child);
        picker.navigate(child.join("missing-directory"));
        assert_eq!(picker.directory, child);
        assert!(picker.error.is_some());
        picker.set_target(file.clone());
        assert_eq!(picker.open_target(), Some(PickerResult::Selected(file)));
    }

    #[cfg(unix)]
    #[test]
    fn clicked_non_utf8_directory_and_file_paths_stay_lossless() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let temp = tempfile::tempdir().unwrap();
        let child = temp.path().join(OsString::from_vec(b"dir\xff".to_vec()));
        fs::create_dir(&child).unwrap();
        let file = child.join(OsString::from_vec(b"dx\xff.GBC".to_vec()));
        fs::write(&file, b"").unwrap();
        let mut picker = FilePicker::new(temp.path().to_owned());
        let path = picker.entries[0].path.clone();
        picker.navigate(path);
        assert_eq!(picker.directory, child);
        picker.set_target(picker.entries[0].path.clone());
        assert_eq!(picker.open_target(), Some(PickerResult::Selected(file)));
    }

    #[cfg(unix)]
    #[test]
    fn non_regular_path_is_not_selected() {
        assert!(classify(Path::new("/dev/null")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn broken_shortcuts_do_not_prevent_browsing_a_folder() {
        let temp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("missing", temp.path().join("broken.gbc")).unwrap();
        let rom = temp.path().join("working.gbc");
        fs::write(&rom, b"").unwrap();
        let entries = directory_entries(temp.path(), InputKind::Original).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, rom);
    }
}
