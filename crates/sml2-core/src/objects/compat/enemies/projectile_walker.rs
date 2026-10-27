//! Slot conversion, terrain probes, and dynamic spawn for ROM `$5A0AA` (mode B anim 37).
use super::terrain_adapter::TerrainWorld;
use super::{handler_0x5a0aa, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5a0aa(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let player_x = self.mario_ref_x;
        let result = {
            let mut world = TerrainWorld::new(self, st, layout);
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
#[path = "../../enemies/tests/projectile_walker.rs"]
mod tests;
