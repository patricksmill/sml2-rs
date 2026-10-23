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

fn begin_walk(hall: &mut Hall, dir: i8) {
    hall.facing = dir;
    hall.walk = dir * STEP as i8;
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
        // Keep the file-select door's screen anchor throughout the hall.
        // Advance with actual movement, never the next station's destination.
        hall.camera = (hall.mario - DOOR).min(0);
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

/// Pick a permanent sign or its floor position anywhere in the enlarged room.
pub(super) fn pedestal_at(
    pixel: (i32, i32),
    hall: &Hall,
    settings: &Settings,
    debug: bool,
) -> Option<usize> {
    let (mx, my) = sml2_runtime::render::view::margin(settings.view());
    let world = (hall.camera + pixel.0 - mx, pixel.1 - my);
    (0..catalog::HALL_PEDESTALS.len()).find(|&index| {
        let (name, value) = catalog::sign(catalog::HALL_PEDESTALS[index], settings, debug);
        let half_width = name.len().max(value.len()) as i32 * 4;
        let y = sign_y(index);
        let dx = (world.0 - pedestal_x(index)).abs();
        (dx <= half_width && (y..y + 24).contains(&world.1))
            || (dx <= 12 && (104..144).contains(&world.1))
    })
}

fn sign_y(index: usize) -> i32 {
    // Same-height neighbors are 64 px apart, leaving room for six-letter words.
    32 + (index % 2) as i32 * 40
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
            let source = assets.file_select_vram().get(offset..offset + 16)?;
            // Keep the dark stroke, leaving the sprites' light fill transparent.
            Some(std::array::from_fn(|i| source[(i / 2) * 2 + 1]))
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

pub(super) fn stamp_signs<P: Copy>(
    frame: &mut [P],
    hall: &Hall,
    assets: &sml2_core::front_end::FrontEndAssets,
    alphabet: &[u8],
    settings: &Settings,
    debug: bool,
    ink: P,
) {
    let view = settings.view();
    let (mx, my) = sml2_runtime::render::view::margin(view);
    for (index, pedestal) in catalog::HALL_PEDESTALS.iter().enumerate() {
        let (name, value) = catalog::sign(*pedestal, settings, debug);
        let y = sign_y(index) + my;
        for (text, y) in [(&name, y), (&value, y + 16)] {
            stamp_line(
                frame,
                view.0,
                assets,
                alphabet,
                text,
                pedestal_x(index),
                y,
                hall.camera - mx,
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
    let mut frame = sml2_core::front_end::render::render_hall_view_output(
        assets,
        tables,
        settings.view(),
        hall.camera,
        (x, y),
        sprite,
        sml2_core::video::OutputMode::PaletteIndices,
    );
    stamp_signs(&mut frame, hall, assets, alphabet, settings, debug, 3);
    frame
}

#[cfg(test)]
#[path = "hall/tests.rs"]
mod tests;
