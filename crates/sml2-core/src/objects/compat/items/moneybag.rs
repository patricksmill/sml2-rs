//! Slot compatibility boundary for ROM `$587A1`.
use super::{handler_0x587a1, ObjectSim, Staging};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x587a1(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        handler_0x587a1::update(&mut actor, FrameTick(u64::from(self.fff0)));
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "../../items/tests/moneybag.rs"]
mod tests;
