//! Slot conversion for ROM `$594FB` (mode A anim 69).
use super::{handler_0x594fb, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x594fb(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x594fb::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x594fb::Effect::Continue) => {}
            Ok(handler_0x594fb::Effect::Armed) => self.sfx1 = 13,
            Ok(handler_0x594fb::Effect::Remove) => {
                self.remove_mode = 1;
                self.remove_object(st, tables);
            }
            Err(_) => self.note_unported(0x594FB),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x594FB_tests.rs"]
mod tests;
