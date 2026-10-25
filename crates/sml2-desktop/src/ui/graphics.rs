//! Shared native graphics policy for the game and scenery editor.
use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup};
use std::sync::Arc;

pub fn configuration() -> WgpuConfiguration {
    let kernel = if cfg!(target_os = "linux") {
        std::fs::read_to_string("/proc/sys/kernel/osrelease").ok()
    } else {
        None
    };
    configuration_for_platform(
        kernel.as_deref(),
        std::env::var_os("WGPU_BACKEND").is_some(),
        std::env::var("WGPU_ADAPTER_NAME").ok(),
    )
}

fn configuration_for_platform(
    kernel: Option<&str>,
    backend_override: bool,
    adapter_name: Option<String>,
) -> WgpuConfiguration {
    let mut configuration = WgpuConfiguration::default();
    let WgpuSetup::CreateNew(setup) = &mut configuration.wgpu_setup else {
        return configuration;
    };
    if kernel.is_some_and(|kernel| kernel.to_ascii_lowercase().contains("microsoft"))
        && !backend_override
        && adapter_name.is_none()
    {
        // WSLg's Vulkan surface can lose its advertised formats during resize
        // or fullscreen transitions. egui-wgpu 0.33 panics when configuring
        // that surface. Use the GL presentation path unless explicitly chosen.
        setup.instance_descriptor.backends = wgpu::Backends::GL;
    }
    if let Some(name) = adapter_name {
        // egui-wgpu's default request_adapter path does not read this variable.
        // Honor the documented override and reject unsuitable surfaces before
        // egui tries to configure them.
        setup.native_adapter_selector = Some(Arc::new(move |adapters, surface| {
            let requested = name.to_lowercase();
            adapters
                .iter()
                .find(|adapter| {
                    adapter.get_info().name.to_lowercase().contains(&requested)
                        && surface.is_none_or(|surface| {
                            adapter.is_surface_supported(surface)
                                && surface.get_default_config(adapter, 1, 1).is_some()
                        })
                })
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "No surface-compatible graphics adapter matches WGPU_ADAPTER_NAME={name:?}. \
                         Unset it for automatic selection or choose another WGPU_BACKEND."
                    )
                })
        }));
    }
    configuration
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(configuration: WgpuConfiguration) -> eframe::egui_wgpu::WgpuSetupCreateNew {
        let WgpuSetup::CreateNew(setup) = configuration.wgpu_setup else {
            panic!("expected new graphics setup");
        };
        setup
    }

    #[test]
    fn wsl_defaults_to_gl_for_resize_and_fullscreen_compatibility() {
        for kernel in [
            "6.18.33.2-microsoft-standard-WSL2",
            "5.15.0-Microsoft-standard-WSL2",
        ] {
            let setup = setup(configuration_for_platform(Some(kernel), false, None));
            assert_eq!(setup.instance_descriptor.backends, wgpu::Backends::GL);
            assert!(setup.native_adapter_selector.is_none());
        }
    }

    #[test]
    fn ordinary_platforms_keep_the_default_graphics_selection() {
        let defaults = setup(WgpuConfiguration::default());
        for kernel in [None, Some("6.18.0-1.fc43.x86_64")] {
            let actual = setup(configuration_for_platform(kernel, false, None));
            assert_eq!(
                actual.instance_descriptor.backends,
                defaults.instance_descriptor.backends
            );
            assert!(actual.native_adapter_selector.is_none());
        }
    }

    #[test]
    fn explicit_backend_and_adapter_choices_override_the_wsl_workaround() {
        let defaults = setup(WgpuConfiguration::default());
        for (backend_override, adapter) in [(true, None), (false, Some("llvmpipe".into()))] {
            let named_adapter = adapter.is_some();
            let actual = setup(configuration_for_platform(
                Some("6.18.33.2-microsoft-standard-WSL2"),
                backend_override,
                adapter,
            ));
            assert_eq!(
                actual.instance_descriptor.backends,
                defaults.instance_descriptor.backends
            );
            assert_eq!(actual.native_adapter_selector.is_some(), named_adapter);
        }
    }
}
