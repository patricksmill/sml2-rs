//! Slot conversion and sound effect boundary for boss anim 17.
use super::{sweep_child, ObjectSim, Staging};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_sweep_child(&mut self, staging: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = sweep_child::update(&mut actor, FrameTick(u64::from(self.fff0)));
        staging.b = actor.to_raw_slot();
        match result {
            Ok(Some(sound)) => self.sfx2 = sound,
            Ok(None) => {}
            Err(_) => self.note_unported(0xA27E),
        }
    }
}

#[cfg(test)]
#[path = "sweep_child_adapter_tests.rs"]
mod tests;
