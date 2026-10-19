//! ROM `$59376` (mode A anim 63): a flashing fuse that respawns as anim 62.
//!
//! State 0 arms flash 16, state 1 and sfx2 1. State 1 falls until the ground
//! probe lands it, then state 2 waits 128 frames. State 3 flashes sprite `^$1B`
//! every fourth frame for 64 frames; on expiry the object is restored to its
//! spawn record as definition 62, faces Mario, and arms the `$59369` tail
//! (contact 82, sprite 112, timer 48).
use super::falling_dropper::step_fall_speed;
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

const RISE_WAIT: u8 = 128;
const FLASH_WAIT: u8 = 64;
const RESPAWN_DEFINITION: u8 = 62;
const TAIL_CONTACT: u8 = 82;
const TAIL_SPRITE: u8 = 112;
const TAIL_TIMER: u8 = 48;
const TAIL_FLASH: u8 = 16;
const TAIL_T24: u8 = 8;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Continue,
    Sfx2,
    Rebirth,
}

pub(super) trait World {
    fn probe_ground(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<Effect, Error> {
    match actor.state {
        // $5937B
        0 => {
            actor.contact_kind = ContactKind::from_raw(0);
            actor.flash = FlashFlags::from_bits(16);
            actor.state = 1;
            Ok(Effect::Sfx2)
        }
        // $59394: fall until the own-Y ground probe lands it.
        1 => {
            if world.probe_ground(actor) {
                actor.state = 2;
                actor.timer = RISE_WAIT;
            } else {
                let speed = step_fall_speed(actor);
                actor.position.move_y(i16::from(speed));
            }
            Ok(Effect::Continue)
        }
        // $593AE: 128-frame wait, then the 64-frame flash phase.
        2 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            actor.state = 3;
            actor.timer = FLASH_WAIT;
            sprite_flash(actor, frame);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            Ok(rebirth(actor))
        }
        // $593BC: flash phase.
        3 => {
            sprite_flash(actor, frame);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer != 0 {
                return Ok(Effect::Continue);
            }
            Ok(rebirth(actor))
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn sprite_flash(actor: &mut Actor, frame: FrameTick) {
    if frame.every(4) {
        actor.sprite ^= 0x1B;
    }
}

/// `$593C5`: definition 62, then the adapter runs `$3AA0` and the `$59369` tail.
fn rebirth(actor: &mut Actor) -> Effect {
    actor.definition = RESPAWN_DEFINITION;
    Effect::Rebirth
}

/// The `$59369` tail written after the spawn reset and `$5B3C6`.
pub(super) fn arm_tail(actor: &mut Actor) {
    actor.contact_kind = ContactKind::from_raw(TAIL_CONTACT);
    actor.sprite = TAIL_SPRITE;
    actor.timer = TAIL_TIMER;
    actor.flash = FlashFlags::from_bits(TAIL_FLASH);
    actor.motion_leg = TAIL_T24;
}
