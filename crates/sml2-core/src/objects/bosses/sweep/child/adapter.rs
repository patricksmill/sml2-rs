//! Slot conversion and sound effect boundary for boss anim 17.
use super::{sweep_child, ObjectSim, Staging};
use crate::actor::Actor;

impl ObjectSim {
    pub(super) fn handler_sweep_child(&mut self, staging: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = sweep_child::update(&mut actor, self.frame_tick());
        staging.b = actor.to_raw_slot();
        match result {
            Ok(Some(sound)) => self.sfx2 = sound,
            Ok(None) => {}
            Err(_) => self.note_unported(0xA27E),
        }
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
