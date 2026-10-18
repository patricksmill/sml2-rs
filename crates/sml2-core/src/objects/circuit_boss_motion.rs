//! Movement states 3..31 of ROM `$A372`; table bytes are ROM-addressed below.
use crate::actor::{Actor, ContactKind, SpriteFlipFlags};

const DROP: &[u8] = &[
    0, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 128,
]; // $A472
const DRIFT: &[u8] = &[
    2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0,
]; // $A48C
const ARC: &[u8] = &[3, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0, 128]; // $A4EA
const SLIDE: &[u8] = &[0, 0, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1, 128]; // $A4F7
const FALL: &[u8] = &[0, 1, 1, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 128]; // $A58F
const FALL_X: &[u8] = &[2, 2, 2, 2, 2, 1, 2, 1, 1, 1, 1, 1, 1, 1, 0, 0]; // $A5A0
const SHORT: &[u8] = &[3, 2, 3, 2, 2, 2, 1, 0, 1, 0, 0, 128]; // $A6B7
const SHORT_X: &[u8] = &[0, 1, 1, 1, 2, 2, 2, 2, 2, 2, 1]; // $A6C3
const LAND: &[u8] = &[0, 0, 1, 0, 1, 2, 2, 2, 3, 2, 3, 128]; // $A730
const LAND_X: &[u8] = &[1, 2, 2, 2, 2, 2, 2, 1, 1, 1, 0]; // $A73C

pub(super) fn update(actor: &mut Actor) -> Result<(), ()> {
    match actor.state {
        4 | 6 | 10 | 12 | 14 | 18 | 21 | 23 | 25 | 29 => {
            let state = actor.state;
            let (vertical, horizontal, dx, dy) = match state {
                4 => (DROP, DRIFT, -1, 1),
                6 | 23 => (ARC, SLIDE, -1, -1),
                10 => (FALL, FALL_X, 1, 1),
                12 | 29 => (ARC, SLIDE, 1, -1),
                14 => (SLIDE, ARC, -1, -1),
                18 => (SHORT, SHORT_X, 1, -1),
                21 => (LAND, LAND_X, -1, 1),
                25 => (SLIDE, ARC, 1, -1),
                _ => unreachable!(),
            };
            if path(actor, vertical, horizontal, dx, dy)? {
                match state {
                    4 => actor.set_travel_span(76),
                    6 => actor.motion_leg = 28,
                    10 => actor.set_travel_span(80),
                    12 | 23 => actor.motion_leg = 68,
                    14 | 25 => actor.set_travel_span(28),
                    18 => {
                        actor.set_travel_span(36);
                        actor.defeat_mode = 128;
                    }
                    21 => actor.set_travel_span(84),
                    29 => actor.motion_leg = 60,
                    _ => unreachable!(),
                }
            }
        }
        3 | 9 | 20 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(());
            }
            let dx = if actor.state == 9 { 2 } else { -2 };
            if traverse(actor, dx) {
                actor.defeat_mode = 0;
                actor.state += 1;
            }
        }
        5 | 11 | 22 | 28 => {
            let dx = if matches!(actor.state, 11 | 28) {
                2
            } else {
                -2
            };
            if traverse(actor, dx) {
                actor.state += 1;
                pose(actor, 69);
                actor.contact_kind = ContactKind::from_raw(68);
                if actor.state == 29 {
                    actor.sprite_flip = SpriteFlipFlags::from_bits(32);
                    actor.motion_leg = 96;
                }
            }
        }
        7 | 13 | 24 | 30 => {
            actor.screen.move_y(-1);
            actor.motion_leg = actor.motion_leg.wrapping_sub(1);
            if actor.motion_leg == 0 {
                let old_state = actor.state;
                actor.state += 1;
                pose(actor, 66);
                match old_state {
                    7 => {
                        actor.set_travel_span(36);
                        actor.screen.move_x(4);
                        actor.defeat_mode = 128;
                        // `$A527` falls through: state 8 moves on this same tick.
                        finish_eight(actor);
                    }
                    13 => actor.sprite_flip = SpriteFlipFlags::from_bits(64),
                    24 => actor.sprite_flip = SpriteFlipFlags::from_bits(96),
                    30 => {
                        actor.sprite_flip = SpriteFlipFlags::from_bits(32);
                        actor.set_travel_span(36);
                        actor.screen.move_x(-4);
                        actor.defeat_mode = 128;
                    }
                    _ => unreachable!(),
                }
            }
        }
        8 => finish_eight(actor),
        15 => {
            if traverse(actor, -2) {
                actor.state = 16;
                actor.motion_leg = 96;
                pose(actor, 69);
                actor.contact_kind = ContactKind::from_raw(69);
            }
        }
        16 | 27 => {
            actor.screen.move_y(3);
            actor.motion_leg = actor.motion_leg.wrapping_sub(3);
            if actor.motion_leg == 0 {
                let span = if actor.state == 16 { 28 } else { 56 };
                actor.state += 1;
                actor.set_travel_span(span);
                pose(actor, 66);
                actor.sprite_flip = SpriteFlipFlags::from_bits(32);
            }
        }
        17 => {
            if traverse(actor, 2) {
                actor.state = 18;
            }
        }
        19 => {
            if traverse(actor, 2) {
                actor.state = 20;
                actor.sprite_flip = SpriteFlipFlags::from_bits(0);
                actor.timer = 64;
                actor.set_travel_span(36);
            }
        }
        26 => {
            if traverse(actor, 2) {
                actor.state = 27;
                pose(actor, 69);
                actor.sprite_flip = SpriteFlipFlags::from_bits(64);
                actor.contact_kind = ContactKind::from_raw(69);
                actor.motion_leg = 96;
            }
        }
        31 => {
            if traverse(actor, 2) {
                actor.state = 3;
                actor.sprite_flip = SpriteFlipFlags::from_bits(0);
                actor.contact_kind = ContactKind::from_raw(69);
                actor.set_travel_span(36);
                actor.timer = 64;
            }
        }
        _ => return Err(()),
    }
    Ok(())
}

fn pose(actor: &mut Actor, sprite: u8) {
    actor.sprite = sprite;
    actor.animation_frame = 0;
}
fn traverse(actor: &mut Actor, dx: i16) -> bool {
    actor.screen.move_x(dx);
    actor.set_travel_span(actor.travel_span().wrapping_sub(2));
    actor.travel_span() == 0
}
fn finish_eight(actor: &mut Actor) {
    if traverse(actor, -2) {
        actor.state = 9;
        actor.timer = 64;
        actor.contact_kind = ContactKind::from_raw(69);
        actor.sprite_flip = SpriteFlipFlags::from_bits(32);
        actor.set_travel_span(36);
    }
}
fn path(
    actor: &mut Actor,
    vertical: &[u8],
    horizontal: &[u8],
    dx: i16,
    dy: i16,
) -> Result<bool, ()> {
    let index = usize::from(actor.motion_phase);
    let y = *vertical.get(index).ok_or(())?;
    if y == 128 {
        actor.motion_phase = 0;
        actor.state += 1;
        return Ok(true);
    }
    actor.screen.move_y(i16::from(y) * dy);
    let x = *horizontal.get(index).ok_or(())?;
    actor.screen.move_x(i16::from(x) * dx);
    actor.motion_phase = actor.motion_phase.wrapping_add(1);
    Ok(false)
}

#[cfg(test)]
#[path = "circuit_boss_motion_tests.rs"]
mod tests;
