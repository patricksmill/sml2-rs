use super::DesktopApp;
use crate::settings::{AppPaths, Filter, Settings, Zoom};
use eframe::egui;
use sml2_runtime::content::ExtractedContent;
use std::path::PathBuf;

pub struct Options {
    pub assets: PathBuf,
    pub paths: AppPaths,
    pub audio: bool,
    pub filter: Option<Filter>,
    pub play: bool,
    pub boot_title: bool,
    pub boot_select: bool,
    /// `--room`: stage a bonus room (screens 9-14).
    pub room: Option<u8>,
    /// `--reveal`: stage the six-coin castle reveal.
    pub reveal: bool,
    pub scale: Option<u8>,
    pub fullscreen: bool,
    pub smoke_frames: Option<u64>,
    pub screenshot: Option<PathBuf>,
}
impl Options {
    pub fn defaults() -> Result<Self, String> {
        Ok(Self {
            assets: ExtractedContent::discover().root().to_owned(),
            paths: AppPaths::discover()?,
            audio: true,
            filter: None,
            play: false,
            boot_title: false,
            boot_select: false,
            room: None,
            reveal: false,
            scale: None,
            fullscreen: false,
            smoke_frames: None,
            screenshot: None,
        })
    }
}

pub fn run(options: Options) -> Result<(), String> {
    let loaded = Settings::load(&options.paths.settings);
    let error = loaded.as_ref().err().cloned();
    let mut settings = loaded.unwrap_or_default();
    if let Some(scale) = options.scale {
        settings.zoom = Zoom::Integer(scale);
    }
    if options.fullscreen {
        settings.fullscreen = true;
    }
    if let Some(filter) = options.filter {
        settings.filter = filter;
    }
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size(settings.window_size)
        .with_min_inner_size([480.0, 400.0])
        .with_fullscreen(settings.fullscreen);
    let validation = std::sync::Arc::new(std::sync::Mutex::new(None));
    let result = validation.clone();
    eframe::run_native(
        "Super Mario Land 2",
        eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = DesktopApp::new(&cc.egui_ctx, options, settings, error);
            app.validation_error = validation;
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())?;
    let error = result.lock().map_err(|e| e.to_string())?.take();
    error.map_or(Ok(()), Err)
}
