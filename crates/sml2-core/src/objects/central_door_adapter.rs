//! Slot conversion and world effects for boss anim 8.
use super::{central_door, ObjectSim, ObjectTables, Staging};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_central_door(&mut self, staging: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&staging.b);
        let result = central_door::update(
            &mut actor,
            central_door::Inputs {
                remove_mode: self.remove_mode,
                sequence_active: self.door_sequence_active != 0,
                door_phase: self.door_phase,
                frame: FrameTick(u64::from(self.fff0)),
                fall_profile: &tables.boss_picker_profile,
            },
        );
        staging.b = actor.to_raw_slot();
        match result {
            Ok(effects) => {
                if let Some(dy) = effects.carry_dy {
                    let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                    self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
                }
                if effects.clear_sequence {
                    self.door_sequence_active = 0;
                }
                if let Some(sound) = effects.sound {
                    self.sfx2 = sound;
                }
            }
            Err(_) => self.note_unported(0x98C8),
        }
    }
}

#[cfg(test)]
#[path = "central_door_adapter_tests.rs"]
mod tests;
