//! ROM `$5A992` (mode B anim 80): a vertical pacer.
//!
//! Even frames toggle the sprite (`$5B521`). State 0 arms contact 72, state 1,
//! and a 64-step leg stored in both counters. States 1/2 then move down or up
//! on even frames and swap with `state ^= 3` when the leg ends.
use crate::actor::{Actor, ContactKind, FrameTick};

const LEG: u8 = 64;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    actor.animate_two_frame_toggle(frame, 2);
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(72);
        actor.state = 1;
        actor.hit_reaction = LEG;
        actor.set_travel_span(LEG);
        return Ok(Effect::Continue);
    }
    if !frame.every(2) {
        return Ok(Effect::Continue);
    }
    let dy = match actor.state {
        1 => 1,
        2 => -1,
        state => return Err(Error::InvalidState(state)),
    };
    actor.position.move_y(dy);
    let remaining = actor.hit_reaction.wrapping_sub(1);
    actor.hit_reaction = remaining;
    if remaining == 0 {
        actor.hit_reaction = actor.travel_span();
        actor.state ^= 3;
    }
    Ok(Effect::Continue)
}
