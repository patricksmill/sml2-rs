//! ROM `$5ADDD` (mode B anim 110): a curved flyer.
//!
//! Every call bobs on every fourth frame (`$5B57E`). State 0 arms contact 72,
//! faces Mario, and sets flash 16. States 1 and 2 follow `$6E62` horizontally
//! and `$6EAC` upward. States 3 and 4 follow `$6E87` horizontally and `$6ED0`
//! downward. A `$80` sample ends the leg, clears the span, and switches state.
//! `$5AEF4` is the same machine with the `$6F55` curves.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};
use std::cmp::Ordering;

const H_CLIMB: &[u8] = &[
    4, 4, 4, 4, 3, 3, 3, 2, 3, 2, 2, 2, 2, 2, 2, 1, 2, 2, 1, 2, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0,
    0, 0, 0, 0, 0, 128,
];
const H_DIVE: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1, 2, 1, 2, 2, 1, 2, 2, 2, 2, 2, 2, 3, 2,
    3, 3, 3, 4, 4, 4, 4, 128,
];
const V_CLIMB: &[u8] = &[
    0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 2, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, 1, 1, 0, 0, 0, 0, 0,
];
const V_DIVE: &[u8] = &[
    0, 0, 0, 0, 0, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 2, 1, 1,
    1, 1, 1, 1, 1, 1, 0,
];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Continue,
}

pub(super) struct Curves {
    pub climb_x: &'static [u8],
    pub climb_y: &'static [u8],
    pub dive_x: &'static [u8],
    pub dive_y: &'static [u8],
}

pub(super) fn anim110() -> Curves {
    Curves {
        climb_x: H_CLIMB,
        climb_y: V_CLIMB,
        dive_x: H_DIVE,
        dive_y: V_DIVE,
    }
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    curves: Curves,
) -> Result<Effect, Error> {
    let _ = actor.animate_four_step_bob_cadence(frame, 4);
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(72);
            let facing = actor.face_toward(player_x);
            actor.flash = FlashFlags::from_bits(16);
            Ok(Effect::Initialized { facing })
        }
        1 => {
            leg(actor, curves.climb_x, curves.climb_y, -1, -1, 3);
            Ok(Effect::Continue)
        }
        2 => {
            leg(actor, curves.climb_x, curves.climb_y, 1, -1, 4);
            Ok(Effect::Continue)
        }
        3 => {
            leg(actor, curves.dive_x, curves.dive_y, 1, 1, 2);
            Ok(Effect::Continue)
        }
        4 => {
            leg(actor, curves.dive_x, curves.dive_y, -1, 1, 1);
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn leg(actor: &mut Actor, horizontal: &[u8], vertical: &[u8], x_sign: i16, y_sign: i16, next: u8) {
    let index = usize::from(actor.travel_span());
    let step = horizontal.get(index).copied().unwrap_or(0x80);
    if step == 0x80 {
        actor.state = next;
        actor.set_travel_span(0);
        return;
    }
    actor.position.move_x(x_sign * i16::from(step));
    let rise = vertical.get(index).copied().unwrap_or(0);
    actor.position.move_y(y_sign * i16::from(rise));
    actor.set_travel_span(actor.travel_span().wrapping_add(1));
}
