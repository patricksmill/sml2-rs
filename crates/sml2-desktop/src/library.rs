#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Rom,
    Dx,
    Packs,
    Editor,
    Play,
    Exit,
}

#[derive(Default, Clone)]
pub struct Library {
    pub vanilla_ready: bool,
    pub checking: bool,
    pub revision_name: String,
    pub rom_error: String,
    pub dx_installed: bool,
    pub dx_mode: bool,
    pub dx_error: String,
    pub importing: bool,
}

impl Library {
    pub fn play_enabled(&self) -> bool {
        self.vanilla_ready && !self.checking && !self.importing
    }

    pub fn play_label(&self) -> &'static str {
        if self.dx_installed && self.dx_mode {
            "Play with DX"
        } else {
            "Play"
        }
    }

    pub fn row_enabled(&self, row: Row) -> bool {
        match row {
            Row::Rom => !self.importing,
            Row::Dx => self.vanilla_ready && !self.checking && !self.importing,
            Row::Packs | Row::Editor => false,
            Row::Play => self.play_enabled(),
            Row::Exit => !self.importing,
        }
    }

    pub fn row_status(&self, row: Row) -> String {
        match row {
            Row::Rom if self.checking => "Checking install…".into(),
            Row::Rom if !self.rom_error.is_empty() => self.rom_error.clone(),
            Row::Rom if self.vanilla_ready => self.revision_name.clone(),
            Row::Rom => "Required".into(),
            Row::Dx if !self.vanilla_ready || self.checking => "Add a ROM first".into(),
            Row::Dx if !self.dx_error.is_empty() => self.dx_error.clone(),
            Row::Dx if !self.dx_installed => "not installed".into(),
            Row::Dx if self.dx_mode => "On".into(),
            Row::Dx => "Off".into(),
            Row::Packs | Row::Editor => "Not in this version".into(),
            Row::Play => self.play_label().into(),
            Row::Exit => "Quit".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Library, Row};

    #[test]
    fn play_stays_off_until_the_vanilla_check_passes() {
        let mut library = Library::default();
        assert!(!library.play_enabled());
        assert_eq!(library.row_status(Row::Rom), "Required");
        assert!(!library.row_enabled(Row::Dx));
        assert_eq!(library.row_status(Row::Dx), "Add a ROM first");
        assert_eq!(library.row_status(Row::Packs), "Not in this version");
        assert!(!library.row_enabled(Row::Packs));
        assert_eq!(library.row_status(Row::Editor), "Not in this version");
        library.checking = true;
        assert_eq!(library.row_status(Row::Rom), "Checking install…");
        assert!(!library.play_enabled());
        library.checking = false;
        library.vanilla_ready = true;
        library.revision_name = "Super Mario Land 2".into();
        assert!(library.play_enabled());
        assert_eq!(library.row_status(Row::Rom), "Super Mario Land 2");
        assert_eq!(library.row_status(Row::Dx), "not installed");
        assert!(library.row_enabled(Row::Dx));
        assert_eq!(library.play_label(), "Play");
        library.dx_installed = true;
        library.dx_mode = true;
        assert_eq!(library.play_label(), "Play with DX");
        library.importing = true;
        assert!(!library.play_enabled());
        assert!(!library.row_enabled(Row::Dx));
        assert!(!library.row_enabled(Row::Rom));
    }
}
