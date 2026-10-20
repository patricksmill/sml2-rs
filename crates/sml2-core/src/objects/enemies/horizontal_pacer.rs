//! ROM `$5A30E` (mode B anim 65): a horizontal pacer.
//!
//! Every call toggles the sprite on even frames (`$5B521`). State 0 arms
//! contact 72, faces Mario, and loads a 56-step leg with a 112-step reload.
//! States 1/2 then move one pixel on even frames. When the leg ends, the
//! reload is copied back, the state toggles with `xor 3`, and the horizontal
//! flip toggles.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

const FIRST_LEG: u8 = 56;
const RELOAD: u8 = 112;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    actor.animate_two_frame_toggle(frame, 2);
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(72);
        let facing = actor.face_toward(player_x);
        actor.set_travel_span(FIRST_LEG);
        actor.motion_leg = RELOAD;
        return Ok(Effect::Initialized { facing });
    }
    if !frame.every(2) {
        return Ok(Effect::Continue);
    }
    match actor.state {
        1 => step(actor, -1),
        2 => step(actor, 1),
        state => Err(Error::InvalidState(state)),
    }
}

fn step(actor: &mut Actor, dx: i16) -> Result<Effect, Error> {
    actor.position.move_x(dx);
    let remaining = actor.travel_span().wrapping_sub(1);
    actor.set_travel_span(remaining);
    if remaining != 0 {
        return Ok(Effect::Continue);
    }
    actor.set_travel_span(actor.motion_leg);
    actor.state ^= 3;
    actor.sprite_flip.toggle_horizontal();
    Ok(Effect::Continue)
}
