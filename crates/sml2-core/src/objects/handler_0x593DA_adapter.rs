//! Slot conversion for ROM `$593DA` (anim 64).
use super::{handler_0x593da, *};
use crate::actor::{Actor, FrameTick};

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl handler_0x593da::World for Inputs<'_> {
    fn mario_distance_x(&self, actor_x: u16) -> u8 {
        pipe_bobber::dist_mario_x(self.sim.mario_ref_x, actor_x)
    }
    fn probe_ground(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
    fn probe_ceiling(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ceil_own(self.st, self.layout, 18);
        self.sim.move_flags & 0x04 != 0
    }
    fn request_sfx1(&mut self, id: u8) {
        self.sim.sfx1 = id;
    }
}

impl ObjectSim {
    pub(super) fn handler_0x593da(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x593da::update(&mut actor, frame, &mut world)
        };
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x593DA);
        }
    }
}

#[cfg(test)]
#[path = "handler_0x593DA_tests.rs"]
mod tests;
