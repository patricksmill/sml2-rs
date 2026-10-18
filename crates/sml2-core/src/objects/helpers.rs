//! Object helpers routines.

use super::*;

// ROM helper entry points are kept together even when the current port has no caller for every probe.
#[allow(dead_code)]
impl ObjectSim {
    pub(super) fn probe(layout: &LevelLayout, x: u16, y: u16) -> u8 {
        layout
            .get(((x >> 4) & 0xFF) as usize, ((y >> 4) & 0xFF) as usize)
            .unwrap_or(0)
    }

    pub(super) fn probe_bias(layout: &LevelLayout, x: u16, y: u16) -> u8 {
        Self::probe(layout, x.wrapping_sub(8), y.wrapping_sub(16))
    }

    pub(super) fn derive_sprite(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_RECHI] >= 0xAD {
            return;
        }
        let sprite = tables.sprite_for(self.level_id, st.b[B_DEF]).unwrap_or(0);
        st.b[B_SPRITE] = sprite;
        // $863C stages $FFDD → sObjectSpawnAttr and $8240 writes it back to
        // +13, so the flip byte is preserved; +14..+30 are zeroed.
        for b in st.b.iter_mut().take(31).skip(14) {
            *b = 0;
        }
    }

    pub(super) fn finish_sprite_swap(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.derive_sprite(st, tables);
        self.done = 1;
    }

    pub(super) fn record_index(&self, st: &Staging) -> Option<usize> {
        let hi = st.b[B_RECHI];
        if hi >= 0xAD {
            return None;
        }
        let addr = ((hi as u16) << 8) | st.b[B_RECLO] as u16;
        let ri = addr.wrapping_sub(STREAM_BASE) as usize / RECORD_LEN;
        (ri < self.records.len()).then_some(ri)
    }

    pub(super) fn record_set_anim(&mut self, st: &Staging, anim: u8) {
        if let Some(ri) = self.record_index(st) {
            self.records[ri].anim_index = anim;
        }
    }

    pub(super) fn remove_object(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if self.remove_mode == 0 {
            if let Some(ri) = self.record_index(st) {
                let actor = crate::actor::Actor::from_raw_slot(&st.b);
                self.records[ri].mode_flag = if actor.marker == 0x10 { 0x10 } else { 0 };
                self.spawned[ri] = false;
            }
        }
        if st.b[B_STATE] == 1 {
            self.standby = self.standby.wrapping_sub(1);
        } else {
            self.active = self.active.wrapping_sub(1);
            if self.remove_mode == 1 {
                self.money_bag_add();
            }
        }
        st.b = [0; 32];
        self.refresh_hitbox(st, tables, false);
        self.done = 2;
        self.remove_mode = 0;
    }

    pub(super) fn money_bag_add(&mut self) {
        self.add_money_bag_reward();
    }

    pub(super) fn bonus_spawn(&mut self) {
        self.spawn_reward_from_camera();
    }

    pub(super) fn sprite_toggle(&self, st: &mut Staging) {
        if self.fff0 & 3 == 0 {
            st.b[B_SPRITE] ^= 1;
        }
    }

    pub(super) fn face_mario(&mut self, st: &mut Staging) {
        // $5B3E0: compare sMarioRefX with the object X.
        let obj_x = st.x();
        self.facing = if self.mario_ref_x > obj_x {
            1
        } else if self.mario_ref_x < obj_x {
            0xFF
        } else {
            0
        };
        if self.facing == 1 {
            st.b[B_FSTATE] = 2;
            st.b[B_FLIP] = 0x20;
        } else {
            st.b[B_FSTATE] = 1;
            st.b[B_FLIP] = 0;
        }
    }

    pub(super) fn fall_step(&self, st: &mut Staging) -> u8 {
        let idx = (st.b[B_PHASE] as usize).min(21);
        let b = FALL_SPEED[idx];
        if st.b[B_PHASE] != 21 {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        b
    }

    pub(super) fn bounce_step(&self, st: &mut Staging) -> u8 {
        let idx = (st.b[B_PHASE] as usize).min(15);
        let b = BOUNCE_SPEED[idx];
        if st.b[B_PHASE] != 15 {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        b
    }

    pub(super) fn sprite_toggle1(&self, st: &mut Staging) {
        if self.fff0 & 1 == 0 {
            st.b[B_SPRITE] ^= 0x01;
        }
    }

    pub(super) fn flash_toggle(&self, st: &mut Staging) {
        if self.fff0 & 1 == 0 {
            st.b[B_FLASH] ^= 0x10;
        }
    }

    pub(super) fn hover_step(&self, st: &mut Staging) -> u8 {
        let idx = (st.b[B_PHASE] as usize).min(21);
        let b = HOVER_SPEED[idx];
        if st.b[B_PHASE] != 21 {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        b
    }

    pub(super) fn spin_step(&self, st: &mut Staging) {
        if self.fff0 & 3 != 0 {
            return;
        }
        st.b[B_ANIMFRAME] = st.b[B_ANIMFRAME].wrapping_add(1);
        if st.b[B_ANIMFRAME] == 4 {
            st.b[B_ANIMFRAME] = 0;
            st.b[B_FLIP] &= !0x20;
        }
        st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_add(SPIN_WOBBLE[st.b[B_ANIMFRAME] as usize]);
        if st.b[B_ANIMFRAME] == 3 {
            st.b[B_FLIP] |= 0x20;
        }
    }

    pub(super) fn dist_mario_x(&self, st: &Staging) -> u8 {
        let d = (self.mario_ref_x as u8).wrapping_sub(st.b[B_XLO]);
        if d & 0x80 != 0 {
            d.wrapping_neg()
        } else {
            d
        }
    }

    pub(super) fn mv_x_add(&mut self, st: &mut Staging, b: u8) {
        let v = st.x().wrapping_add(b as u16);
        set_x(&mut st.b, v);
    }

    pub(super) fn mv_x_sub(&mut self, st: &mut Staging, b: u8) {
        let v = st.x().wrapping_sub(b as u16);
        set_x(&mut st.b, v);
    }

    pub(super) fn mv_y_add(&mut self, st: &mut Staging, b: u8) {
        let v = st.y().wrapping_add(b as u16);
        set_y(&mut st.b, v);
    }

    pub(super) fn mv_y_sub(&mut self, st: &mut Staging, b: u8) {
        let v = st.y().wrapping_sub(b as u16);
        set_y(&mut st.b, v);
    }

    pub(super) fn probe_keep(&mut self, st: &Staging, layout: &LevelLayout, limit: u8, flag: u8) {
        if Self::probe(layout, st.px, st.py) < limit {
            return;
        }
        self.move_flags &= !flag;
    }

    pub(super) fn m_ground_left16(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.move_flags = 0x88;
        st.py = st.y();
        if st.b[B_YLO] & 0x0C != 0 {
            self.move_flags &= !0x08;
            return;
        }
        st.px = st.x().wrapping_sub(16);
        self.probe_keep(st, layout, 0x48, 0x08);
    }

    pub(super) fn m_ground_right16(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.move_flags = 0x88;
        st.py = st.y();
        if st.b[B_YLO] & 0x0C != 0 {
            self.move_flags &= !0x08;
            return;
        }
        st.px = st.x().wrapping_add(16);
        self.probe_keep(st, layout, 0x48, 0x08);
    }

    pub(super) fn m_ground_own(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.move_flags = 0x88;
        st.py = st.y();
        if st.b[B_YLO] & 0x0C != 0 {
            self.move_flags &= !0x08;
            return;
        }
        st.px = st.x();
        self.probe_keep(st, layout, 0x48, 0x08);
    }

    pub(super) fn m_ground_below16(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.move_flags = 0x88;
        st.py = st.y().wrapping_add(16);
        if (st.py as u8) & 0x0C != 0 {
            self.move_flags &= !0x08;
            return;
        }
        st.px = st.x();
        self.probe_keep(st, layout, 0x48, 0x08);
    }

    pub(super) fn m_ceil_own(&mut self, st: &mut Staging, layout: &LevelLayout, dist: u8) {
        self.move_flags = 0x44;
        st.py = st.y().wrapping_sub(dist as u16);
        st.px = st.x();
        self.probe_keep(st, layout, 0x38, 0x04);
    }

    pub(super) fn m_ceil_side(&mut self, st: &mut Staging, layout: &LevelLayout, dx: i16) {
        self.move_flags = 0x44;
        st.py = st.y().wrapping_sub(18);
        st.px = if dx < 0 {
            st.x().wrapping_sub(16)
        } else {
            st.x().wrapping_add(16)
        };
        self.probe_keep(st, layout, 0x38, 0x04);
    }

    pub(super) fn wall_probe(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        dy: u8,
        dx: i16,
        flag: u8,
    ) {
        let flags = if flag == 0x01 { 0x11 } else { 0x22 };
        self.move_flags = flags;
        st.py = st.y().wrapping_sub(dy as u16);
        st.px = if dx < 0 {
            st.x().wrapping_sub((-dx) as u16)
        } else {
            st.x().wrapping_add(dx as u16)
        };
        if Self::probe(layout, st.px, st.py) < 0x38 {
            return;
        }
        st.py = st.y().wrapping_sub(5);
        if Self::probe(layout, st.px, st.py) < 0x38 {
            return;
        }
        self.move_flags &= !flag;
    }

    pub(super) fn m_right_wall_low(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 8, 7, 0x01);
    }

    pub(super) fn m_right_wall(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, 7, 0x01);
    }

    pub(super) fn m_right_wall_mid(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, 11, 0x01);
    }

    pub(super) fn m_right_wall_far(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, 19, 0x01);
    }

    pub(super) fn m_right_wall_high(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 19, 11, 0x01);
    }

    pub(super) fn m_left_wall_low(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 8, -7, 0x02);
    }

    pub(super) fn m_left_wall(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, -7, 0x02);
    }

    pub(super) fn m_left_wall_mid(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, -11, 0x02);
    }

    pub(super) fn m_left_wall_far(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, -19, 0x02);
    }

    pub(super) fn m_left_wall_high(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 19, -11, 0x02);
    }
}

/// BCD add (the engine uses `add`/`inc` + `daa`).
pub(super) fn bcd_add(v: u8, add: u8) -> (u8, bool) {
    let mut lo = (v & 0x0F) + (add & 0x0F);
    let mut hi = (v >> 4) + (add >> 4);
    if lo > 9 {
        lo -= 10;
        hi += 1;
    }
    if hi > 9 {
        hi -= 10;
        return ((hi << 4) | lo, true);
    }
    ((hi << 4) | lo, false)
}
