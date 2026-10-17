//! ROM `$586E6` (mode B anim 31): arm contact 137 and return.
//!
//! The whole body is `ld a,137; ldh [$FFE2],a; ret`; the slot's other bytes
//! are untouched.
use crate::actor::{Actor, ContactKind};

pub(super) fn update(actor: &mut Actor) {
    actor.contact_kind = ContactKind::from_raw(137);
}
