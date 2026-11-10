//! Slot compatibility boundary for ROM `$587A1`.
use super::{ObjectSim, Staging, handler_0x587a1};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x587a1(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        handler_0x587a1::update(&mut actor, self.frame_tick());
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "../../items/tests/moneybag.rs"]
mod tests;
