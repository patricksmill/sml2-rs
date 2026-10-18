//! Slot conversion and dynamic spawn for ROM `$5A8E4` (mode B anim 54).
use super::{handler_0x5a8e4, *};
use crate::actor::Actor;

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl handler_0x5a8e4::World for Inputs<'_> {
    fn probe_ground(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ground_below16(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5a8e4(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let player_x = self.mario_ref_x;
        let frame_even = self.fff0 & 1 == 0;
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x5a8e4::update(&mut actor, player_x, frame_even, &mut world)
        };
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a8e4::Effect::Continue) => {}
            Ok(handler_0x5a8e4::Effect::SpawnChild {
                anim,
                sprite,
                flip,
                lift,
                sfx1,
            }) => {
                self.spawn_anim = anim;
                self.spawn_sprite = sprite;
                self.spawn_attr = flip.bits();
                // $5B36D copies the parent's pre-lift position.
                self.spawn_dynamic_5b36d(st);
                self.mv_y_sub(st, lift);
                self.sfx1 = sfx1;
            }
            Err(_) => self.note_unported(0x5A8E4),
        }
    }
}

#[cfg(test)]
#[path = "handler_0x5A8E4_tests.rs"]
mod tests;
