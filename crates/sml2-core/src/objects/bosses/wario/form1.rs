//! Wario's first form, ROM `$A8AB` (bank 2 CPU `$68AB`).

use crate::objects::{
    ObjectSim, ObjectTables, Staging, B_ANIMFRAME, B_FLASH, B_FLIP, B_FSTATE, B_PHASE, B_SCRX,
    B_SCRY, B_SPRITE, B_T24, B_T25, B_T28, B_TYPE,
};

impl ObjectSim {
    pub(in crate::objects) fn handler_wario20(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.boss_damage_pass(st);
        if self.done != 0 {
            self.spawn_sprite = st.b[B_SPRITE];
            self.saved_frame_state = st.b[B_FSTATE];
            self.saved_type = st.b[B_TYPE];
            if self.boss_hit_latch != 0 {
                st.b[B_SPRITE] = 0x52;
                st.b[B_T28] = self.boss_timer_reload;
            }
            self.boss_type0_state1(st);
            return;
        }
        match st.b[B_FSTATE] {
            0 => {
                if self.fade_timer as u8 == 0x61 {
                    self.sfx1 = 0x0F;
                    self.ending_door_active = 1;
                }
                if self.ending_door_active != 0 {
                    self.sprite_blink(st, 7, 1);
                }
                if !self.fade_idle() {
                    return;
                }
                self.ending_door_active = 0;
                st.b[B_FSTATE] = 3;
                st.b[B_TYPE] = 0x41;
                st.b[B_SPRITE] = 0x4F;
            }
            1 => self.boss_hit_stun(st),
            2 => {
                self.boss_flash_b40e(st);
                let (step, _) = self.step_picker(st, &tables.boss_picker_profile, 21);
                self.scr_y_add(st, step);
                if st.b[B_SCRY] >= 0x8A {
                    st.b[B_FSTATE] = 6;
                    st.b[B_T28] = 0x80;
                }
            }
            3 => {
                if st.b[B_T24] == 0 {
                    let (step, cap) = self.boss_reverse_picker_b1e4(st);
                    self.scr_y_sub(st, step);
                    if st.b[B_PHASE] == cap {
                        st.b[B_PHASE] = 0;
                        st.b[B_T24] = 1;
                    }
                } else {
                    let (step, _) = self.step_picker(st, &tables.boss_picker_profile, 24);
                    self.scr_y_add(st, step);
                    if st.b[B_SCRY] >= 0x8A {
                        st.b[B_PHASE] = 0;
                        st.b[B_T24] = 0;
                        self.wario20_impact(st, 0x34, 0x30);
                    }
                }
            }
            4 => {
                if self.timer_low & 7 == 0 {
                    st.b[B_FSTATE] = 5;
                    st.b[B_SPRITE] = 0x4F;
                    return;
                }
                self.boss_sprite_cycle_b339(st);
                if st.b[B_T25] == 0 {
                    self.scr_x_sub(st, 2);
                    if st.b[B_SCRX] >= 0x10 {
                        return;
                    }
                } else {
                    self.scr_x_add(st, 2);
                    if st.b[B_SCRX] < 0x98 {
                        return;
                    }
                }
                st.b[B_FLIP] ^= 0x20;
                st.b[B_T25] ^= 1;
                st.b[B_T28] = 8;
            }
            5 => {
                let cap = if st.b[B_T24] == 0 {
                    let (step, cap) = self.boss_reverse_picker_b1d6(st);
                    self.scr_y_sub(st, step);
                    cap
                } else {
                    let (step, cap) = self.step_picker(st, &tables.boss_picker_profile, 21);
                    self.scr_y_add(st, step);
                    cap
                };
                if st.b[B_PHASE] == cap {
                    st.b[B_PHASE] = 0;
                    st.b[B_T24] ^= 1;
                    if st.b[B_T24] == 0 {
                        self.wario20_impact(st, 0x38, 0x38);
                    }
                }
            }
            6 => {
                self.boss_flash_b40e(st);
                st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                if st.b[B_T28] == 0 {
                    st.b[B_ANIMFRAME] = 0;
                    st.b[B_FLASH] = 0;
                    st.b[B_FSTATE] = 8;
                    st.b[B_SPRITE] = 0x4C;
                    st.b[B_FLIP] = 0x20;
                }
            }
            7 => {
                if self.boss_delay == 0 {
                    return;
                }
                self.sprite_blink(st, 7, 0x1F);
                st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                if st.b[B_T28] == 0 {
                    st.b[B_FSTATE] = 11;
                    st.b[B_SPRITE] = 0x72;
                    self.boss_delay = 0;
                }
            }
            8 => self.wario_phase1_exit(st, tables),
            9 => {
                if st.b[B_T28] != 0 {
                    st.b[B_T28] -= 1;
                    return;
                }
                let cap = if st.b[B_T24] == 0 {
                    let (step, cap) = self.boss_reverse_picker_b1e4(st);
                    self.scr_y_sub(st, step);
                    cap
                } else {
                    let (step, cap) = self.step_picker(st, &tables.boss_picker_profile, 12);
                    self.scr_y_add(st, step);
                    cap
                };
                if st.b[B_PHASE] == cap {
                    st.b[B_PHASE] = 0;
                    st.b[B_ANIMFRAME] = 0;
                    st.b[B_SPRITE] = 0x4C;
                    st.b[B_T24] ^= 1;
                    if st.b[B_T24] == 0 {
                        st.b[B_FSTATE] = 4;
                    }
                }
            }
            _ => self.note_unported(0xA8AB),
        }
    }

    fn wario20_impact(&mut self, st: &mut Staging, pan: u8, wait: u8) {
        st.b[B_FSTATE] = 9;
        self.camera_pan_set = Some(pan);
        self.boss_transition = 1;
        self.sfx0 = 0x18;
        st.b[B_T28] = wait;
    }
}

#[cfg(test)]
#[path = "tests/form1.rs"]
mod tests;
