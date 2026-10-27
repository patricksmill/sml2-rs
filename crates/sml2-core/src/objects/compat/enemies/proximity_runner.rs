//! Slot conversion for ROM `$59774` (mode A anim 84).
use super::{handler_0x59774, pipe_bobber::dist_mario_x, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

struct Probes<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl handler_0x59774::World for Probes<'_> {
    fn left_blocked(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_left_wall_high(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }
    fn right_blocked(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_right_wall_high(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x59774(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let mario_ref_x = self.mario_ref_x;
        let dist = dist_mario_x(mario_ref_x, actor.position.x);
        let mut world = Probes {
            sim: self,
            st,
            layout,
        };
        let result = handler_0x59774::update(&mut actor, frame, mario_ref_x, dist, &mut world);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x59774::Effect::FacedMario { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(_) => {}
            Err(_) => self.note_unported(0x59774),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/proximity_runner.rs"]
mod tests;
