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
            wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
                on_surface_error: std::sync::Arc::new(surface_error_action),
                ..Default::default()
            },
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

/// egui-wgpu's default drops the frame on every acquire error and rebuilds the
/// swapchain only when the window size changes. On Linux an out-of-date or lost
/// swapchain must be rebuilt here: a window that starts fullscreen never
/// resizes, so it would drop every frame (issue #1), and each dropped frame
/// leaks the textures egui uploaded before acquiring. Windows reports
/// `Outdated` constantly while minimized at 0×0, where a rebuild at the last
/// size would fail, so it keeps skipping.
fn recreates_surface(error: &eframe::wgpu::SurfaceError, linux: bool) -> bool {
    use eframe::wgpu::SurfaceError;
    linux && matches!(error, SurfaceError::Outdated | SurfaceError::Lost)
}

fn surface_error_action(
    error: eframe::wgpu::SurfaceError,
) -> eframe::egui_wgpu::SurfaceErrorAction {
    use eframe::egui_wgpu::SurfaceErrorAction;
    use eframe::wgpu::SurfaceError;
    use std::sync::atomic::{AtomicU8, Ordering};

    let recreate = recreates_surface(&error, cfg!(target_os = "linux"));
    // Report each kind once: no logger is installed, and this is the only
    // evidence a user can paste from a dropped-frame report.
    static REPORTED: AtomicU8 = AtomicU8::new(0);
    let bit = 1
        << match error {
            SurfaceError::Timeout => 0,
            SurfaceError::Outdated => 1,
            SurfaceError::Lost => 2,
            SurfaceError::OutOfMemory => 3,
            SurfaceError::Other => 4,
        };
    if REPORTED.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
        let action = if recreate {
            "rebuilding the surface"
        } else {
            "skipping the frame"
        };
        eprintln!("note: graphics frame dropped ({error}); {action}");
    }
    if recreate {
        SurfaceErrorAction::RecreateSurface
    } else {
        SurfaceErrorAction::SkipFrame
    }
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
            GetSystemMetrics, ICON_BIG, IMAGE_ICON, LoadImageW, SM_CXICON, SendMessageW, WM_SETICON,
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
    use super::{Options, recreates_surface};
    use eframe::wgpu::SurfaceError;

    #[test]
    fn linux_rebuilds_a_stale_swapchain_instead_of_dropping_frames_forever() {
        for error in [SurfaceError::Outdated, SurfaceError::Lost] {
            assert!(recreates_surface(&error, true));
            // Minimized Windows windows report Outdated at 0x0.
            assert!(!recreates_surface(&error, false));
        }
        for error in [
            SurfaceError::Timeout,
            SurfaceError::OutOfMemory,
            SurfaceError::Other,
        ] {
            assert!(!recreates_surface(&error, true));
        }
    }

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
