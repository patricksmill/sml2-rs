//! ROM `$5AC3C` (mode B anim 102): a thwomp-style dropper.
//!
//! State 0 arms flash 16, contact 72, and state 3. State 3 waits until Mario
//! is within 48 pixels. State 1 bobs on even frames and falls on the `$7488`
//! curve plus one, landing into state 2 with sound 10. State 2 bobs and rises
//! one pixel until a ceiling 32 pixels up returns it to state 3.
use super::pipe_bobber::dist_mario_x;
use super::staging::FALL_SPEED;
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

const DELTAS: [i8; 3] = [-2, 1, 1];
const REACH: u8 = 0x30;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Landed,
    Continue,
}

pub(super) trait World {
    fn grounded(&mut self, actor: &Actor) -> bool;
    fn ceiling(&mut self, actor: &Actor) -> bool;
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    player_x: u16,
    world: &mut impl World,
) -> Result<Effect, Error> {
    match actor.state {
        0 => {
            actor.flash = FlashFlags::from_bits(16);
            actor.contact_kind = ContactKind::from_raw(72);
            actor.state = 3;
            Ok(Effect::Continue)
        }
        1 => {
            bob(actor, frame);
            if world.grounded(actor) {
                actor.motion_phase = 0;
                actor.state = 2;
                return Ok(Effect::Landed);
            }
            let phase = usize::from(actor.motion_phase);
            let step = i16::from(*FALL_SPEED.get(phase).unwrap_or(&0));
            if actor.motion_phase != 21 {
                actor.motion_phase = actor.motion_phase.wrapping_add(1);
            }
            actor.position.move_y(step + 1);
            Ok(Effect::Continue)
        }
        2 => {
            bob(actor, frame);
            if world.ceiling(actor) {
                actor.state = 3;
            } else {
                actor.position.move_y(-1);
            }
            Ok(Effect::Continue)
        }
        3 => {
            if dist_mario_x(player_x, actor.position.x) < REACH {
                actor.state = 1;
            }
            Ok(Effect::Continue)
        }
        state => Err(Error::InvalidState(state)),
    }
}

fn bob(actor: &mut Actor, frame: FrameTick) {
    if !frame.every(2) {
        return;
    }
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    if actor.animation_frame == 3 {
        actor.animation_frame = 0;
    }
    let delta = DELTAS[usize::from(actor.animation_frame)];
    actor.sprite = actor.sprite.wrapping_add_signed(delta);
}
