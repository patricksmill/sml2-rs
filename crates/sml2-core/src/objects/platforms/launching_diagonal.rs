//! ROM `$5A5EC` (mode B anim 71): a diagonal platform that launches when ridden.
//!
//! State 0 arms flash 16 and contact 32, and advances once Mario stands on it.
//! States 1 and 3 rise one pixel and move right on even frames for 48 steps.
//! State 2 falls and moves right for 32 steps, then waits 64 frames in state 4
//! before `$5B2BF` restores the spawn record. A stood-on rider is carried.
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

const CLIMB: u8 = 0x30;
const DROP: u8 = 0x20;
const PAUSE: u8 = 64;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Moved { dx: i16, dy: i16, riding: bool },
    Respawn,
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.flash = FlashFlags::from_bits(16);
            actor.contact_kind = ContactKind::from_raw(32);
            if actor.interactions.stood_on() {
                actor.state = 1;
            }
            Ok(Effect::Continue)
        }
        1 | 3 => diagonal(actor, frame, -1, CLIMB, true),
        2 => diagonal(actor, frame, 1, DROP, false),
        4 => {
            if frame.every(2) {
                actor.flash.toggle_palette(frame);
            }
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                Ok(Effect::Respawn)
            } else {
                Ok(Effect::Continue)
            }
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn diagonal(
    actor: &mut Actor,
    frame: FrameTick,
    dy: i16,
    limit: u8,
    climb: bool,
) -> Result<Effect, Error> {
    if !frame.every(2) {
        return Ok(Effect::Continue);
    }
    actor.position.move_y(dy);
    actor.position.move_x(1);
    let span = actor.travel_span().wrapping_add(1);
    actor.set_travel_span(span);
    if span == limit {
        actor.set_travel_span(0);
        if climb {
            actor.state = actor.state.wrapping_add(1);
        } else {
            actor.state = 3;
            actor.timer = PAUSE;
        }
    }
    Ok(Effect::Moved {
        dx: 1,
        dy,
        riding: actor.interactions.stood_on(),
    })
}
