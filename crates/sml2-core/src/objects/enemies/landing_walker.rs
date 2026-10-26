//! Landing walker family: ROM `$59431` (anim 65) and `$5945A` (anim 66).
//!
//! Both entries toggle the sprite through `$5B511` and send states 1/2 into
//! the shared `$582FB`/`$5831A` patrol body. Every other state runs the inline
//! fall body: `$3B40` ground probe, `$5B451` fall curve, and on landing
//! `$5B3C6` plus the entry's contact (77 for anim 65, 72 for anim 66); anim 66
//! also refreshes flash 16 on every fall frame.
use super::fallwalker;
use super::staging::FALL_SPEED;
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick, MotionProfileError};
use std::cmp::Ordering;

/// The two state-3+ landing bodies' differing immediates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Kind {
    pub(super) contact: u8,
    pub(super) flash_on_fall: bool,
}

/// ROM `$59455`: contact 77, no fall flash (anim 65).
pub(super) const ANIM_65: Kind = Kind {
    contact: 77,
    flash_on_fall: false,
};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    Motion(MotionProfileError),
}
impl From<MotionProfileError> for Error {
    fn from(error: MotionProfileError) -> Self {
        Self::Motion(error)
    }
}

pub(super) trait World: fallwalker::PatrolWorld {
    fn mario_ref_x(&self) -> u16;
    fn set_facing(&mut self, relation: Ordering);
}

pub(super) fn update(
    actor: &mut Actor,
    frame: FrameTick,
    world: &mut impl World,
    kind: Kind,
) -> Result<(), Error> {
    // $5B511: toggle sprite bit 0 every eighth frame, before the dispatch.
    actor.animate_two_frame_toggle(frame, 8);
    match actor.state {
        // $582FB / $5831A: shared patrol, including the odd-frame ledge turn.
        1 | 2 => {
            let _ = fallwalker::patrol(actor, frame, world);
        }
        // $5943E / $59467: the inline fall body. State 0 and every state past
        // 2 fall; landing selects the patrol direction through $5B3C6.
        _ => {
            if kind.flash_on_fall {
                actor.flash = FlashFlags::from_bits(16);
            }
            if world.probe_ground(actor) {
                actor.motion_phase = 0;
                let relation = actor.face_toward(world.mario_ref_x());
                world.set_facing(relation);
                actor.contact_kind = ContactKind::from_raw(kind.contact);
            } else {
                let dy = actor.motion_step(&FALL_SPEED, 21)?;
                actor.position.move_y(i16::from(dy));
            }
        }
    }
    Ok(())
}
