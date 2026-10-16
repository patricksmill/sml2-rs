//! Wario's moving child, ROM `$AAA5`, shared by boss animations 21 and 24.

use super::*;

impl ObjectSim {
    pub(super) fn handler_wario21(&mut self, st: &mut Staging, tables: &ObjectTables) {
        match st.b[B_FSTATE] {
            0 => st.b[B_FSTATE] = 1,
            1 => {
                if self.fff0 & 3 == 0 {
                    if st.b[B_T25] == 0 {
                        self.scr_x_sub(st, 3);
                    } else {
                        self.scr_x_add(st, 3);
                    }
                    if st.b[B_SCRX] >= 0xB0 {
                        st.b[B_T25] ^= 1;
                    }
                }
                if self.boss_transition != 0 {
                    self.boss_transition = 0;
                    st.b[B_FSTATE] = 2;
                    st.b[B_TYPE] = 0x40;
                }
            }
            2 => {
                self.boss_flash_b40e(st);
                let (step, cap) = self.step_picker(st, &tables.boss_picker_profile, 36);
                self.scr_y_add(st, step);
                if st.b[B_PHASE] == cap {
                    st.b[B_FLASH] = 0;
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = 3;
                }
            }
            3 => {
                if self.fff0 & 7 != 0 {
                    return;
                }
                st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_add(1);
                if st.b[B_SPRITE] != 0x63 {
                    return;
                }
                st.b[B_SCRY] = 0x38;
                st.b[B_SPRITE] = 0x60;
                st.b[B_TYPE] = 0;
                st.b[B_FSTATE] = 0;
                st.b[B_T25] = 0;
                if self.div & 1 != 0 {
                    st.b[B_SCRX] = 0xA7;
                } else {
                    st.b[B_SCRX] = 0;
                    st.b[B_T25] = 1;
                }
            }
            _ => self.note_unported(0xAAA5),
        }
    }
}

#[cfg(test)]
#[path = "wario21_tests.rs"]
mod tests;
