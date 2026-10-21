//! Engine-level slot operations shared by every handler adapter: the tile
//! probes behind the `move_flags` results, sprite re-derivation, slot removal
//! with its record bookkeeping, and BCD arithmetic.
//!
//! This is the boundary that stays slot-based. Actor behaviour (motion curves,
//! facing, animation, movement) lives in the native modules over `Actor`;
//! adapters reach these operations only after converting the actor back to
//! its slot bytes.

use super::*;

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
                self.add_money_bag_reward();
            }
        }
        st.b = [0; 32];
        self.refresh_hitbox(st, tables, false);
        self.done = 2;
        self.remove_mode = 0;
    }

    pub(super) fn probe_keep(&mut self, st: &Staging, layout: &LevelLayout, limit: u8, flag: u8) {
        if Self::probe(layout, st.px, st.py) < limit {
            return;
        }
        self.move_flags &= !flag;
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

    pub(super) fn wall_probe(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        dy: u8,
        retry_dy: u8,
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
        st.py = st.y().wrapping_sub(retry_dy as u16);
        if Self::probe(layout, st.px, st.py) < 0x38 {
            return;
        }
        self.move_flags &= !flag;
    }

    /// Bank 2 `$841A`.
    pub(super) fn m_right_wall_low(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 8, 5, 7, 0x01);
    }

    /// Bank 2 `$843C`.
    pub(super) fn m_right_wall(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, 5, 7, 0x01);
    }

    /// Bank 2 `$84A6`.
    pub(super) fn m_right_wall_high(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 19, 4, 11, 0x01);
    }

    /// Bank 2 `$84C8`.
    pub(super) fn m_left_wall_low(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 8, 5, -7, 0x02);
    }

    /// Bank 2 `$84EA`.
    pub(super) fn m_left_wall(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, 5, -7, 0x02);
    }

    /// Bank 2 `$8554`.
    pub(super) fn m_left_wall_high(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 19, 4, -11, 0x02);
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
