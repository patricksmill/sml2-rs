//! Mario Zone boss controller (anim 12), ROM `$9E07`.

use crate::level::LevelLayout;
use crate::object::ObjectTables;
use crate::objects::{
    ObjectSim, Staging, B_DEF, B_FSTATE, B_PHASE, B_SCRX, B_SPRITE, B_T24, B_T28, B_TYPE,
};

impl ObjectSim {
    /// `$9E07` (boss anim 12): Pig 3 (brick house / Bupon), the controller.
    /// Waits for the parts to clear, spawns child anim 13 (brick house), drops
    /// from chimney, and leaps across the arena with high vertical arcs (step + 2).
    /// With each hit, `sBossHits` decreases, making Bupon bounce progressively faster.
    pub(in crate::objects) fn handler_boss_def2_controller(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = (layout, tables);
        self.boss_damage_pass(st);
        if self.done != 0 {
            self.spawn_sprite = st.b[B_SPRITE]; // $9E10: banks sprite before stun
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
                // $9E6E / $9EB5: 8-frame crouch on landing (sprite 40 -> 41)
                if st.b[B_T28] != 0 {
                    st.b[B_SPRITE] = 0x28;
                    st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                    if st.b[B_T28] != 0 {
                        return;
                    }
                    st.b[B_SPRITE] = 0x29;
                }
                // $9E81 / $9EC8: moves on even frames only
                if self.fff0 & 1 != 0 {
                    return;
                }
                let speed = 4u8.wrapping_sub(self.boss_hits_remaining >> 1);
                if st.b[B_FSTATE] == 4 {
                    self.scr_x_sub(st, speed); // $B264: left
                    if st.b[B_SCRX] < 0x10 {
                        self.boss_end_flip(st); // $9D3D: flip to state 5
                        return;
                    }
                } else {
                    self.scr_x_add(st, speed); // $B26A: right
                    if st.b[B_SCRX] >= 0xA0 {
                        self.boss_end_flip(st); // $9D3D: flip to state 4
                        return;
                    }
                }
                // $9E9A: vertical leaping motion (higher arc than Pig 2: step + 2)
                if st.b[B_T24] == 0 {
                    let (step, cap) = self.step_picker(st, &tables.boss_picker_profile_15, 15);
                    self.scr_y_sub(st, step.wrapping_add(2));
                    if st.b[B_PHASE] == cap {
                        st.b[B_PHASE] = 0;
                        st.b[B_T24] = 1;
                    }
                } else {
                    let (step, cap) = self.step_picker(st, &tables.boss_picker_profile, 15);
                    self.scr_y_add(st, step.wrapping_add(2));
                    if st.b[B_PHASE] == cap {
                        st.b[B_PHASE] = 0;
                        st.b[B_T24] = 0;
                        st.b[B_T28] = 8;
                        // $9DC7 completes by phase; keep the resulting height.
                        self.sfx1 = st.b[B_DEF];
                    }
                }
            }
            s => self.note_unported(0x9E5C + (s as u32) * 2),
        }
    }
}
