//! The brick room to the left of the file select, on its 32-pixel station lattice.

use super::catalog;
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
    if dest == DOOR {
        return 0;
    }
    // Leave room for the six-character station name and choice.
    if (32..128).contains(&dest_screen) {
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

pub(super) fn sprite_frame(hall: &Hall) -> u8 {
    if hall.hop > 0 {
        return 9;
    }
    if hall.walk != 0 {
        let frame = (hall.walk_timer & 0x0C) >> 2;
        return if hall.facing < 0 {
            frame + 5
        } else {
            frame + 1
        };
    }
    if hall.facing < 0 {
        4
    } else {
        0
    }
}

pub(super) fn presentation(hall: &Hall) -> (u8, u8, u8) {
    let x = (hall.mario - camera_left(hall)) as u8;
    let y = 0x7C_u8.wrapping_sub(if hall.hop > 0 { 10 } else { 0 });
    (x, y, sprite_frame(hall))
}

/// The station occupied now, or the destination of the current march.
pub(super) fn sign_station(hall: &Hall) -> Option<usize> {
    let x = hall.mario + i32::from(hall.walk);
    (0..catalog::HALL_PEDESTALS.len()).find(|index| pedestal_x(*index) == x)
}

/// The game's menu/ending alphabet, already loaded for the Easy Mode card.
/// Its letter A is font tile $11 (ASCII minus '0'); the low plane is the
/// colored backdrop and the high plane is the letter stencil.
pub(super) fn alphabet(assets: &sml2_runtime::session::SessionAssets) -> &[u8] {
    assets
        .ending_scene
        .as_ref()
        .and_then(|ending| ending.transfer_source(0x1A, 0x4656, 0x300))
        .unwrap_or(&[])
}

fn glyph(
    assets: &sml2_core::front_end::FrontEndAssets,
    alphabet: &[u8],
    ch: u8,
) -> Option<[u8; 16]> {
    match ch {
        b'A'..=b'Z' => {
            let offset = usize::from(ch - b'0') * 16;
            let source = alphabet.get(offset..offset + 16)?;
            Some(std::array::from_fn(|i| source[(i / 2) * 2 + 1]))
        }
        b'0'..=b'9' => {
            let offset = (0x80 + usize::from(ch - b'0')) * 16;
            assets
                .file_select_vram()
                .get(offset..offset + 16)?
                .try_into()
                .ok()
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn stamp_line<P: Copy>(
    frame: &mut [P],
    stride: usize,
    assets: &sml2_core::front_end::FrontEndAssets,
    alphabet: &[u8],
    text: &str,
    center_world_x: i32,
    world_y: i32,
    camera: i32,
    ink: P,
) {
    let start = center_world_x - text.len() as i32 * 4 - camera;
    for (i, ch) in text.bytes().enumerate() {
        if let Some(bytes) = glyph(assets, alphabet, ch) {
            sml2_core::front_end::stamp_glyph(
                frame,
                stride,
                &bytes,
                (start + i as i32 * 8, world_y),
                ink,
            );
        }
    }
}

pub(super) fn stamp_sign<P: Copy>(
    frame: &mut [P],
    hall: &Hall,
    assets: &sml2_core::front_end::FrontEndAssets,
    alphabet: &[u8],
    settings: &Settings,
    debug: bool,
    ink: P,
) {
    if let Some(index) = sign_station(hall) {
        let (name, value) = catalog::sign(catalog::HALL_PEDESTALS[index], settings, debug);
        for (text, y) in [(&name, 48), (&value, 64)] {
            stamp_line(
                frame,
                160,
                assets,
                alphabet,
                text,
                pedestal_x(index),
                y,
                hall.camera,
                ink,
            );
        }
    }
}

pub(super) fn pixels(
    hall: &Hall,
    assets: &sml2_core::front_end::FrontEndAssets,
    tables: &sml2_core::object::ObjectTables,
    alphabet: &[u8],
    settings: &Settings,
    debug: bool,
) -> Vec<u8> {
    let (x, y, sprite) = presentation(hall);
    let mut frame = sml2_core::front_end::render_hall_output(
        assets,
        tables,
        hall.camera,
        (x, y),
        sprite,
        sml2_core::video::OutputMode::PaletteIndices,
    );
    stamp_sign(&mut frame, hall, assets, alphabet, settings, debug, 3);
    frame
}

#[cfg(test)]
#[path = "hall/tests.rs"]
mod tests;
