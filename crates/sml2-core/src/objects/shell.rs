//! Object shell routines.

use super::*;

impl ObjectSim {
    pub(super) fn handler_carried(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        if self.mario.travelling {
            // $58444: pipe travel → sActionLatch = 0, `$5B2D7` removal.
            self.action_latch = 0;
            self.remove_object(st, tables);
            return;
        }
        if st.b[B_FSTATE] == 0 {
            // $58452: type 0, state 1.
            st.b[B_TYPE] = 0;
            st.b[B_FSTATE] = 1;
            return;
        }
        self.follow_mario(st);
        self.ko_overlap(st);
        if self.object_hit_flag != 0 {
            self.release_and_defeat(st);
            return;
        }
        self.m_ceil_own(st, layout, 8); // $3B68
        if self.move_flags & 0x04 != 0 {
            self.release_and_defeat(st);
            return;
        }
        if self.mario.b_held {
            return;
        }
        // B released: `$3AB4` (`$85DC`) drops the anim, then the placement.
        self.action_latch = 0;
        st.b[B_DEF] = st.b[B_DEF].wrapping_sub(1);
        self.finish_sprite_swap(st, tables); // `$85DC` -> `$863C` -> `$8240`.
        self.kick_placement(st, layout);
    }

    pub(super) fn release_and_defeat(&mut self, st: &mut Staging) {
        self.action_latch = 0;
        self.defeat(st);
    }

    pub(super) fn follow_mario(&mut self, st: &mut Staging) {
        set_x(&mut st.b, self.mario.x);
        set_y(&mut st.b, self.mario.y);
        self.mv_x_add(st, 8);
        let b = match (self.mario.powerup != 0, self.mario.ducking) {
            (true, false) => 5,
            (false, true) => 20,
            _ => 14,
        };
        self.mv_y_add(st, b);
    }

    pub(super) fn ko_overlap(&mut self, st: &Staging) {
        let sy = st.b[B_SCRY];
        let sx = st.b[B_SCRX];
        for j in 0..SLOT_COUNT {
            let ty = self.slots[j].b[B_TYPE];
            if ty == 0 || ty == 0x5F || ty & 0xA0 != 0 {
                continue;
            }
            let b = &self.slots[j].b;
            let oy = b[B_SCRY];
            let ox = b[B_SCRX];
            let d = b[B_HB_TOP].wrapping_add(3).wrapping_add(oy);
            let e = b[B_HB_BOTTOM]
                .wrapping_add(10)
                .wrapping_add(oy)
                .wrapping_sub(d);
            if sy.wrapping_sub(d) >= e {
                continue;
            }
            let d = b[B_HB_LEFT].wrapping_sub(5).wrapping_add(ox);
            let e = b[B_HB_RIGHT]
                .wrapping_add(5)
                .wrapping_add(ox)
                .wrapping_sub(d);
            if sx.wrapping_sub(d) >= e {
                continue;
            }
            self.slots[j].b[B_FLAGS] |= 0x02;
            self.object_hit_flag = 1;
        }
    }

    pub(super) fn kick_placement(&mut self, st: &mut Staging, layout: &LevelLayout) {
        if self.mario.dir_right {
            self.mv_x_add(st, 16);
            self.m_right_wall_low(st, layout);
            if self.move_flags & 0x01 == 0 {
                return;
            }
            self.mv_x_sub(st, 16);
            st.b[B_TYPE] = 0x5F;
            st.b[B_FSTATE] = 1;
        } else {
            self.mv_x_sub(st, 16);
            self.m_left_wall_low(st, layout);
            if self.move_flags & 0x02 == 0 {
                return;
            }
            self.mv_x_add(st, 16);
            st.b[B_TYPE] = 0x5F;
            st.b[B_FSTATE] = 2;
        }
    }
}
