//! Mario Zone boss parts (anims 10 & 11), ROM `$9C36` / `$9D4F`.

use crate::level::LevelLayout;
use crate::object::ObjectTables;
use crate::objects::{
    ObjectSim, Staging, B_ANIMFRAME, B_DEF, B_FLIP, B_FSTATE, B_PHASE, B_SCRX, B_SCRY, B_SPRITE,
    B_STATE, B_T24, B_T28, B_TYPE,
};

impl ObjectSim {
    pub(in crate::objects) fn flip_blink(&self, st: &mut Staging) {
        if self.fff0 & 0x0F == 0 {
            st.b[B_FLIP] ^= 0x20;
        }
    }

    pub(in crate::objects) fn sprite_cycle8(&self, st: &mut Staging) {
        if self.fff0 & 0x03 != 0 {
            return;
        }
        let f = st.b[B_ANIMFRAME].wrapping_add(1);
        if f == 8 {
            st.b[B_ANIMFRAME] = 0;
            st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_sub(7);
        } else {
            st.b[B_ANIMFRAME] = f;
            st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_add(1);
        }
    }

    /// `$9C3F`: the hit body anims 10 and 11 share. A stomp takes 2 HP, a
    /// projectile 1, and 0 *or a borrow* sends the part to `$5435` — the
    /// defeat fall. The controller never runs this.
    pub(in crate::objects) fn boss_hp_hit(&mut self, st: &mut Staging) {
        self.spawn_sprite = st.b[B_SPRITE];
        self.saved_frame_state = st.b[B_FSTATE];
        self.saved_type = st.b[B_TYPE];
        let mut b = 1u8;
        if self.boss_hit_latch != 0 {
            self.sfx0 = 0x13;
            b = 2;
            st.b[B_SPRITE] = 0x1F;
            st.b[B_T28] = self.boss_timer_reload;
        } else {
            self.sfx1 = 9;
        }
        st.b[B_TYPE] = 0;
        let (hp, borrow) = st.b[B_STATE].overflowing_sub(b);
        st.b[B_STATE] = hp;
        if hp == 0 || borrow {
            self.boss_defeat_fall(st);
            return;
        }
        st.b[B_FSTATE] = 1;
    }

    /// `$9C8B`: the intro's end — arm the fight (type `$45`, state 3, sprite
    /// `$20`, a `$2F`-style sfx).
    pub(in crate::objects) fn boss_fade_done(&mut self, st: &mut Staging) {
        st.b[B_SPRITE] = 0x20;
        st.b[B_T28] = 0;
        st.b[B_FSTATE] = 3;
        st.b[B_TYPE] = 0x45;
        self.sfx1 = 4;
    }

    /// `$9CDF`: the bob body the walk states run each frame. `B_T24` picks the
    /// ascent profile (cap 15) or the descent one (cap 21) — the descent also
    /// ends the flight at `B_SCRY = $90`.
    pub(in crate::objects) fn boss_bob(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_T24] == 0 {
            let (b, c) = self.step_picker(st, &tables.boss_picker_profile_15, 15);
            self.scr_y_sub(st, b);
            if st.b[B_PHASE] != c {
                return;
            }
            st.b[B_PHASE] = 0;
            st.b[B_T24] = 1;
            return;
        }
        let (b, _) = self.step_picker(st, &tables.boss_picker_profile, 21);
        self.scr_y_add(st, b);
        if st.b[B_SCRY] < 0x90 {
            return;
        }
        st.b[B_SCRY] = 0x90;
        st.b[B_PHASE] = 0;
        st.b[B_T24] = 0;
        if st.b[B_FSTATE] <= 3 {
            st.b[B_FSTATE] = 4;
            st.b[B_FLIP] = 0x20;
        }
    }

    /// `$9D3D`: a walk hit the end of the arena — flip the state bit and the
    /// object, and re-announce the anim id.
    pub(in crate::objects) fn boss_end_flip(&mut self, st: &mut Staging) {
        st.b[B_FSTATE] ^= 1;
        st.b[B_FLIP] ^= 0x20;
        self.sfx1 = st.b[B_DEF];
    }

    /// `$9C36` (boss anim 10): Pig 1 (straw house) runs horizontally along the floor.
    pub(in crate::objects) fn handler_boss_def2_pig1(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = layout;
        self.boss_damage_pass(st);
        if self.done != 0 {
            self.boss_hp_hit(st); // $9C3F
            return;
        }
        if st.b[B_FSTATE] == 0 {
            self.flip_blink(st); // $B402
            if !self.fade_idle() {
                return; // $B14E
            }
            self.boss_fade_done(st); // $9C8B
            return;
        }
        match st.b[B_FSTATE] {
            1 => self.boss_hit_stun(st), // $9CAC
            2 => {
                self.sprite_blink(st, 0x01, 0x01); // $B2F6
                self.boss_morph_to_anim3(st); // $9443
            }
            3 => {
                self.scr_x_add(st, 1);
                self.boss_bob(st, tables); // $9CDF
            }
            4 | 5 => {
                let hp = st.b[B_STATE] >> 1;
                let b = 4u8.wrapping_sub(hp);
                self.sprite_cycle8(st); // $B3D7
                if st.b[B_FSTATE] == 4 {
                    self.scr_x_add(st, b);
                    if st.b[B_SCRX] >= 0xA0 {
                        self.boss_end_flip(st); // $9D3D
                    }
                } else {
                    self.scr_x_sub(st, b);
                    if st.b[B_SCRX] < 0x10 {
                        self.boss_end_flip(st);
                    }
                }
            }
            s => self.note_unported(0x9CA1 + (s as u32) * 2),
        }
    }

    /// `$9D4F` (boss anim 11): Pig 2 (wood house) hops/bounces across the room.
    pub(in crate::objects) fn handler_boss_def2_pig2(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = layout;
        self.boss_damage_pass(st);
        if self.done != 0 {
            self.boss_hp_hit(st); // $9C3F
            return;
        }
        if st.b[B_FSTATE] == 0 {
            self.flip_blink(st); // $B402
            if !self.fade_idle() {
                return; // $B14E
            }
            // $9D63: waits until exactly one child is left.
            if self.boss_def_count != 1 {
                return;
            }
            self.remove_mode = 0;
            st.b[B_T28] = st.b[B_T28].wrapping_add(1);
            if st.b[B_T28] != 0x30 {
                return;
            }
            self.boss_fade_done(st);
            return;
        }
        match st.b[B_FSTATE] {
            1 => self.boss_hit_stun(st), // $9CAC
            2 => {
                self.sprite_blink(st, 0x01, 0x01); // $B2F6
                self.boss_morph_to_anim3(st); // $9443
            }
            3 => {
                self.scr_x_sub(st, 1);
                st.b[B_T28] = 8;
                let before_y = st.b[B_SCRY];
                self.boss_bob(st, tables); // $9CDF
                if before_y < 0x90 && st.b[B_SCRY] == 0x90 {
                    // Landed from right-side chimney; face and hop left.
                    st.b[B_FSTATE] = 5;
                    st.b[B_FLIP] = 0;
                    st.b[B_SPRITE] = 0x28;
                }
            }
            4 | 5 => {
                let hp = st.b[B_STATE] >> 1;
                let b = 4u8.wrapping_sub(hp);
                if st.b[B_T28] != 0 {
                    st.b[B_SPRITE] = if st.b[B_T28] == 1 { 0x20 } else { 0x28 };
                    st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                    if st.b[B_T28] != 0 {
                        return;
                    }
                }
                if st.b[B_FSTATE] == 4 {
                    self.scr_x_add(st, b);
                    if st.b[B_SCRX] >= 0xA0 {
                        self.boss_end_flip(st);
                    }
                } else {
                    self.scr_x_sub(st, b);
                    if st.b[B_SCRX] < 0x10 {
                        self.boss_end_flip(st);
                    }
                }
                let before_y = st.b[B_SCRY];
                self.boss_bob(st, tables);
                if before_y < 0x90 && st.b[B_SCRY] == 0x90 {
                    st.b[B_T28] = 8;
                    st.b[B_SPRITE] = 0x28;
                }
            }
            s => self.note_unported(0x9D78 + (s as u32) * 2),
        }
    }

    /// Shared router for def-2 parts: `left == false` -> Pig 1, `left == true` -> Pig 2.
    pub(in crate::objects) fn handler_boss_def2_part(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
        left: bool,
    ) {
        if left {
            self.handler_boss_def2_pig2(st, layout, tables);
        } else {
            self.handler_boss_def2_pig1(st, layout, tables);
        }
    }
}
