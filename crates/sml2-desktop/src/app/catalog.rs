//! Settings shared by the hall and the plain window. A cycle is one jump.

use crate::settings::{Settings, Zoom};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pedestal {
    Zoom,
    Fullscreen,
    Filter,
    Volume,
    Mute,
    DeadZone,
    Widescreen,
    CameraZoom,
    Motion,
    DxMode,
    Character,
    Toolbar,
    Debug,
}

pub(super) const PEDESTALS: [Pedestal; 13] = [
    Pedestal::Zoom,
    Pedestal::Fullscreen,
    Pedestal::Filter,
    Pedestal::Volume,
    Pedestal::Mute,
    Pedestal::DeadZone,
    Pedestal::Widescreen,
    Pedestal::CameraZoom,
    Pedestal::Motion,
    Pedestal::DxMode,
    Pedestal::Character,
    Pedestal::Toolbar,
    Pedestal::Debug,
];

/// The hall's pedestals. Character is chosen by the file-select door.
pub(super) const HALL_PEDESTALS: [Pedestal; 12] = [
    Pedestal::Zoom,
    Pedestal::Fullscreen,
    Pedestal::Filter,
    Pedestal::Volume,
    Pedestal::Mute,
    Pedestal::DeadZone,
    Pedestal::Widescreen,
    Pedestal::CameraZoom,
    Pedestal::Motion,
    Pedestal::DxMode,
    Pedestal::Toolbar,
    Pedestal::Debug,
];

pub(super) fn cycle(pedestal: Pedestal, settings: &mut Settings, debug: &mut bool) {
    match pedestal {
        Pedestal::Zoom => {
            let current = match settings.zoom {
                Zoom::Auto => 0,
                Zoom::Integer(n) => n as i32,
            };
            let next = (current + 1) % 9;
            settings.zoom = if next == 0 {
                Zoom::Auto
            } else {
                Zoom::Integer(next as u8)
            };
        }
        Pedestal::Fullscreen => settings.fullscreen = !settings.fullscreen,
        Pedestal::Filter => settings.filter = settings.filter.cycle(1),
        Pedestal::Volume => {
            settings.volume = ((settings.volume * 10.0).round() + 1.0).rem_euclid(11.0) / 10.0;
        }
        Pedestal::Mute => settings.muted = !settings.muted,
        Pedestal::DeadZone => {
            let mut percent = (settings.dead_zone * 100.0).round() as i32 + 5;
            if percent > 80 {
                percent = 10;
            }
            settings.dead_zone = percent as f32 / 100.0;
        }
        Pedestal::Widescreen => settings.widescreen = settings.widescreen.next(),
        Pedestal::CameraZoom => settings.camera_zoom = settings.camera_zoom.next(),
        Pedestal::Motion => settings.motion_smoothing = !settings.motion_smoothing,
        Pedestal::DxMode => settings.dx_mode = !settings.dx_mode,
        Pedestal::Character => {
            settings.character = match settings.character {
                sml2_core::character::Character::Mario => sml2_core::character::Character::Luigi,
                sml2_core::character::Character::Luigi => sml2_core::character::Character::Mario,
            };
        }
        Pedestal::Toolbar => settings.show_toolbar = !settings.show_toolbar,
        Pedestal::Debug => *debug = !*debug,
    }
}

pub(super) fn label(pedestal: Pedestal, settings: &Settings, debug: bool) -> String {
    match pedestal {
        Pedestal::Zoom => format!(
            "Window scale {}",
            match settings.zoom {
                Zoom::Auto => "auto".to_string(),
                Zoom::Integer(n) => format!("{n}×"),
            }
        ),
        Pedestal::Fullscreen => format!("Fullscreen {}", on(settings.fullscreen)),
        Pedestal::Filter => format!("Filter {}", settings.filter.label()),
        Pedestal::Volume => format!("Volume {:.0}%", settings.volume * 100.0),
        Pedestal::Mute => format!("Mute {}", on(settings.muted)),
        Pedestal::DeadZone => format!("Dead zone {:.0}%", settings.dead_zone * 100.0),
        Pedestal::Widescreen => format!("Widescreen {}", settings.widescreen.label()),
        Pedestal::CameraZoom => format!("Camera {}", settings.camera_zoom.label()),
        Pedestal::Motion => format!("Smoothing {}", on(settings.motion_smoothing)),
        Pedestal::DxMode => format!("DX mode {}", on(settings.dx_mode)),
        Pedestal::Character => format!(
            "Character {}",
            match settings.character {
                sml2_core::character::Character::Mario => "Mario",
                sml2_core::character::Character::Luigi => "Luigi",
            }
        ),
        Pedestal::Toolbar => format!("Toolbar {}", on(settings.show_toolbar)),
        Pedestal::Debug => format!("Debug {}", on(debug)),
    }
}

fn on(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

/// Compact labels drawn with the menu alphabet and file-select digits.
pub(super) fn sign(pedestal: Pedestal, settings: &Settings, debug: bool) -> (String, String) {
    use crate::settings::{CameraZoom, Filter, Widescreen};
    let on = |value| if value { "ON" } else { "OFF" }.to_string();
    let (name, value) = match pedestal {
        Pedestal::Zoom => (
            "ZOOM",
            match settings.zoom {
                Zoom::Auto => "AUTO".into(),
                Zoom::Integer(n) => n.to_string(),
            },
        ),
        Pedestal::Fullscreen => ("FULL", on(settings.fullscreen)),
        Pedestal::Filter => (
            "FILTER",
            match settings.filter {
                Filter::Gray => "GRAY".into(),
                Filter::Dmg => "DMG".into(),
                Filter::Pocket => "POCKET".into(),
                Filter::Gbc => "GBC".into(),
                Filter::Amber => "AMBER".into(),
                Filter::Sgb(index) => {
                    format!("SGB {}{}", index / 8 + 1, char::from(b'A' + index % 8))
                }
            },
        ),
        Pedestal::Volume => ("VOL", format!("{:.0}", settings.volume * 100.0)),
        Pedestal::Mute => ("MUTE", on(settings.muted)),
        Pedestal::DeadZone => ("DEAD", format!("{:.0}", settings.dead_zone * 100.0)),
        Pedestal::Widescreen => (
            "WIDE",
            match settings.widescreen {
                Widescreen::Off => "OFF",
                Widescreen::Wide16x9 => "169",
            }
            .into(),
        ),
        Pedestal::CameraZoom => (
            "CAM",
            match settings.camera_zoom {
                CameraZoom::Native => "100",
                CameraZoom::Zoom125 => "125",
                CameraZoom::Zoom150 => "150",
                CameraZoom::Zoom200 => "200",
            }
            .into(),
        ),
        Pedestal::Motion => ("SMOOTH", on(settings.motion_smoothing)),
        Pedestal::DxMode => ("DX", on(settings.dx_mode)),
        Pedestal::Character => (
            "CHAR",
            match settings.character {
                sml2_core::character::Character::Mario => "MARIO",
                sml2_core::character::Character::Luigi => "LUIGI",
            }
            .into(),
        ),
        Pedestal::Toolbar => ("BAR", on(settings.show_toolbar)),
        Pedestal::Debug => ("DEBUG", on(debug)),
    };
    (name.into(), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_jump_cycles_volume_mute_and_the_toolbar() {
        let mut settings = Settings::default();
        let mut debug = false;
        assert!((settings.volume - 0.7).abs() < f32::EPSILON);
        cycle(Pedestal::Volume, &mut settings, &mut debug);
        assert!((settings.volume - 0.8).abs() < f32::EPSILON);
        for _ in 0..4 {
            cycle(Pedestal::Volume, &mut settings, &mut debug);
        }
        assert!(
            (settings.volume - 0.1).abs() < f32::EPSILON,
            "100% wraps to 0%"
        );
        cycle(Pedestal::Mute, &mut settings, &mut debug);
        assert!(settings.muted);
        cycle(Pedestal::Toolbar, &mut settings, &mut debug);
        assert!(!settings.show_toolbar);
        cycle(Pedestal::Debug, &mut settings, &mut debug);
        assert!(debug);
        assert_eq!(label(Pedestal::Mute, &settings, debug), "Mute on");
    }
}
