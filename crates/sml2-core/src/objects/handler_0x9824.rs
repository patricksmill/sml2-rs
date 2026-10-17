//! ROM `$9824` (boss anim 7): the door that the anim-6 boss opens.
//!
//! State 0 starts the matching side once the sequence is active. State 1
//! flashes for 64 frames. State 2 rises two pixels, carrying a rider, until
//! the span ends or the boss is being removed. State 3 clears the sequence.
//! State 4 falls until screen Y reaches `$84`, then returns to state 0.
use super::*;

impl ObjectSim {
    pub(super) fn handler_0x9824(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_FSTATE] == 0 {
            self.arm_the_matching_door(st);
            return;
        }
        match st.b[B_FSTATE] {
            1 => self.flash_the_door(st),
            2 => self.rise(st),
            3 => self.clear_the_sequence(st),
            4 => self.fall_to_the_sill(st, tables),
            _ => self.note_unported(0x9824),
        }
    }

    fn arm_the_matching_door(&mut self, st: &mut Staging) {
        if self.door_sequence_active == 0 || self.door_phase & 1 == 0 {
            return;
        }
        let slot = self.boss_slot_ptr_lo;
        let want = if (self.door_phase == 1) == (self.door_parity == 0) {
            0x20
        } else {
            0x60
        };
        if slot != want {
            return;
        }
        st.b[B_FSTATE] = 1;
        st.b[B_HITREACT] = 80;
        st.b[B_T28] = 64;
        st.b[B_SPRITE] = 25;
    }

    fn flash_the_door(&mut self, st: &mut Staging) {
        if self.fff0 & 3 == 0 {
            st.b[B_FLIP] ^= 0x20;
        }
        st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
        if st.b[B_T28] != 0 {
            return;
        }
        st.b[B_FSTATE] = 2;
        st.b[B_SPRITE] = 19;
        self.sfx2 = 5;
    }

    fn rise(&mut self, st: &mut Staging) {
        if self.remove_mode != 0 {
            st.b[B_TYPE] = 0;
            return;
        }
        self.scr_y_sub(st, 2);
        if st.b[B_FLAGS] & 0x08 != 0 {
            let y = self.mario_y_adjust.unwrap_or(self.mario.y);
            self.mario_y_adjust = Some(y.wrapping_sub(2));
        }
        st.b[B_HITREACT] = st.b[B_HITREACT].wrapping_sub(2);
        if st.b[B_HITREACT] != 0 {
            return;
        }
        st.b[B_FSTATE] = 3;
        st.b[B_T28] = 32;
    }

    fn clear_the_sequence(&mut self, st: &mut Staging) {
        st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
        if st.b[B_T28] != 0 {
            return;
        }
        self.door_sequence_active = 0;
        st.b[B_FSTATE] = 4;
    }

    fn fall_to_the_sill(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_SCRY] >= 0x84 {
            st.b[B_SCRY] = 0x84;
            st.b[B_FSTATE] = 0;
            return;
        }
        let (step, _) = self.step_picker(st, &tables.boss_picker_profile, 21);
        self.scr_y_add(st, step);
    }
}

#[cfg(test)]
#[path = "handler_0x9824_tests.rs"]
mod tests;
