//! Mario Zone / boss clear reward animation (anims 4 & 5), ROM `$9642` / `$9698`.

use crate::object::ObjectTables;
use crate::objects::{
    ObjectSim, Staging, B_ANIMFRAME, B_FLIP, B_FSTATE, B_PHASE, B_SCRY, B_T28,
};

impl ObjectSim {
    /// `$965E`: sprite animation helper for anim 4.
    fn boss_reward_sprite_helper(&self, st: &mut Staging) {
        if self.fff0 & 1 == 0 {
            let f = st.b[B_ANIMFRAME].wrapping_add(1);
            if f > 5 {
                st.b[B_ANIMFRAME] = 1;
            } else {
                st.b[B_ANIMFRAME] = f;
            }
            if st.b[B_ANIMFRAME] == 3 {
                st.b[B_FLIP] ^= 0x20;
            }
        }
    }

    /// `$9642` (boss anim 4): spawned by def-2 spawner or anim 13.
    pub(in crate::objects) fn handler_boss_anim4(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_FSTATE] == 0 {
            st.b[B_FSTATE] = 1;
            self.spawn_anim = 5;
            self.spawn_sprite = 0x0C;
            self.boss_spawn_child(st); // $B224
        }
        self.boss_reward_sprite_helper(st); // $965E
        self.handler_boss_anim4_5_states(st, tables, 0x966A);
    }

    /// `$9698` (boss anim 5): the second part of the spawn chain.
    pub(in crate::objects) fn handler_boss_anim5(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_FSTATE] == 0 {
            st.b[B_T28] = st.b[B_T28].wrapping_add(1);
            if st.b[B_T28] >= 8 {
                self.scr_y_add(st, 0x18);
                st.b[B_FSTATE] = 1;
                st.b[B_T28] = 0;
            }
            return;
        }
        self.handler_boss_anim4_5_states(st, tables, 0x96BE);
    }

    fn handler_boss_anim4_5_states(&mut self, st: &mut Staging, tables: &ObjectTables, table_base: u32) {
        match st.b[B_FSTATE] {
            1 => {
                let (dy, cap) = self.boss_reverse_picker_b1d6(st);
                self.scr_y_add(st, dy);
                if st.b[B_PHASE] == cap {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = 2;
                    st.b[B_FLIP] ^= 0x40;
                }
            }
            2 => {
                let (dy, cap) = self.step_picker(st, &tables.boss_picker_profile, 21);
                self.scr_y_add(st, dy);
                if st.b[B_PHASE] == cap {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = 3;
                    st.b[B_FLIP] ^= 0x40;
                }
            }
            3 => {
                // $9688: rise 4 px a frame until SCRY < 8 then level clear
                self.scr_y_sub(st, 4);
                if st.b[B_SCRY] < 8 {
                    self.boss_clear_flag = 1;
                }
            }
            s => self.note_unported(table_base + (s as u32) * 2),
        }
    }
}
