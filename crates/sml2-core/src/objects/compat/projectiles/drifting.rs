//! Slot conversion for ROM `$5A11E` (mode B anim 38).
use super::{handler_0x5a11e, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5a11e(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5a11e::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x5A11E);
        }
    }
}

#[cfg(test)]
#[path = "../../projectiles/tests/drifting.rs"]
mod tests;
