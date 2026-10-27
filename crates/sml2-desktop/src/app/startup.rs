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
    pub launch_library: bool,
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
            launch_library: false,
        })
    }

    pub fn finish_launch(&mut self, _explicit_assets: bool) {
        self.launch_library = false;
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
    let icon = egui::IconData {
        rgba: include_bytes!("../../../../assets/icon.rgba").to_vec(),
        width: 32,
        height: 32,
    };
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size(settings.window_size)
        .with_min_inner_size([480.0, 400.0])
        .with_fullscreen(settings.fullscreen)
        .with_icon(icon)
        // Wayland ignores the icon above; shells match this ID to sml2.desktop.
        .with_app_id("sml2");
    let validation = std::sync::Arc::new(std::sync::Mutex::new(None));
    let result = validation.clone();
    eframe::run_native(
        "Six Golden Coins",
        eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(move |cc| {
            #[cfg(windows)]
            set_taskbar_icon(cc);
            let mut app = DesktopApp::new(&cc.egui_ctx, options, settings, error);
            app.validation_error = validation;
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())?;
    let error = result.lock().map_err(|e| e.to_string())?.take();
    error.map_or(Ok(()), Err)
}

/// winit gives Windows eframe's icon as the small (title bar) icon only; the
/// taskbar and Alt+Tab read the big one, so hand them the full `.ico` the build
/// script embeds as resource 1.
#[cfg(windows)]
fn set_taskbar_icon(cc: &eframe::CreationContext<'_>) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            GetSystemMetrics, LoadImageW, SendMessageW, ICON_BIG, IMAGE_ICON, SM_CXICON,
            WM_SETICON,
        },
    };

    let Ok(handle) = cc.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    // SAFETY: the handle is eframe's live window on this thread, and resource 1
    // is a name-or-ordinal pointer (MAKEINTRESOURCEW) into this module.
    unsafe {
        let size = GetSystemMetrics(SM_CXICON);
        let icon = LoadImageW(
            GetModuleHandleW(std::ptr::null()),
            std::ptr::without_provenance(1),
            IMAGE_ICON,
            size,
            size,
            0,
        );
        if !icon.is_null() {
            SendMessageW(
                handle.hwnd.get() as _,
                WM_SETICON,
                ICON_BIG as usize,
                icon as isize,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn launch_opens_the_in_game_title_directly() {
        let mut options = Options::defaults().unwrap();
        options.finish_launch(false);
        assert!(!options.launch_library);
        options.play = true;
        options.finish_launch(false);
        assert!(!options.launch_library);
        options.play = false;
        options.boot_select = true;
        options.finish_launch(false);
        assert!(!options.launch_library);
        options.boot_select = false;
        options.boot_title = true;
        options.finish_launch(true);
        assert!(!options.launch_library);
    }
}
