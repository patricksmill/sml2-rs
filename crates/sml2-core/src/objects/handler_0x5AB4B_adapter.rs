//! Slot conversion for ROM `$5AB4B` (mode B anim 99).
use super::{handler_0x5ab4b, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5ab4b(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x5ab4b::update(&mut actor, self.easy_mode);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5ab4b::Effect::Continue) => {}
            Ok(handler_0x5ab4b::Effect::Bounce) => self.sfx1 = 10,
            Err(_) => self.note_unported(0x5AB4B),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x5AB4B_tests.rs"]
mod tests;
