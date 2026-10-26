//! ROM `$5A85C` (mode B anim 79): a ridden platform that climbs, sinks, climbs
//! again, flashes, and respawns.
//!
//! State 0 arms type 32 and waits for Mario to stand on it (`$FFE7` bit 3).
//! States 1/3 run a 48-step leg up-left; state 2 runs a 32-step leg down-left.
//! Both legs move the platform one pixel on each axis and carry Mario with it
//! (`$3D22`/`$3D2D`/`$3D3D` and the `$3CC0`/`$3CF1` carry pair). State 4 flashes
//! the palette on even frames for 64 frames, then takes the `$5B2BF` reset.
use crate::actor::{Actor, ContactKind, FrameTick};

const CONTACT: u8 = 32;
const UP_LEG: u8 = 0x30;
const DOWN_LEG: u8 = 0x20;
const FLASH_WAIT: u8 = 64;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    /// One platform step; `dy` is also the rider's carry. `dx` is always -1.
    Ride {
        dy: i16,
    },
    Reset,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Result<Effect, Error> {
    match actor.state {
        // $5A861: arm the interaction class, then start once Mario stands on it.
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            if actor.interactions.stood_on() {
                actor.state = 1;
            }
            Ok(Effect::Continue)
        }
        // $5A87B: up-left leg. State 3 re-enters the same body.
        1 | 3 => {
            if !frame.every(2) {
                return Ok(Effect::Continue);
            }
            actor.position.move_y(-1);
            actor.position.move_x(-1);
            actor.set_travel_span(actor.travel_span().wrapping_add(1));
            if actor.travel_span() == UP_LEG {
                actor.set_travel_span(0);
                actor.state = actor.state.wrapping_add(1);
            }
            Ok(Effect::Ride { dy: -1 })
        }
        // $5A89F: down-left leg, then the 64-frame flash wait.
        2 => {
            if !frame.every(2) {
                return Ok(Effect::Continue);
            }
            actor.position.move_y(1);
            actor.position.move_x(-1);
            actor.set_travel_span(actor.travel_span().wrapping_add(1));
            if actor.travel_span() == DOWN_LEG {
                actor.set_travel_span(0);
                actor.state = 3;
                actor.timer = FLASH_WAIT;
            }
            Ok(Effect::Ride { dy: 1 })
        }
        // $5A8C6: blink (`$5B68B`), count down, then `$5A663` -> `$5B2BF`.
        4 => {
            if frame.every(2) {
                actor.flash.toggle_palette();
            }
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            Ok(Effect::Reset)
        }
        state => Err(Error::InvalidState(state)),
    }
}
