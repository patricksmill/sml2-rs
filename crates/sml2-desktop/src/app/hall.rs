//! The brick room to the left of the file select, on its 32-pixel station lattice.

use super::catalog::{self, Pedestal};
use crate::settings::Settings;

pub(super) const STEP: i32 = 0x20;
pub(super) const DOOR: i32 = 0x28;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Hall {
    pub mario: i32,
    pub hop: u8,
    pub goal: Option<usize>,
    pub walk: i8,
    pub walk_timer: u8,
    pub facing: i8,
    pub camera: i32,
}

impl Hall {
    pub(super) fn enter() -> Self {
        Self {
            mario: DOOR,
            hop: 0,
            goal: None,
            walk: 0,
            walk_timer: 0,
            facing: -1,
            camera: 0,
        }
    }
}

pub(super) fn pedestal_x(index: usize) -> i32 {
    DOOR - STEP * (index as i32 + 1)
}

fn furthest() -> i32 {
    pedestal_x(catalog::HALL_PEDESTALS.len() - 1)
}

fn aim_camera(mario: i32, camera: i32, dir: i8) -> i32 {
    let dest = mario + i32::from(dir) * STEP;
    let dest_screen = dest - camera;
    if (8..152).contains(&dest_screen) {
        camera
    } else if dir < 0 {
        dest - 48
    } else {
        (dest - 112).min(0)
    }
}

fn begin_walk(hall: &mut Hall, dir: i8) {
    hall.facing = dir;
    hall.walk = dir * STEP as i8;
    hall.camera = aim_camera(hall.mario, hall.camera, dir);
}

fn land(hall: &mut Hall, settings: &mut Settings, debug: &mut bool) {
    let Some(index) = hall.goal else {
        return;
    };
    if pedestal_x(index) != hall.mario {
        return;
    }
    catalog::cycle(catalog::HALL_PEDESTALS[index], settings, debug);
    hall.hop = 12;
    hall.goal = None;
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
    if hall.walk != 0 {
        let dir = hall.walk.signum();
        hall.mario += i32::from(dir);
        hall.walk -= dir;
        hall.walk_timer = hall.walk_timer.wrapping_add(1);
        if hall.walk_timer >= 0x0C {
            hall.walk_timer = 0;
        }
        if hall.walk == 0 {
            land(hall, settings, debug);
        }
        return false;
    }
    if left || right {
        hall.goal = None;
    }
    if let Some(index) = hall.goal {
        let target = pedestal_x(index);
        if hall.mario == target {
            land(hall, settings, debug);
        } else {
            begin_walk(hall, if hall.mario > target { -1 } else { 1 });
        }
        return false;
    }
    if right && hall.mario == DOOR {
        return true;
    }
    if left && hall.mario > furthest() {
        begin_walk(hall, -1);
    } else if right && hall.mario < DOOR {
        begin_walk(hall, 1);
    } else if jump {
        if let Some(index) =
            (0..catalog::HALL_PEDESTALS.len()).find(|index| hall.mario == pedestal_x(*index))
        {
            catalog::cycle(catalog::HALL_PEDESTALS[index], settings, debug);
            hall.hop = 12;
        }
    }
    false
}

pub(super) fn camera_left(hall: &Hall) -> i32 {
    hall.camera
}

pub(super) fn pedestal_at(native_x: u8, hall: &Hall) -> Option<usize> {
    let world = hall.camera + i32::from(native_x);
    (0..catalog::HALL_PEDESTALS.len()).find(|index| (world - pedestal_x(*index)).abs() <= 12)
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
        OutputMode::PaletteIndices,
    )
    .expect("file select background");
    let mut frame = render_bg_wide_output(
        &native,
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        TileData::Signed,
        (160, 144),
        (-camera_left(hall), 0),
        OutputMode::PaletteIndices,
        file_select_backdrop,
    );
    let cam = camera_left(hall);
    for index in 0..catalog::HALL_PEDESTALS.len() {
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
    fn a_held_direction_marches_one_pipe_and_stops_on_the_lattice() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        assert!(!step(
            &mut hall,
            true,
            false,
            false,
            &mut settings,
            &mut debug
        ));
        assert_eq!(hall.mario, DOOR, "the decision frame does not slide");
        assert_eq!(hall.walk, -STEP as i8);
        for _ in 0..31 {
            assert!(!step(
                &mut hall,
                false,
                false,
                false,
                &mut settings,
                &mut debug
            ));
        }
        assert_ne!(hall.walk, 0, "releasing does not cancel the march");
        assert!(!step(
            &mut hall,
            false,
            false,
            false,
            &mut settings,
            &mut debug
        ));
        assert_eq!(hall.mario, pedestal_x(0));
        assert_eq!(hall.walk, 0);
        let rested = hall.mario;
        assert!(!step(
            &mut hall,
            false,
            false,
            false,
            &mut settings,
            &mut debug
        ));
        assert_eq!(hall.mario, rested, "he stands on the pipe");
        assert!((0..160).contains(&(hall.mario - hall.camera)));
    }

    #[test]
    fn every_march_stays_on_the_native_screen_and_a_click_hops_on_arrival() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        for _ in 0..catalog::HALL_PEDESTALS.len() {
            for _ in 0..33 {
                assert!(!step(
                    &mut hall,
                    true,
                    false,
                    false,
                    &mut settings,
                    &mut debug
                ));
                let screen = hall.mario - hall.camera;
                assert!(
                    (0..160).contains(&screen),
                    "screen x {screen} at mario {} camera {}",
                    hall.mario,
                    hall.camera
                );
            }
        }
        assert_eq!(hall.mario, furthest());
        assert_eq!(hall.walk, 0);
        step(&mut hall, true, false, false, &mut settings, &mut debug);
        assert_eq!(
            hall.mario,
            furthest(),
            "the last pipe does not march further"
        );
        assert_eq!(hall.walk, 0);

        let mut hall = Hall::enter();
        hall.goal = Some(0);
        step(&mut hall, false, false, false, &mut settings, &mut debug);
        for _ in 0..31 {
            step(&mut hall, false, false, false, &mut settings, &mut debug);
        }
        assert_eq!(hall.hop, 0);
        step(&mut hall, false, false, false, &mut settings, &mut debug);
        assert_eq!(hall.mario, pedestal_x(0));
        assert!(hall.hop > 0);
        assert_eq!(settings.zoom, crate::settings::Zoom::Integer(1));
    }

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
            assert!(!step(
                &mut hall,
                true,
                false,
                false,
                &mut settings,
                &mut debug
            ));
        }
        assert_eq!(hall.mario, volume_at);
        assert!(!step(
            &mut hall,
            false,
            false,
            true,
            &mut settings,
            &mut debug
        ));
        assert!((settings.volume - 0.8).abs() < f32::EPSILON);
        assert!(hall.hop > 0);
        let during_hop = settings.volume;
        step(&mut hall, false, false, true, &mut settings, &mut debug);
        assert!(
            (settings.volume - during_hop).abs() < f32::EPSILON,
            "the hop ignores A"
        );
    }

    #[test]
    fn walking_right_through_the_door_leaves() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        assert!(step(
            &mut hall,
            false,
            true,
            false,
            &mut settings,
            &mut debug
        ));
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
        hall.camera = hall.mario - 48;
        let assets = test_assets();
        let frame = pixels(&hall, &assets);
        assert_eq!(frame.len(), 160 * 144);
        let screen_x = (hall.mario - camera_left(&hall)) as usize;
        let mario = frame[100 * 160 + screen_x];
        assert_eq!(mario, 3, "Mario is palette index 3");
        assert_ne!(frame[8 * 160 + 8], 3, "the checker behind him is not Mario");
    }

    #[test]
    fn the_doorway_frame_is_palette_indices() {
        let hall = Hall::enter();
        assert_eq!(hall.mario, DOOR);
        let frame = pixels(&hall, &test_assets());
        let decoded: Vec<_> = frame
            .iter()
            .copied()
            .map(sml2_core::video::PalettePixel::from_token)
            .collect();
        assert_eq!(decoded.len(), frame.len());
    }
}
