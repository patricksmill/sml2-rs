//! Slot conversion for ROM `$5ACE4` (mode B anims 104 and 105).
use super::{handler_0x5ace4, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5ace4(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5ace4::update(
            &mut actor,
            frame,
            handler_0x5ace4::Course {
                dx: -2,
                screen_limit: 0xB0,
                face_right: false,
            },
        );
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5ace4::Effect::Continue) => {}
            Ok(handler_0x5ace4::Effect::Flicker) => self.done = 1,
            Ok(handler_0x5ace4::Effect::Launch) => self.sfx2 = 6,
            Ok(handler_0x5ace4::Effect::Respawn) => self.respawn_object_5b2bf(st, tables),
            Err(_) => self.note_unported(0x5ACE4),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/flashing_mine.rs"]
mod tests;
