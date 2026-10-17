//! ROM `$5AD4D` (mode B anim 107): a pacing platform.
//!
//! State 0 arms contact 32, a 40-pixel span, an 80-pixel reload, flash 16,
//! phase 1, and state 2. States 2 and 4 bob every fourth frame, then alternate
//! one- and two-pixel steps. Easy mode takes those steps only on even frames.
//! A rider is carried with the step. At the end of a leg the platform pauses
//! for 48 frames in state 3 or 1, then reloads the span and turns around.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

const BOB: [i8; 4] = [-3, 1, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Carried { dx: i16 },
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    easy: bool,
) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            actor.state = 2;
            actor.set_travel_span(40);
            actor.hit_reaction = 80;
            actor.flash = FlashFlags::from_bits(16);
            actor.motion_phase = 1;
            Ok(Effect::Continue)
        }
        1 | 3 => pause(actor),
        2 => slide(actor, frame, easy, true),
        4 => slide(actor, frame, easy, false),
        state => Err(Error::InvalidState(state)),
    }
}

fn pause(actor: &mut Actor) -> Result<Effect, Error> {
    actor.timer = actor.timer.wrapping_sub(1);
    if actor.timer != 0 {
        return Ok(Effect::Continue);
    }
    actor.state = actor.state.wrapping_add(1);
    actor.set_travel_span(actor.hit_reaction);
    actor.motion_phase = 0;
    Ok(Effect::Continue)
}

fn slide(actor: &mut Actor, frame: FrameTick, easy: bool, right: bool) -> Result<Effect, Error> {
    let _ = actor.animate_sprite_delta_cycle(frame, 4, &BOB);
    if easy && !frame.every(2) {
        return Ok(Effect::Continue);
    }
    actor.motion_phase ^= 1;
    let step = actor.motion_phase.wrapping_add(1);
    let dx = if right { i16::from(step) } else { -i16::from(step) };
    actor.position.move_x(dx);
    let carried = actor.interactions.stood_on();
    let span = actor.travel_span().wrapping_sub(step);
    actor.set_travel_span(span);
    if span == 0 {
        actor.state = if right { 3 } else { 1 };
        actor.timer = 48;
    }
    if carried {
        Ok(Effect::Carried { dx })
    } else {
        Ok(Effect::Continue)
    }
}
