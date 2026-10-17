//! ROM `$58E7E` (anim 48): a rectangular patrol platform.
//!
//! State 0 rises 32 px (`$3D2D`), faces Mario by X (`$5B3C6`), arms contact 32
//! and sets the leg counters: `+25` (travel span) = 40, `+24` (motion leg) = 80,
//! `+16` (reload) = 80. States 1/2/4/6 move horizontally on the `+25` counter;
//! states 3/5 move vertically on `+24`. The sprite-flip byte selects the next
//! horizontal direction at each corner.
use crate::actor::{Actor, ContactKind};
use std::cmp::Ordering;

/// ROM `$58E8F`: `ld a,40`; doubled into `+24`/`+16` as 80.
const HALF_LEG: u8 = 40;
const FULL_LEG: u8 = 80;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized {
        facing: Ordering,
    },
    Moved {
        carry_dx: Option<i16>,
        carry_dy: Option<i16>,
    },
}

pub(super) fn update(actor: &mut Actor, player_x: u16) -> Result<Effect, Error> {
    match actor.state {
        // $58E83: rise 32, face by X, arm contact and the two leg counters.
        0 => {
            actor.position.move_y(-32);
            let relation = actor.face_toward(player_x);
            actor.contact_kind = ContactKind::from_raw(32);
            actor.set_travel_span(HALF_LEG);
            actor.motion_leg = FULL_LEG;
            actor.motion_phase = FULL_LEG;
            Ok(Effect::Initialized { facing: relation })
        }
        // $58EA7: first horizontal leg, 40 frames.
        1 => {
            actor.position.move_x(-1);
            let carry_dx = actor.interactions.stood_on().then_some(-1);
            if !tick_horizontal(actor) {
                return Ok(Effect::Moved {
                    carry_dx,
                    carry_dy: None,
                });
            }
            actor.state = 3;
            Ok(Effect::Moved {
                carry_dx,
                carry_dy: None,
            })
        }
        // $58EBE: first horizontal leg, the other direction.
        2 => {
            actor.position.move_x(1);
            let carry_dx = actor.interactions.stood_on().then_some(1);
            if !tick_horizontal(actor) {
                return Ok(Effect::Moved {
                    carry_dx,
                    carry_dy: None,
                });
            }
            actor.state = 3;
            Ok(Effect::Moved {
                carry_dx,
                carry_dy: None,
            })
        }
        // $58EC8: vertical leg; the flip byte picks the next horizontal side.
        3 => {
            actor.position.move_y(1);
            let carry_dy = actor.interactions.stood_on().then_some(1);
            if tick_vertical(actor) {
                actor.state = if actor.sprite_flip.bits() == 0 { 4 } else { 6 };
            }
            Ok(Effect::Moved {
                carry_dx: None,
                carry_dy,
            })
        }
        // $58EE9: horizontal leg.
        4 => {
            actor.position.move_x(1);
            let carry_dx = actor.interactions.stood_on().then_some(1);
            if tick_horizontal(actor) {
                actor.state = if actor.sprite_flip.bits() == 0 { 5 } else { 3 };
            }
            Ok(Effect::Moved {
                carry_dx,
                carry_dy: None,
            })
        }
        // $58F0A: vertical leg upward.
        5 => {
            actor.position.move_y(-1);
            let carry_dy = actor.interactions.stood_on().then_some(-1);
            if tick_vertical(actor) {
                actor.state = if actor.sprite_flip.bits() == 0 { 6 } else { 4 };
            }
            Ok(Effect::Moved {
                carry_dx: None,
                carry_dy,
            })
        }
        // $58F2B: horizontal leg, the other direction.
        6 => {
            actor.position.move_x(-1);
            let carry_dx = actor.interactions.stood_on().then_some(-1);
            if tick_horizontal(actor) {
                actor.state = if actor.sprite_flip.bits() == 0 { 3 } else { 5 };
            }
            Ok(Effect::Moved {
                carry_dx,
                carry_dy: None,
            })
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// `$FFE9` countdown: reload from `+16` when it reaches zero.
fn tick_horizontal(actor: &mut Actor) -> bool {
    let next = actor.travel_span().wrapping_sub(1);
    actor.set_travel_span(next);
    if next != 0 {
        return false;
    }
    actor.set_travel_span(actor.motion_phase);
    true
}

/// `$FFE8` countdown: reload from `+16` when it reaches zero.
fn tick_vertical(actor: &mut Actor) -> bool {
    actor.motion_leg = actor.motion_leg.wrapping_sub(1);
    if actor.motion_leg != 0 {
        return false;
    }
    actor.motion_leg = actor.motion_phase;
    true
}
