//! Slot conversion for ROM `$5AD2D` (mode B anim 106).
use super::{handler_0x5ace4, handler_0x5ad2d, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5ad2d(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let bounds = self.horizontal_reset_bounds(handler_0x5ad2d::SCREEN_LIMIT);
        let result = handler_0x5ad2d::update(&mut actor, frame, bounds);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5ace4::Effect::Continue) => {}
            Ok(handler_0x5ace4::Effect::Flicker) => self.done = 1,
            Ok(handler_0x5ace4::Effect::Launch) => self.sfx2 = 6,
            Ok(handler_0x5ace4::Effect::Respawn) => self.respawn_object_5b2bf(st, tables),
            Err(_) => self.note_unported(0x5AD2D),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/sliding_mine.rs"]
mod tests;
