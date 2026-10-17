//! ROM `$59763` (anim 83): `$5B57E` sprite bob, then the shared `$58215`
//! walker loop once `$5B3C6` has picked a direction.
use crate::actor::{Actor, ContactKind, FrameTick};
use std::cmp::Ordering;

pub(super) const CONTACT: u8 = 77;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized { facing: Ordering },
    Motion,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Effect {
    let _ = actor.animate_four_step_bob_cadence(frame, 4);
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(CONTACT);
        let facing = actor.face_toward(player_x);
        return Effect::Initialized { facing };
    }
    Effect::Motion
}
