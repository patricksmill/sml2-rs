//! Slot conversion for ROM `$5B06D` (mode B anim 89).
use super::{handler_0x5b06d, *};
use crate::actor::Actor;

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl handler_0x5b06d::World for Inputs<'_> {
    fn probe_ground(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5b06d(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let player_x = self.mario_ref_x;
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x5b06d::update(&mut actor, player_x, &mut world)
        };
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5b06d::Effect::Continue) => {}
            Ok(handler_0x5b06d::Effect::Remove) => {
                self.remove_mode = 2;
                self.remove_object(st, tables);
            }
            Err(_) => self.note_unported(0x5B06D),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x5B06D_tests.rs"]
mod tests;
