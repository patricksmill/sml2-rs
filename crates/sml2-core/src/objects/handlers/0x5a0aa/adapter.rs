//! Slot conversion, terrain probes, and dynamic spawn for ROM `$5A0AA` (mode B anim 37).
use super::{handler_0x5a0aa, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl fallwalker::FallWalkerWorld for Inputs<'_> {
    fn probe_left_wall(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_left_wall(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }
    fn probe_right_wall(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_right_wall(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
    fn probe_ground(&mut self, actor: &Actor) -> bool {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
    fn step_fall(&mut self, _actor: &mut Actor) -> u8 {
        0
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5a0aa(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let player_x = self.mario_ref_x;
        let result = {
            let mut world = Inputs {
                sim: self,
                st,
                layout,
            };
            handler_0x5a0aa::update(&mut actor, frame, player_x, &mut world)
        };
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a0aa::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5a0aa::Effect::Spawn { anim, sprite, flip }) => {
                self.spawn_anim = anim;
                self.spawn_sprite = sprite;
                self.spawn_attr = flip.bits();
                self.spawn_dynamic_5b36d(st);
                self.sfx1 = 17;
            }
            Ok(handler_0x5a0aa::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5A0AA),
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
