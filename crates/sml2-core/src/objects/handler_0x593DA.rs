//! ROM `$593DA` (anim 64): proximity-triggered fall, then a one-pixel rise
//! until the ceiling probe stops it.
//!
//! State 0 arms flash 16 / contact 72 and waits for Mario within 48 px
//! (`$5B4B4`). State 1 falls on the `$5B451` curve plus one, toggling the
//! sprite every frame, and lands into state 2 with phase 0 and sfx 10. State 2
//! rises one pixel per frame, toggling every fourth frame, until `$3B7C`'s
//! 18-px ceiling probe returns it to state 0.
use super::staging::FALL_SPEED;
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick, MotionProfileError};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Motion(MotionProfileError),
}
impl From<MotionProfileError> for Error {
    fn from(error: MotionProfileError) -> Self {
        Self::Motion(error)
    }
}

pub(super) trait World {
    /// `$5B4B4`: `|sMarioRefXLo - objectXLo|` in eight bits.
    fn mario_distance_x(&self, actor_x: u16) -> u8;
    fn probe_ground(&mut self, actor: &Actor) -> bool;
    fn probe_ceiling(&mut self, actor: &Actor) -> bool;
    fn request_sfx1(&mut self, id: u8);
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
) -> Result<(), Error> {
    match actor.state {
        // $593DF: arm the trigger and wait for Mario to come within $30.
        0 => {
            actor.flash = FlashFlags::from_bits(16);
            actor.contact_kind = ContactKind::from_raw(72);
            if world.mario_distance_x(actor.position.x) < 0x30 {
                actor.state = 1;
            }
        }
        // $593F8: fall; land into the rise with the phase reset and sfx 10.
        1 => {
            actor.animate_two_frame_toggle(frame, 1);
            if world.probe_ground(actor) {
                actor.motion_phase = 0;
                actor.state = 2;
                world.request_sfx1(10);
            } else {
                let dy = actor.motion_step(&FALL_SPEED, 21)?;
                actor.position.move_y(i16::from(dy.wrapping_add(1)));
            }
        }
        // $5941A: rise one pixel until the ceiling probe stops the climb.
        2 => {
            actor.animate_two_frame_toggle(frame, 4);
            if world.probe_ceiling(actor) {
                actor.state = 0;
            } else {
                actor.position.move_y(-1);
            }
        }
        state => return Err(Error::InvalidState(state)),
    }
    Ok(())
}
