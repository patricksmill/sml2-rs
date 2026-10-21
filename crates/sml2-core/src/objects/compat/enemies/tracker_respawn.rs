//! Slot conversion for ROM `$5917A` (mode A anim 54).
use super::{handler_0x5917a, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5917a(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x5917a::update(&mut actor);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5917a::Effect::Continue) => {}
            Ok(handler_0x5917a::Effect::MoneyBag) => self.add_money_bag_reward(),
            Ok(handler_0x5917a::Effect::Respawn) => self.respawn_object_5b2bf(st, tables),
            Err(handler_0x5917a::Error::Rise(_)) => self.note_unported(0x581CE),
            Err(_) => self.note_unported(0x5917A),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/tracker_respawn.rs"]
mod tests;
