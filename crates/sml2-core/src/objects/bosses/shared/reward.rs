//! Shared reward morph at ROM `$98F8`, with caller-selected placement.
use crate::actor::{Actor, ContactKind, ScreenPosition};

pub(super) fn morph(actor: &mut Actor, screen: ScreenPosition) {
    actor.screen = screen;
    actor.definition = 4;
    actor.sprite = 9;
    actor.state = 0;
    actor.motion_phase = 0;
    actor.contact_kind = ContactKind::NONE;
}
