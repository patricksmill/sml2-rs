//! ROM `$5AC7D` (mode B anim 103): a growing block.
//!
//! State 0 arms contact 32 and, on easy, sprite 224. State 1 blinks on even
//! frames and, once stood on, advances to the normal or easy growth. Those
//! states step the sprite every eighth frame, chime at `$CE`/`$E2`, and
//! despawn at `$D0`/`$E4`.
use crate::actor::{Actor, ContactKind, FrameTick};

const NORMAL_CHIME: u8 = 0xCE;
const NORMAL_DONE: u8 = 0xD0;
const EASY_CHIME: u8 = 0xE2;
const EASY_DONE: u8 = 0xE4;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Chime,
    Despawn,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, easy: bool) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(32);
            actor.state = 1;
            if easy {
                actor.sprite = 224;
            }
            Ok(Effect::Continue)
        }
        1 => {
            actor.animate_two_frame_toggle(frame, 2);
            if actor.interactions.stood_on() {
                actor.state = if easy { 3 } else { 2 };
            }
            Ok(Effect::Continue)
        }
        2 => grow(actor, frame, NORMAL_CHIME, NORMAL_DONE),
        3 => grow(actor, frame, EASY_CHIME, EASY_DONE),
        state => Err(Error::InvalidState(state)),
    }
}

fn grow(actor: &mut Actor, frame: FrameTick, chime: u8, done: u8) -> Result<Effect, Error> {
    if !frame.every(8) {
        return Ok(Effect::Continue);
    }
    actor.sprite = actor.sprite.wrapping_add(1);
    if actor.sprite == chime {
        Ok(Effect::Chime)
    } else if actor.sprite == done {
        Ok(Effect::Despawn)
    } else {
        Ok(Effect::Continue)
    }
}
