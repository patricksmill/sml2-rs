//! Slot conversion for ROM `$599A6` (mode A anim 86).
use super::{handler_0x599a6, *};
use crate::actor::Actor;
use std::cmp::Ordering;

struct Ground<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl handler_0x599a6::World for Ground<'_> {
    fn grounded(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x599a6(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let mario_ref_x = self.mario_ref_x;
        let mut world = Ground {
            sim: self,
            st,
            layout,
        };
        let result = handler_0x599a6::update(&mut actor, mario_ref_x, &mut world);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x599a6::Effect::FacedMario { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(_) => {}
            Err(_) => self.note_unported(0x599A6),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/umbrella_hopper.rs"]
mod tests;
