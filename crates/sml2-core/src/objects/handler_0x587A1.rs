//! Camera-spawned money bag reward, ROM `$587A1` (anim 26).
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) fn update(actor: &mut Actor, frame: FrameTick) {
    actor.flash.flash_palette_on_tick(frame, 2);
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(132);
        actor.position.y = actor.position.y.wrapping_sub(56);
        actor.state = 1;
    } else {
        actor.position.y = actor.position.y.wrapping_add(1);
    }
}
