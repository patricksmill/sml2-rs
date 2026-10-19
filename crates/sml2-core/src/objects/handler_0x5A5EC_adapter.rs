//! Slot conversion, rider carry, and spawn restore for ROM `$5A5EC` (mode B anim 71).
use super::{handler_0x5a5ec, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5a5ec(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = handler_0x5a5ec::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a5ec::Effect::Moved { dx, dy, riding }) if riding => {
                self.carry_mario_dx(dx);
                let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
            }
            Ok(handler_0x5a5ec::Effect::Moved { .. }) | Ok(handler_0x5a5ec::Effect::Continue) => {}
            Ok(handler_0x5a5ec::Effect::Respawn) => self.respawn_object_5b2bf(st, tables),
            Err(_) => self.note_unported(0x5A5EC),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x5A5EC_tests.rs"]
mod tests;
