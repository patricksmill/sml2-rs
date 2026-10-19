//! Slot conversion for ROM `$59BE1` (mode A anim 89).
use super::{handler_0x59be1, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x59be1(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x59be1::update(&mut actor);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x59be1::Effect::Continue) => {}
            Ok(handler_0x59be1::Effect::Remove) => {
                self.remove_mode = 1;
                self.remove_object(st, tables);
            }
            Err(_) => self.note_unported(0x59BE1),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
