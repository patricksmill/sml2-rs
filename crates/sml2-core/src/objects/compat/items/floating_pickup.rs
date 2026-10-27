//! Slot conversion for ROM `$58804` (mode B anim 24).
use super::{handler_0x58804, pipe_bobber, *};
use crate::actor::Actor;

struct Inputs<'a> {
    sim: &'a ObjectSim,
}

impl handler_0x58804::World for Inputs<'_> {
    fn mario_distance_x(&self, actor_x: u16) -> u8 {
        pipe_bobber::dist_mario_x(self.sim.mario_ref_x, actor_x)
    }
}

impl ObjectSim {
    pub(super) fn handler_0x58804(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x58804::update(&mut actor, frame, &Inputs { sim: self });
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x58804::Effect::Continue) => {}
            Ok(handler_0x58804::Effect::Despawn) => self.remove_object(st, tables),
            Err(_) => self.note_unported(0x58804),
        }
    }
}

#[cfg(test)]
#[path = "../../items/tests/floating_pickup.rs"]
mod tests;
