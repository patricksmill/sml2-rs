//! ROM `$5A8D2` (mode B anims 53 / 81): mark the slot done and arm contact 16
//! on the first call when state is still zero.
use crate::actor::{Actor, ContactKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Effect {
    /// State was already non-zero; only the done flag applies.
    AlreadyActive,
    /// Armed contact 16 and advanced to state 1.
    Armed,
}

pub(super) fn update(actor: &mut Actor) -> Effect {
    if actor.state != 0 {
        return Effect::AlreadyActive;
    }
    actor.contact_kind = ContactKind::from_raw(16);
    actor.state = 1;
    Effect::Armed
}
