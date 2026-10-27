//! Slot conversion, rider carry, and spawn restore for ROM `$5A258` (mode B anim 63).
use super::{handler_0x5a258, *};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_0x5a258(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x5a258::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a258::Effect::Moved { carry_dy: Some(dy) }) => {
                let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
            }
            Ok(handler_0x5a258::Effect::Moved { carry_dy: None })
            | Ok(handler_0x5a258::Effect::Continue) => {}
            Ok(handler_0x5a258::Effect::Respawn) => self.respawn_object_5b2bf(st, tables),
            Err(_) => self.note_unported(0x5A258),
        }
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/rider_drop.rs"]
mod tests;
