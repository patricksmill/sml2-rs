//! Mario Zone boss controller (anim 12), ROM `$9E07`.

use crate::level::LevelLayout;
use crate::object::ObjectTables;
use crate::objects::{
    ObjectSim, Staging, B_FSTATE, B_SCRX, B_SPRITE, B_T28, B_TYPE,
};

impl ObjectSim {
    /// `$9E07` (boss anim 12): the controller. It has no HP — its hit body banks
    /// the slot and parks it without touching `$FFD8` — waits for the parts to
    /// clear, then spawns one child (anim `$0D`, sprite `$0E`) and starts the
    /// fade-done walk, whose speed comes from `sBossHits` (`4 - hits/2`).
    pub(in crate::objects) fn handler_boss_def2_controller(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = (layout, tables);
        self.boss_damage_pass(st);
        if self.done != 0 {
            self.spawn_sprite = st.b[B_SPRITE]; // $9E10: no hp decrement here
            self.saved_frame_state = st.b[B_FSTATE];
            self.saved_type = st.b[B_TYPE];
            if self.boss_hit_latch != 0 {
                st.b[B_SPRITE] = 0x1F;
                st.b[B_T28] = self.boss_timer_reload;
            }
            self.boss_type0_state1(st); // $9725
            return;
        }
        if st.b[B_FSTATE] == 0 {
            self.flip_blink(st);
            if self.boss_def_count != 0 {
                return; // $9E37: wait for the parts
            }
            self.remove_mode = 0;
            st.b[B_T28] = st.b[B_T28].wrapping_add(1);
            if st.b[B_T28] != 0x30 {
                return;
            }
            self.spawn_anim = 0x0D;
            self.spawn_sprite = 0x0E;
            self.boss_spawn_child(st); // $B224
            self.boss_fade_done(st);
            return;
        }
        match st.b[B_FSTATE] {
            1 => self.boss_hit_stun(st),
            2 => {
                self.sprite_blink(st, 0x01, 0x01);
                self.boss_morph_to_anim3(st);
            }
            3 => {
                st.b[B_T28] = 8;
                self.boss_bob(st, tables);
            }
            4 | 5 => {
                let b = 4u8.wrapping_sub(self.boss_hits_remaining >> 1);
                if st.b[B_FSTATE] == 4 {
                    if self.fff0 & 1 != 0 {
                        return;
                    }
                    self.scr_x_add(st, b);
                    if st.b[B_SCRX] >= 0xA0 {
                        self.boss_end_flip(st);
                        return;
                    }
                } else {
                    if self.fff0 & 1 != 0 {
                        return;
                    }
                    self.scr_x_sub(st, b);
                    if st.b[B_SCRX] < 0x10 {
                        self.boss_end_flip(st);
                        return;
                    }
                }
            }
            s => self.note_unported(0x9E5C + (s as u32) * 2),
        }
    }
}
