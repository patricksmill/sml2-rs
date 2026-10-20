//! Wario's Castle decorations ($3DFE8-$3E1CA) and sky raster bands ($3E7F2).
//!
//! Screen 7, loaded by `$3D6EB` from the `$3BD80` castle tilemap. The
//! decomp's `sTurtle*` names for this state predate that identification.

use super::super::MapSim;

const STEP_FRAMES: [u8; 4] = [0, 1, 2, 1];
const BLINK_FRAMES: [u8; 4] = [0, 1, 0, 2];
const LIGHTNING_X: [u8; 4] = [0x30, 0x70, 0x60, 0x40];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WarioCastleAnimation {
    pub(super) anim_count: u8,
    pub(super) phase: u8,
    pub(super) flash: u8,
    pub(super) accum: u8,
    pub(super) movement: u8,
    pub(super) blink_timer: u8,
    pub(super) raster_phase: u8,
    pub(super) band_scroll: [u8; 3],
    pub(super) bg_palette: u8,
    pub(super) obj_palette: u8,
}

impl Default for WarioCastleAnimation {
    fn default() -> Self {
        Self {
            anim_count: 0,
            phase: 0,
            flash: 0,
            accum: 0,
            movement: 0,
            blink_timer: 1,
            raster_phase: 0,
            band_scroll: [0, 23, 0],
            bg_palette: 0xE1,
            obj_palette: 0x39,
        }
    }
}

/// The zone uses stride-three records at the addresses named for the hub's
/// stride-four sprites. Keep that aliasing at the conversion boundary.
///
/// Slot 1 is the flag on the keep (its id cycles 53-55); slot 2 the lightning
/// bolt; slot 3 the wanderer, which paces the castle front and drops out of
/// sight at the gate (x `$40-$5F`, y >= `$50`).
struct Decorations {
    flag_frame: u8,
    lightning_x: u8,
    lightning_frame: u8,
    wanderer: (u8, u8, u8),
}

impl Decorations {
    fn read(map: &MapSim) -> Self {
        Self {
            flag_frame: map.zone_slots[1].2,
            lightning_x: map.zone_slots[2].0,
            lightning_frame: map.zone_slots[2].2,
            wanderer: map.zone_slots[3],
        }
    }

    fn write(self, map: &mut MapSim) {
        map.zone_slots[1].2 = self.flag_frame;
        map.zone_slots[2].0 = self.lightning_x;
        map.zone_slots[2].2 = self.lightning_frame;
        map.zone_slots[3] = self.wanderer;
    }
}

pub(super) fn reset(map: &mut MapSim) {
    // $3D6EB resets these fields, leaving the other animation counters intact.
    let state = &mut map.zone_animation.wario_castle;
    state.movement = 0;
    state.blink_timer = 1;
    state.band_scroll[1] = 23;
    state.bg_palette = 0xE1;
    state.obj_palette = 0x39;
}

pub(super) fn tick(map: &mut MapSim, div: u8) {
    let mut decor = Decorations::read(map);
    let frame = map.frame_counter;
    let state = &mut map.zone_animation.wario_castle;
    update_bands(state);
    if frame & 7 == 0 {
        state.anim_count = state.anim_count.wrapping_add(1);
        if state.anim_count >= 3 {
            state.anim_count = 0;
        }
        decor.flag_frame = state.anim_count.wrapping_add(53);
    }
    if frame & 3 == 0 {
        decor.lightning_frame = 0xFF;
        if div & 0x7E == 0 {
            decor.lightning_frame = 56;
            state.phase = state.phase.wrapping_add(1) & 3;
            decor.lightning_x = LIGHTNING_X[state.phase as usize];
        }
        if map.zone_animation.transition_state == 0 {
            let inverse = match state.flash {
                1 => {
                    state.flash = 2;
                    false
                }
                2 => {
                    state.flash = 0;
                    true
                }
                _ => {
                    let lightning = decor.lightning_frame == 56;
                    if lightning {
                        state.flash = 1;
                    }
                    lightning
                }
            };
            state.bg_palette = if inverse { 0x1E } else { 0xE1 };
            state.obj_palette = if inverse { 0xC6 } else { 0x39 };
        }
    }
    if frame & 1 == 0 {
        state.accum = state.accum.wrapping_add(1);
        animate_wanderer(
            state,
            &mut decor.wanderer,
            div,
            map.zone_animation.sampled_timer,
        );
    }
    decor.write(map);
}

fn update_bands(state: &mut WarioCastleAnimation) {
    state.raster_phase = state.raster_phase.wrapping_add(1);
    if state.raster_phase & 3 != 0 {
        return;
    }
    state.band_scroll[0] = state.band_scroll[0].wrapping_sub(1);
    if state.raster_phase & 4 != 0 {
        return;
    }
    state.band_scroll[1] = state.band_scroll[1].wrapping_sub(1);
    if state.band_scroll[1] >= 24 {
        state.band_scroll[1] = 23;
    }
    if state.raster_phase & 8 == 0 {
        state.band_scroll[2] = state.band_scroll[2].wrapping_sub(1);
    }
}

fn animate_wanderer(state: &mut WarioCastleAnimation, wanderer: &mut (u8, u8, u8), div: u8, timer: u8) {
    let (x, y, sprite) = wanderer;
    match state.movement {
        1 | 2 => {
            *sprite =
                STEP_FRAMES[(state.accum & 3) as usize] + if state.movement == 1 { 57 } else { 60 };
            if (state.movement == 1 && *x < 0x28) || (state.movement == 2 && *x >= 0x78) {
                choose_movement(state, div, timer);
                return;
            }
            let step = state.accum & 1;
            *x = if state.movement == 1 {
                x.wrapping_sub(step)
            } else {
                x.wrapping_add(step)
            };
            if (0x40..0x60).contains(x) && *y >= 0x50 {
                *sprite = 0xFF;
            }
        }
        3 | 4 => {
            *sprite = 63;
            if (state.movement == 3 && *y < 0x36) || (state.movement == 4 && *y >= 0x62) {
                state.movement = if *x < 0x32 { 2 } else { 1 };
                if state.movement == 2 {
                    state.blink_timer = 30;
                }
                return;
            }
            *y = if state.movement == 3 {
                y.wrapping_sub(4)
            } else {
                y.wrapping_add(4)
            };
            *sprite = 0xFF;
        }
        _ => {
            *sprite = 63 + BLINK_FRAMES[((state.accum & 0x0C) >> 2) as usize];
            if state.blink_timer != 0 {
                state.blink_timer = state.blink_timer.wrapping_sub(1);
            } else {
                choose_movement(state, div, timer);
            }
        }
    }
}

fn choose_movement(state: &mut WarioCastleAnimation, div: u8, timer: u8) {
    // The ROM retries DIV+TIMA while hardware time advances. Sampling successive
    // DIV bytes with fixed TIMA preserves rejection rules without a native spin.
    // Exact random phase depends on CPU timing, which the native map omits.
    let mut sample = div.wrapping_add(timer);
    loop {
        let candidate = sample & 7;
        sample = sample.wrapping_add(1);
        if candidate == state.movement {
            continue;
        }
        let direction = if candidate >= 5 {
            candidate - 4
        } else {
            candidate
        };
        if direction != 0 && (direction < 3) == (state.movement < 3) {
            continue;
        }
        state.movement = direction;
        state.blink_timer = 30;
        return;
    }
}

#[cfg(test)]
#[path = "animation_wario_castle_tests.rs"]
mod tests;
