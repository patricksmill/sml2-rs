//! Slot conversion and dynamic spawn for ROM `$596D9` (anim 81).
use super::{handler_0x596d9, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x596d9(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = handler_0x596d9::update(&mut actor, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x596d9::Effect::Initialized { facing })
            | Ok(handler_0x596d9::Effect::Refaced { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x596d9::Effect::SpawnChild { anim, sprite, flip }) => {
                self.spawn_anim = anim;
                self.spawn_sprite = sprite;
                self.spawn_attr = flip.bits();
                self.spawn_dynamic_5b36d(st);
            }
            Ok(handler_0x596d9::Effect::Waiting) => {}
            Err(_) => self.note_unported(0x596D9),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/flyer_spawner.rs"]
mod tests;
