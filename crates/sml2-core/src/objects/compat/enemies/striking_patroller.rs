//! Slot conversion and terrain probes for ROM `$5A179` (mode B anim 39).
use super::terrain_adapter::TerrainWorld;
use super::{handler_0x5a179, pipe_bobber, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl handler_0x5a179::World for TerrainWorld<'_> {
    fn mario_distance_x(&self, actor_x: u16) -> u8 {
        pipe_bobber::dist_mario_x(self.sim.mario_ref_x, actor_x)
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5a179(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let player_x = self.mario_ref_x;
        let result = {
            let mut world = TerrainWorld::new(self, st, layout);
            handler_0x5a179::update(&mut actor, frame, player_x, &mut world)
        };
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a179::Effect::Initialized { facing })
            | Ok(handler_0x5a179::Effect::Refaced { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5a179::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5A179),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/striking_patroller.rs"]
mod tests;
