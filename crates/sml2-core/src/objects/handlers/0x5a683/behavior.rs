//! ROM `$5A683` (mode B anim 73): a platform that extends, slides, then sinks
//! its sprite once the rider has waited.
//!
//! State 0 arms contact 32 and jumps to state 4, which waits to be stood on.
//! State 1 advances the sprite until 190, then state 2 slides right for 112
//! even frames. State 3 counts 32 frames and then lowers the sprite until 188,
//! clearing contact.
use crate::actor::{Actor, ContactKind, FrameTick};

const EXTENDED: u8 = 0xBE;
const REST: u8 = 0xBC;
const SLIDE: u8 = 112;
const PAUSE: u8 = 32;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Carried,
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(32);
        actor.state = 4;
        return Ok(Effect::Continue);
    }
    match actor.state {
        1 => {
            actor.sprite = actor.sprite.wrapping_add(1);
            if actor.sprite == EXTENDED {
                actor.state = 2;
                actor.set_travel_span(SLIDE);
            }
            Ok(Effect::Continue)
        }
        2 => {
            if !frame.every(2) {
                return Ok(Effect::Continue);
            }
            actor.position.move_x(1);
            let remaining = actor.travel_span().wrapping_sub(1);
            actor.set_travel_span(remaining);
            if remaining == 0 {
                actor.state = 3;
                actor.timer = PAUSE;
            }
            if actor.interactions.stood_on() {
                Ok(Effect::Carried)
            } else {
                Ok(Effect::Continue)
            }
        }
        3 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(Effect::Continue);
            }
            if actor.sprite == REST {
                return Ok(Effect::Continue);
            }
            actor.sprite = actor.sprite.wrapping_sub(1);
            actor.contact_kind = ContactKind::from_raw(0);
            Ok(Effect::Continue)
        }
        4 => {
            if actor.interactions.stood_on() {
                actor.state = 1;
            }
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}
