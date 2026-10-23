//! The room to the left of the file select. World 0 is the doorway.

use super::catalog::{self, Pedestal};
use crate::settings::Settings;

pub(super) const GAP: i32 = 48;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Hall {
    pub mario: i32,
    pub hop: u8,
    pub goal: Option<usize>,
}

impl Hall {
    pub(super) fn enter() -> Self {
        Self {
            mario: 0,
            hop: 0,
            goal: None,
        }
    }
}

pub(super) fn pedestal_x(index: usize) -> i32 {
    -GAP * (index as i32 + 1)
}

pub(super) fn step(
    hall: &mut Hall,
    left: bool,
    right: bool,
    jump: bool,
    settings: &mut Settings,
    debug: &mut bool,
) -> bool {
    if hall.hop > 0 {
        hall.hop -= 1;
        return false;
    }
    if let Some(index) = hall.goal {
        let target = pedestal_x(index);
        if hall.mario < target {
            hall.mario += 1;
        } else if hall.mario > target {
            hall.mario -= 1;
        } else {
            catalog::cycle(catalog::PEDESTALS[index], settings, debug);
            hall.hop = 12;
            hall.goal = None;
        }
        return false;
    }
    if left {
        hall.mario -= 1;
    } else if right {
        hall.mario += 1;
    }
    if hall.mario > 0 {
        return true;
    }
    if jump {
        if let Some(index) = (0..catalog::PEDESTALS.len()).find(|index| {
            (hall.mario - pedestal_x(*index)).abs() <= 8
        }) {
            catalog::cycle(catalog::PEDESTALS[index], settings, debug);
            hall.hop = 12;
        }
    }
    false
}

pub(super) fn camera_left(mario: i32) -> i32 {
    (mario - 100).min(-20)
}

pub(super) fn pedestal_at(native_x: u8, mario: i32) -> Option<usize> {
    let world = camera_left(mario) + native_x as i32;
    (0..catalog::PEDESTALS.len()).find(|index| (world - pedestal_x(*index)).abs() <= 12)
}

pub(super) fn pixels(hall: &Hall, assets: &sml2_core::front_end::FrontEndAssets) -> Vec<u8> {
    use sml2_core::front_end::file_select::FILE_SELECT_BGP;
    use sml2_core::front_end::screen::{
        file_select_backdrop, render_bg_output, render_bg_wide_output, TileData,
    };
    use sml2_core::video::OutputMode;
    let native = render_bg_output(
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        (0, 0),
        (160, 144),
        TileData::Signed,
        OutputMode::Grayscale,
    )
    .expect("file select background");
    let mut frame = render_bg_wide_output(
        &native,
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        TileData::Signed,
        (160, 144),
        (-camera_left(hall.mario), 0),
        OutputMode::PaletteIndices,
        file_select_backdrop,
    );
    let cam = camera_left(hall.mario);
    for index in 0..catalog::PEDESTALS.len() {
        fill(&mut frame, pedestal_x(index) - cam - 8, 120, 16, 8, 2);
    }
    let hop = if hall.hop > 0 { 10 } else { 0 };
    fill(&mut frame, hall.mario - cam - 4, 100 - hop, 8, 16, 3);
    frame
}

fn fill(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, shade: u8) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if (0..160).contains(&px) && (0..144).contains(&py) {
                frame[(py * 160 + px) as usize] = shade;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walking_left_reaches_volume_and_a_jump_raises_it() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        let volume_at = pedestal_x(
            catalog::PEDESTALS
                .iter()
                .position(|p| *p == Pedestal::Volume)
                .unwrap(),
        );
        for _ in 0..200 {
            if hall.mario == volume_at {
                break;
            }
            assert!(!step(&mut hall, true, false, false, &mut settings, &mut debug));
        }
        assert_eq!(hall.mario, volume_at);
        assert!(!step(&mut hall, false, false, true, &mut settings, &mut debug));
        assert!((settings.volume - 0.8).abs() < f32::EPSILON);
        assert!(hall.hop > 0);
        let during_hop = settings.volume;
        step(&mut hall, false, false, true, &mut settings, &mut debug);
        assert!((settings.volume - during_hop).abs() < f32::EPSILON, "the hop ignores A");
    }

    #[test]
    fn walking_right_through_the_door_leaves() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        assert!(step(&mut hall, false, true, false, &mut settings, &mut debug));
    }

    fn test_assets() -> sml2_core::front_end::FrontEndAssets {
        use sml2_core::front_end::{DEMO_TAPES_LEN, TILEMAP_LEN, VRAM_LEN};
        sml2_core::front_end::FrontEndAssets::new(
            vec![0u8; VRAM_LEN],
            vec![0u8; TILEMAP_LEN],
            vec![0u8; VRAM_LEN],
            vec![0u8; TILEMAP_LEN],
            vec![0u8; DEMO_TAPES_LEN],
        )
        .unwrap()
    }

    #[test]
    fn the_picture_puts_mario_on_the_volume_pedestal() {
        let mut hall = Hall::enter();
        hall.mario = pedestal_x(3);
        let assets = test_assets();
        let frame = pixels(&hall, &assets);
        assert_eq!(frame.len(), 160 * 144);
        let screen_x = (hall.mario - camera_left(hall.mario)) as usize;
        let mario = frame[100 * 160 + screen_x];
        assert_eq!(mario, 3, "Mario is palette index 3");
        assert_ne!(frame[8 * 160 + 8], 3, "the checker behind him is not Mario");
    }
}
