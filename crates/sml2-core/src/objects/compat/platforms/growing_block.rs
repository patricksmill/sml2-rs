//! Slot conversion for ROM `$5AC7D` (mode B anim 103).
use super::{handler_0x5ac7d, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5ac7d(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5ac7d::update(&mut actor, frame, self.easy_mode);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5ac7d::Effect::Continue) => {}
            Ok(handler_0x5ac7d::Effect::Chime) => self.sfx2 = 1,
            Ok(handler_0x5ac7d::Effect::Despawn) => self.remove_object(st, tables),
            Err(_) => self.note_unported(0x5AC7D),
        }
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/growing_block.rs"]
mod tests;
