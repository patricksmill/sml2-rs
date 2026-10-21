//! Space scenery, ROM $0F:$5B40-$5D8B. The disassembly's four-byte
//! sprite aliases address the zone loader's three-byte decoration records.
use super::super::MapSim;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SpaceAnimation {
    pub(super) blink_timer: u8,
    pub(super) launch_state: u8,
    pub(super) launch_timer: u8,
}

const BLINK: [u8; 4] = [0x49, 0x4A, 0x4B, 0x49];
const DRIFTER: [u8; 4] = [0x46, 0x47, 0x48, 0x47];
const STARS: [[u8; 4]; 3] = [
    [0x43, 0x44, 0x45, 0xFF],
    [0xFF, 0x43, 0x44, 0x45],
    [0x45, 0xFF, 0x43, 0x44],
];
const STAR_POSITIONS: [(u8, u8); 16] = [
    (0x20, 0x80),
    (0x18, 0x48),
    (0x28, 0x68),
    (0x60, 0x30),
    (0x70, 0x3A),
    (0x90, 0x80),
    (0x98, 0x64),
    (0x90, 0x14),
    (0x10, 0x10),
    (0x40, 0x14),
    (0x14, 0x8A),
    (0x9C, 0x28),
    (0x9C, 0x90),
    (0x12, 0x78),
    (0x48, 0x46),
    (0x40, 0x56),
];

#[derive(Clone, Copy)]
struct Sprite {
    x: u8,
    y: u8,
    frame: u8,
}
impl From<(u8, u8, u8)> for Sprite {
    fn from((x, y, frame): (u8, u8, u8)) -> Self {
        Self { x, y, frame }
    }
}
impl From<Sprite> for (u8, u8, u8) {
    fn from(s: Sprite) -> Self {
        (s.x, s.y, s.frame)
    }
}

pub(super) fn reset(map: &mut MapSim) {
    map.zone_animation.space.blink_timer = 0;
    map.zone_animation.space.launch_state = 0;
    // $3D58D: the first Space path has already launched the UFO.
    if map.level_progress[16] & 1 != 0 {
        map.zone_animation.space.launch_state = 3;
        map.zone_animation.space.launch_timer = 0;
        map.zone_slots[7].2 = 112;
        map.zone_slots[9].2 = 123;
    }
}

/// $3D533: first clear of Space 1 starts the launch before its path bit is set.
pub(super) fn start_launch(map: &mut MapSim) {
    map.zone_animation.space.launch_state = 1;
    map.zone_animation.space.launch_timer = 60;
    map.zone_slots[9].2 = 117;
}

pub(super) fn tick(map: &mut MapSim, div: u8) {
    let frame = map.frame_counter;
    let phase = ((frame & 0x18) >> 3) as usize;
    let mut blinker = Sprite::from(map.zone_slots[0]);
    let mut stars = [
        Sprite::from(map.zone_slots[1]),
        Sprite::from(map.zone_slots[2]),
        Sprite::from(map.zone_slots[3]),
    ];
    let mut drifter = Sprite::from(map.zone_slots[6]);
    // `$3DB40`'s blink roll is this screen's first `rDIV` read after `$3C13E`.
    let blink_div =
        (frame & 3 == 0 && map.zone_animation.space.blink_timer == 0).then(|| map.next_div(div));
    let state = &mut map.zone_animation;
    if frame & 3 == 0 {
        if state.space.blink_timer == 0 {
            let div = blink_div.unwrap_or(div);
            state.random_accum = state
                .random_accum
                .wrapping_add(div.rotate_left(4))
                .wrapping_add(div)
                .wrapping_add(state.global_frame);
            if state.random_accum & 7 == 0 {
                state.space.blink_timer = 5;
            }
        }
        if state.space.blink_timer != 0 {
            state.space.blink_timer -= 1;
            blinker.frame = BLINK[(state.space.blink_timer.wrapping_sub(1) & 3) as usize];
        }
    }
    drifter.frame = DRIFTER[phase];
    drifter.x = drifter.x.wrapping_sub(1);
    if frame & 3 == 0 {
        drifter.y = drifter.y.wrapping_add(1);
        if drifter.y == 0 {
            drifter.x = div;
        }
    }
    if frame & 7 == 0 {
        for (star, sequence) in stars.iter_mut().zip(STARS) {
            star.frame = sequence[phase];
        }
        // Three stars respawn at staggered phases; phase 3 only animates.
        if phase < 3 {
            let (x, y) = STAR_POSITIONS[((div & 0x1E) >> 1) as usize];
            stars[phase].x = x;
            stars[phase].y = y;
        }
    }
    map.zone_slots[0] = blinker.into();
    for (slot, star) in map.zone_slots[1..4].iter_mut().zip(stars) {
        *slot = star.into();
    }
    map.zone_slots[6] = drifter.into();
    tick_launch(map);
    // $3DC3D renders before this path-marker byte advances.
    if map.zone_slots[9].2 < 123 && frame & 7 == 0 {
        map.zone_animation.render_slots = Some(map.zone_slots);
        map.zone_slots[9].2 += 1;
        map.zone_animation.sfx0 = 6;
    }
}

fn tick_launch(map: &mut MapSim) {
    let frame = map.frame_counter;
    let mut ufo = Sprite::from(map.zone_slots[7]);
    let mut exhaust = Sprite::from(map.zone_slots[8]);
    let state = &mut map.zone_animation;
    match state.space.launch_state {
        0 => {
            ufo.frame = 255;
            exhaust.frame = 255;
        }
        1 if frame & 1 != 0 => {
            ufo.frame = 111;
            exhaust.frame = 113 + ((frame & 0x0C) >> 2);
            ufo.x = ufo.x.wrapping_add(3);
            exhaust.x = ufo.x;
            ufo.y = ufo.y.wrapping_sub(3);
            exhaust.y = ufo.y;
            state.space.launch_timer = state.space.launch_timer.wrapping_sub(1);
            if state.space.launch_timer == 0 {
                state.space.launch_state = 2;
                ufo.x = 168;
                ufo.y = 14;
                exhaust.frame = 255;
                state.space.launch_timer = 10;
            }
        }
        1 => {}
        2 => {
            ufo.x = ufo.x.wrapping_sub(2);
            ufo.y = ufo.y.wrapping_add(2);
            ufo.frame = 111;
            state.space.launch_timer = state.space.launch_timer.wrapping_sub(1);
            if state.space.launch_timer == 0 {
                state.space.launch_state = 3;
                ufo.x = 156;
                ufo.y = 36;
                ufo.frame = 112;
                exhaust.frame = 255;
                state.patch_ids[0] = 56;
                state.patch_ids[1] = 52;
                state.space.launch_timer = 20;
                state.sfx0 = 24;
            }
        }
        _ => {
            ufo = Sprite {
                x: 140,
                y: 36,
                frame: 255,
            };
            exhaust.frame = 255;
            state.scroll_x = 0;
            state.scroll_y = 0;
            if state.space.launch_timer != 0 {
                state.space.launch_timer -= 1;
                if frame & 2 != 0 {
                    state.scroll_x = state.space.launch_timer >> 1;
                    state.scroll_y = !state.scroll_x;
                    if state.space.launch_timer == 0 {
                        state.sfx0 = 21;
                    }
                }
            }
        }
    }
    map.zone_slots[7] = ufo.into();
    map.zone_slots[8] = exhaust.into();
}

#[cfg(test)]
#[path = "animation_space_tests.rs"]
mod tests;
