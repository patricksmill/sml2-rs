//! ROM `$5A258` (mode B anim 63): a platform that drops while ridden, then
//! rises back once Mario steps off.
//!
//! State 0 arms contact 32 and, once stood on, loads a 24-pixel drop into both
//! counters. State 1 sinks one pixel on even frames while ridden, carrying the
//! rider, and switches to state 2 when Mario leaves. State 2 waits 16 frames,
//! then rises until the drop is undone, or returns to state 1 if ridden again.
//! State 3 restores the spawn record through `$5B2BF`.
use crate::actor::{Actor, ContactKind, FrameTick};

const DROP: u8 = 24;
const LEAVE_TIMER: u8 = 16;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Moved { carry_dy: Option<i16> },
    Respawn,
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    let riding = actor.interactions.stood_on();
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            if riding {
                actor.state = 1;
                actor.motion_leg = DROP;
                actor.set_travel_span(DROP);
            }
            Ok(Effect::Continue)
        }
        1 => {
            if !riding {
                actor.state = 2;
                actor.timer = LEAVE_TIMER;
                return Ok(Effect::Continue);
            }
            if !frame.every(2) {
                return Ok(Effect::Continue);
            }
            if actor.motion_leg == 0 {
                actor.state = 3;
                return Ok(Effect::Respawn);
            }
            actor.motion_leg -= 1;
            actor.position.move_y(1);
            Ok(Effect::Moved { carry_dy: Some(1) })
        }
        2 => {
            if riding {
                actor.state = 1;
                actor.timer = 0;
                return Ok(Effect::Continue);
            }
            if actor.timer != 0 {
                actor.timer -= 1;
                if actor.timer != 0 {
                    return Ok(Effect::Continue);
                }
            }
            if actor.motion_leg == actor.travel_span() {
                return Ok(Effect::Continue);
            }
            actor.motion_leg = actor.motion_leg.wrapping_add(1);
            actor.position.move_y(-1);
            Ok(Effect::Moved { carry_dy: None })
        }
        3 => Ok(Effect::Respawn),
        state => Err(Error::InvalidState(state)),
    }
}
