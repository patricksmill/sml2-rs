//! Slot boundary for the native side door (boss anim 7, ROM `$9824`).
use super::*;
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x9824(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = side_door::update(
            &mut actor,
            central_door::Inputs {
                remove_mode: self.remove_mode,
                sequence_active: self.door_sequence_active != 0,
                door_phase: self.door_phase,
                frame: FrameTick(u64::from(self.fff0)),
                fall_profile: &tables.boss_picker_profile,
            },
            self.door_parity,
            self.boss_slot_ptr_lo,
        );
        st.b = actor.to_raw_slot();
        self.apply_door_result(result, 0x9824);
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
