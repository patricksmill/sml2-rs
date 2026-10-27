//! Slot conversion and dynamic spawn for ROM `$59FDF` (mode B anim 35).
use super::{handler_0x59fdf, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x59fdf(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x59fdf::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x59fdf::Effect::Initialized { facing })
            | Ok(handler_0x59fdf::Effect::Refaced { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x59fdf::Effect::Spawn { anim, sprite, flip }) => {
                self.spawn_anim = anim;
                self.spawn_sprite = sprite;
                self.spawn_attr = flip.bits();
                self.spawn_dynamic_5b36d(st);
            }
            Ok(handler_0x59fdf::Effect::Waiting) => {}
            Err(_) => self.note_unported(0x59FDF),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/bobbing_spawner.rs"]
mod tests;
