//! Slot conversion for ROM `$5AA68` (mode B anim 84).
use super::{handler_0x5917a, handler_0x5aa68, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5aa68(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let bounds = self.vertical_reset_bounds(handler_0x5917a::FALL_LIMIT);
        let result = handler_0x5aa68::update(&mut actor, bounds);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5917a::Effect::Continue | handler_0x5917a::Effect::MoneyBag) => {}
            Ok(handler_0x5917a::Effect::Respawn) => self.respawn_object_5b2bf(st, tables),
            Err(_) => self.note_unported(0x5AA68),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/splitter_respawn.rs"]
mod tests;
