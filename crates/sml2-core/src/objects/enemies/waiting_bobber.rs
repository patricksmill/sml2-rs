//! ROM `$5AA71` (mode B anims 86 and 87): a vertical bob that waits for Mario.
//!
//! State 0 arms contact 72, state 3, and an 8-step leg in both counters. State
//! 3 waits until Mario is within 32 pixels, sets sprite bit 0, and enters the
//! rise. States 1/2 then move up or down and swap with `state ^= 3` when the
//! leg ends.
use super::pipe_bobber::dist_mario_x;
use crate::actor::{Actor, ContactKind};

const LEG: u8 = 8;
const REACH: u8 = 0x20;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

pub(super) fn update(actor: &mut Actor, player_x: u16) -> Result<(), Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(72);
        actor.state = 3;
        actor.hit_reaction = LEG;
        actor.set_travel_span(LEG);
        return Ok(());
    }
    match actor.state {
        1 => step(actor, -1),
        2 => step(actor, 1),
        3 => {
            if dist_mario_x(player_x, actor.position.x) >= REACH {
                return Ok(());
            }
            actor.sprite |= 1;
            actor.state = 1;
            Ok(())
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn step(actor: &mut Actor, dy: i16) -> Result<(), Error> {
    actor.position.move_y(dy);
    let remaining = actor.hit_reaction.wrapping_sub(1);
    actor.hit_reaction = remaining;
    if remaining == 0 {
        actor.hit_reaction = actor.travel_span();
        actor.state ^= 3;
    }
    Ok(())
}
