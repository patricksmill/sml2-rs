//! Slot conversion for ROM `$5A031` (mode B anim 36).
use super::{handler_0x5a031, *};
use crate::actor::{Actor, FrameTick};

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl Inputs<'_> {
    fn sync(&mut self, actor: &Actor) {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
    }
}

impl handler_0x5a031::World for Inputs<'_> {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_left_wall(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }
    fn probe_right_wall(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_right_wall(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
    fn probe_ground(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5a031(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x5a031::update(&mut actor, frame, &mut world)
        };
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a031::Effect::Continue) => {}
            Ok(handler_0x5a031::Effect::Remove) => {
                self.remove_mode = 2;
                self.remove_object(st, tables);
            }
            Err(_) => self.note_unported(0x5A031),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/side_wall_bouncer.rs"]
mod tests;
