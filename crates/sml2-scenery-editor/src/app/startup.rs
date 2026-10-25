use super::EditorApp;
use crate::settings::{AppPaths, Settings};
use eframe::egui;
use sml2_runtime::content::ExtractedContent;
use std::path::PathBuf;

pub struct Options {
    pub assets: PathBuf,
    pub explicit_assets: bool,
    pub paths: AppPaths,
    pub audio: bool,
    pub smoke_frames: Option<u64>,
}

impl Options {
    pub fn defaults() -> Result<Self, String> {
        Ok(Self {
            assets: ExtractedContent::discover().root().to_owned(),
            explicit_assets: false,
            paths: AppPaths::discover()?,
            audio: true,
            smoke_frames: None,
        })
    }
}

pub fn run(options: Options) -> Result<(), String> {
    let loaded = Settings::load(&options.paths.settings);
    let error = loaded.as_ref().err().cloned();
    let settings = loaded.unwrap_or_default();
    eframe::run_native(
        "SML2 Scenery Editor",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1000.0, 760.0])
                .with_min_inner_size([480.0, 400.0]),
            renderer: eframe::Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(EditorApp::new(
                &cc.egui_ctx,
                options,
                settings,
                error,
            )))
        }),
    )
    .map_err(|e| e.to_string())
}
