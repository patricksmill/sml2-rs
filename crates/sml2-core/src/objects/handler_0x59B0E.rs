//! ROM `$59B0E` (anim 90): `$5B678` cadence then the `$58642` crawler body.
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 77;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Crawl,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Effect {
    let _ = actor.animate_two_frame_toggle(frame, 4);
    if actor.state == 0 {
        actor.face_toward(player_x);
        actor.contact_kind = ContactKind::from_raw(CONTACT);
        Effect::Initialized
    } else {
        Effect::Crawl
    }
}
