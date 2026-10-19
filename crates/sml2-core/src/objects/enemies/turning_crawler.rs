//! ROM `$59B0E` (anim 90): `$5B678` cadence then the `$58642` crawler body.
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) const CONTACT: u8 = 77;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Crawl,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Effect {
    // $5B678: toggle flip bit 5 every eighth frame (the sprite is untouched).
    if frame.every(8) {
        actor.sprite_flip.toggle_horizontal();
    }
    if actor.state == 0 {
        actor.face_toward(player_x);
        actor.contact_kind = ContactKind::from_raw(CONTACT);
        Effect::Initialized
    } else {
        Effect::Crawl
    }
}
