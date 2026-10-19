//! Slot conversion for ROM `$586E6` (mode B anim 31).
use super::{handler_0x586e6, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x586e6(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        handler_0x586e6::update(&mut actor);
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "../../items/tests/contact_marker.rs"]
mod tests;
