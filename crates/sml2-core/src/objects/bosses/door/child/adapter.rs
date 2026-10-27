//! Staging and shared boss effects for anim 9.
use super::{door_child, ObjectSim, Staging};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_door_child(&mut self, staging: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = door_child::update(
            &mut actor,
            door_child::Inputs {
                frame: self.frame_tick(),
                door_phase: self.door_phase,
                sequence_active: self.door_sequence_active != 0,
            },
        );
        staging.b = actor.to_raw_slot();
        match result {
            Ok(effects) => {
                if effects.activate_sequence {
                    self.door_sequence_active = 1;
                }
                if effects.counted_remove {
                    self.boss_finish_counted(staging);
                }
            }
            Err(_) => self.note_unported(0x9927),
        }
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
