//! Slot conversion and terrain probes for ROM `$59E5B` (mode B anim 11).
use super::{handler_0x59e5b, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

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

impl handler_0x59e5b::World for Inputs<'_> {
    fn left_wall(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_left_wall(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }
    fn right_wall(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_right_wall(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
    fn ceiling(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_ceil_own(self.st, self.layout, 18);
        self.sim.move_flags & 0x04 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x59e5b(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let player_x = self.mario_ref_x;
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x59e5b::update(&mut actor, frame, player_x, &mut world)
        };
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x59e5b::Effect::Facing(relation)) => {
                self.facing = match relation {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x59e5b::Effect::Continue) => {}
            Err(_) => self.note_unported(0x59E5B),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x59E5B_tests.rs"]
mod tests;
