//! ROM `$5A9D3` (mode B anim 82): `$5B511` blink, forced flip 0, then the
//! shared `$58642` crawl body. State 0 arms contact 77 and state 1.
use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags};

pub(super) const CONTACT: u8 = 77;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized,
    Crawl,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> Effect {
    // $5B511: toggle sprite bit 0 every eighth frame, before the dispatch.
    actor.animate_two_frame_toggle(frame, 8);
    // $5A9D6: `xor a; ldh [$FFDD],a`.
    actor.sprite_flip = SpriteFlipFlags::default();
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(CONTACT);
        actor.state = 1;
        Effect::Initialized
    } else {
        Effect::Crawl
    }
}
