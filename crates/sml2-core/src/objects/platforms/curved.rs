//! ROM `$5A355` (mode B anim 67): a curved moving platform.
//!
//! State 0 rises 36 pixels, faces Mario, and arms contact 72. Mario on the
//! right starts at state `$10` with the long horizontal index; otherwise the
//! start is `$12`. Later even frames step Y from the high nibble and X from
//! the low nibble through the `$6519` and `$652F` curves, carrying a stood-on
//! rider by the same amounts.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

/// ROM `$6519`, 22 bytes. Index `$15` is the following byte the CPU would read.
const CURVE_A: [u8; 22] = [
    128, 1, 0, 1, 0, 2, 1, 1, 2, 2, 2, 3, 2, 3, 2, 3, 3, 3, 3, 3, 3, 128,
];
/// ROM `$652F`.
const CURVE_B: [u8; 22] = [
    128, 4, 3, 3, 3, 3, 3, 2, 3, 2, 3, 2, 2, 2, 1, 1, 2, 0, 1, 0, 0, 128,
];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized {
        facing: Ordering,
    },
    Moved {
        carry_dx: Option<i16>,
        carry_dy: Option<i16>,
    },
    Idle,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Effect {
    if actor.state == 0 {
        actor.position.move_y(-36);
        let facing = actor.face_toward(player_x);
        let right = facing == Ordering::Greater;
        actor.state = if right { 0x10 } else { 0x12 };
        actor.motion_leg = 1;
        actor.set_travel_span(if right { 20 } else { 1 });
        actor.contact_kind = ContactKind::from_raw(72);
        return Effect::Initialized { facing };
    }
    if !frame.every(2) {
        return Effect::Idle;
    }
    let dy = step_y(actor);
    let dx = step_x(actor);
    let riding = actor.interactions.stood_on();
    Effect::Moved {
        carry_dx: riding.then_some(dx),
        carry_dy: riding.then_some(dy),
    }
}

fn step_y(actor: &mut Actor) -> i16 {
    let (curve, down, advance) = match actor.state >> 4 {
        1 => (&CURVE_A, true, true),
        2 => (&CURVE_B, true, true),
        3 => (&CURVE_B, false, false),
        _ => (&CURVE_A, false, false),
    };
    let step = sample(curve, actor.motion_leg);
    let dy = if down {
        i16::from(step)
    } else {
        -i16::from(step)
    };
    actor.position.move_y(dy);
    if advance {
        actor.motion_leg = actor.motion_leg.wrapping_add(1);
        if actor.motion_leg == 0x15 {
            actor.state = actor.state.wrapping_add(0x10);
        }
    } else {
        actor.motion_leg = actor.motion_leg.wrapping_sub(1);
        if actor.motion_leg == 0 {
            let mut next = actor.state.wrapping_add(0x10);
            if next >= 0x50 {
                next = next.wrapping_sub(0x40);
            }
            actor.state = next;
        }
    }
    dy
}

fn step_x(actor: &mut Actor) -> i16 {
    let index = actor.travel_span();
    let (curve, right, retreat) = match actor.state & 0x0F {
        0 => (&CURVE_A, true, true),
        1 => (&CURVE_A, false, false),
        2 => (&CURVE_B, false, false),
        _ => (&CURVE_B, true, true),
    };
    let step = sample(curve, index);
    let dx = if right {
        i16::from(step)
    } else {
        -i16::from(step)
    };
    actor.position.move_x(dx);
    if retreat {
        let next = index.wrapping_sub(1);
        actor.set_travel_span(next);
        if next == 0 {
            let mut state = actor.state.wrapping_add(1);
            if state & 0x04 != 0 {
                state = state.wrapping_sub(4);
            }
            actor.state = state;
        }
    } else {
        let next = index.wrapping_add(1);
        actor.set_travel_span(next);
        if next == 0x15 {
            actor.state = actor.state.wrapping_add(1);
        }
    }
    dx
}

fn sample(curve: &[u8; 22], index: u8) -> u8 {
    curve.get(usize::from(index)).copied().unwrap_or(0)
}
