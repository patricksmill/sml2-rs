use super::{DesktopApp, Screen};
use crate::{
    dx_import::{self, Event, Worker},
    game::Game,
};
use eframe::egui::{Context, Ui};
use sml2_runtime::content::{ExtractedContent, SessionAssets};

impl DesktopApp {
    pub(super) fn restore_dx(&mut self) {
        let Some(key) = self.settings.dx_installation.clone() else {
            return;
        };
        let assets = self.options.assets.clone();
        self.dx_message = "Checking installed DX preview…".into();
        self.dx_worker = Some(Worker::start(move |cancel, _| {
            dx_import::restore(&dx_import::cache_root()?, &assets, key, cancel)
                .map_err(|e| e.to_string())
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

    pub(super) fn dx_controls(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.add_space(12.0);
        ui.strong("DX opening-level preview");
        if self.dx_worker.is_none() {
            if self.menu.button(ui, "Import SML2DX…") {
                let assets = self.options.assets.clone();
                self.dx_message = "Choose the patched SML2DX v1.8.1 ROM…".into();
                self.dx_worker = Some(Worker::start(move |cancel, progress| {
                    let path = rfd::FileDialog::new()
                        .set_title("Import SML2DX v1.8.1")
                        .add_filter("Game Boy Color ROM", &["gbc", "gb"])
                        .pick_file()
                        .ok_or_else(|| "DX import cancelled".to_string())?;
                    dx_import::import_path(
                        &dx_import::cache_root()?,
                        &assets,
                        &path,
                        cancel,
                        progress,
                    )
                    .map_err(|e| e.to_string())
                }));
            }
            if self.dx_bundle.is_some() && self.menu.button(ui, "Play DX preview") {
                self.start_dx(ctx);
            }
        } else {
            ui.spinner();
            if self.menu.button(ui, "Cancel DX import") {
                if let Some(worker) = &self.dx_worker {
                    worker.cancel();
                }
                self.dx_message = "Cancelling DX import…".into();
            }
        }
        if !self.dx_message.is_empty() {
            ui.label(&self.dx_message);
        }
        ui.label("Mario · opening level only · progress saves are isolated");
    }

    pub(super) fn start_dx(&mut self, ctx: &Context) {
        if self.dx_normal.is_some() || self.dx_bundle.is_none() {
            return;
        }
        if self.game.is_none() {
            self.dx_message = "Extracted base assets are required. Supply their directory with --assets, then retry loading game.".into();
            return;
        }
        let result = SessionAssets::load_from(&ExtractedContent::new(&self.options.assets))
            .map_err(|error| format!("DX preview needs extracted base assets (--assets): {error}"))
            .and_then(|assets| Game::preview(assets, self.options.audio));
        match result {
            Ok(preview) => {
                if let Some(game) = &mut self.game {
                    game.audio_running(false);
                }
                self.dx_normal = self.game.replace(preview);
                self.debug = false;
                self.paused = false;
                self.navigate(ctx, Screen::Game);
            }
            Err(error) => self.dx_message = error,
        }
    }

    pub(super) fn finish_dx(&mut self) {
        if let Some(normal) = self.dx_normal.take() {
            self.game = Some(normal);
            self.paused = false;
            self.step = false;
            self.return_screen = Screen::Home;
        }
    }
}
