//! Slot conversion for ROM `$5A8D2` (mode B anims 53 / 81).
use super::{handler_0x5a8d2, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5a8d2(&mut self, st: &mut Staging) {
        self.done = 1;
        let mut actor = Actor::from_raw_slot(&st.b);
        handler_0x5a8d2::update(&mut actor);
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
