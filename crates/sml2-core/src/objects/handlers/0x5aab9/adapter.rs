//! Slot conversion and the bounce sound for ROM `$5AAB9` (mode B anim 98).
use super::{handler_0x5aab9, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5aab9(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x5aab9::update(&mut actor, self.easy_mode, -64);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5aab9::Effect::Continue) => {}
            Ok(handler_0x5aab9::Effect::Bounce) => self.sfx1 = 10,
            Err(_) => self.note_unported(0x5AAB9),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
