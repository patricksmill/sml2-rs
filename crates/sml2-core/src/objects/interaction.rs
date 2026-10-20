//! Object interaction routines.

use super::*;

impl ObjectSim {
    pub(super) fn interaction_dispatch(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let f = st.b[B_FLAGS];
        if f == 0 {
            return;
        }
        if f & 0x01 != 0 {
            self.dispatch_stomp(st, tables);
            return;
        }
        if f & 0x02 != 0 {
            if self.sfx0 != 9 {
                self.sfx0 = 6;
            }
            self.defeat(st);
            return;
        }
        if f & 0x04 != 0 {
            if self.sfx0 != 9 {
                self.sfx0 = 16;
            }
            self.defeat(st);
            return;
        }
        if f & 0x10 != 0 {
            // $8611: touched. A bit-7 type goes to the `$868A` cluster, which is
            // a switch on the type: `$86`-`$88` and `$8A`+ do **nothing at all**
            // (`cp $86 / ret nc`), while `$80`-`$85` and `$89` switch the anim
            // (`$FFD5` = slot +5) to 16, clear the flip byte and run `$863C`
            // (`$8240` sprite re-derive + `sObjectDone = 1`, i.e. the object
            // stops for this frame) — the slot is now an anim-16 pickup and the
            // next frames' `$586F5` handler plays the rise-and-despawn. The
            // order differs by type: `$85` (`$86DA`) writes the anim *before*
            // `$863C`, so its sprite is the anim-16 one; the rest (`$8695`) run
            // `$863C` first and write the anim after, so their sprite stays the
            // **old** anim's — the anim-16 handler never re-derives it. Either
            // way `$8240` clears `+14..+30` (the rise counter, the state gate
            // byte and the interaction flags) before the copy-back. The type
            // `$06` carry path has its own arm.
            let ty = st.b[B_TYPE];
            if ty & 0x80 != 0 {
                if (0x86..=0x88).contains(&ty) || ty >= 0x8A {
                    return;
                }
                st.b[B_FLIP] = 0;
                if ty == 0x85 {
                    st.b[B_DEF] = 16;
                    self.finish_sprite_swap(st, tables);
                } else {
                    self.derive_sprite(st, tables);
                    st.b[B_DEF] = 16;
                    self.done = 1;
                }
                return;
            }
            if ty != 0x06 {
                return;
            }
            // `$8611`'s type-`$06` arm — the carry/kick entry.
            if self.mario.b_held {
                // B held: the one-shot latch, then anim += 2 (2 -> 4 = carried)
                // and `$863C`. `$8240` inside it clears `+14..+30`, the type byte
                // included, so the carried object leaves here as (anim 4, type 0,
                // state 0) and the anim-4 handler takes it from the next frame.
                if self.action_latch != 0 {
                    return;
                }
                self.action_latch = 1;
                st.b[B_DEF] = st.b[B_DEF].wrapping_add(2);
                self.finish_sprite_swap(st, tables);
                return;
            }
            // B not held: `$8633` (anim += 1 -> the kicked walker) then the
            // placement tail, which needs the layout the dispatcher does not
            // carry; the caller runs it (see `pending_kick`).
            st.b[B_DEF] = st.b[B_DEF].wrapping_add(1);
            self.pending_kick = true;
            return;
        }
        if f & 0x20 != 0 {
            self.dispatch_below(st, tables);
        }
    }

    pub(super) fn dispatch_stomp(&mut self, st: &mut Staging, tables: &ObjectTables) {
        st.b[B_FSTATE] = 0;
        if self.mario.spin_jump {
            self.defeat(st);
            return;
        }
        let ty = st.b[B_TYPE];
        if ty & 0x02 == 0 {
            self.defeat(st);
            return;
        }
        if ty & 0x01 == 0 {
            // $8633: sfx 3; anim + 1; sprite re-derive.
            self.sfx0 = 3;
            st.b[B_DEF] = st.b[B_DEF].wrapping_add(1);
            self.finish_sprite_swap(st, tables);
        } else {
            // $85D7/$85DC: sfx 3; anim − 1.
            self.sfx0 = 3;
            st.b[B_DEF] = st.b[B_DEF].wrapping_sub(1);
            self.finish_sprite_swap(st, tables);
        }
    }

    pub(super) fn defeat(&mut self, st: &mut Staging) {
        self.money_bag_add();
        st.b[B_DEF] = 0;
        st.b[B_FSTATE] = 0;
        st.b[B_DEFEAT] = 0;
        st.b[B_TYPE] = 0;
        self.done = 1;
        self.clear_stage_flags = true;
    }

    pub(super) fn dispatch_below(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.clear_stage_flags = true;
        let ty = st.b[B_TYPE];
        if ty & 0x80 != 0 {
            // $86E4.
            if ty & 0x7F != 0 {
                // $8721: the type survives the re-derive, then reaction 2.
                let saved_type = ty;
                self.finish_sprite_swap(st, tables);
                st.b[B_TYPE] = saved_type;
                st.b[B_HITREACT] = 2;
                return;
            }
            // $86E4's fall-through — the `$80` item pose.
            if st.b[B_DEF] == 0x11 && self.mario.powerup != 0 {
                self.record_set_anim(st, 18);
                st.b[B_DEF] = 18;
                st.b[B_SPRITE] = 26;
            }
            // $8708: type = anim + $70, frame state 1, Y += 16, marker $10.
            st.b[B_TYPE] = st.b[B_DEF].wrapping_add(0x70);
            st.b[B_FSTATE] = 1;
            self.mv_y_add(st, 16);
            st.b[B_MARKER] = 0x10;
            return;
        }
        st.b[B_FSTATE] = 0;
        if ty & 0x10 != 0 {
            // $86C6.
            if ty & 0x08 != 0 {
                // $86D2 → $85D7/$85DC: sfx 3, anim − 1, sprite re-derive.
                st.b[B_DEF] = st.b[B_DEF].wrapping_sub(1);
                self.finish_sprite_swap(st, tables);
                st.b[B_HITREACT] = 1;
                return;
            }
            st.b[B_DEF] = st.b[B_DEF].wrapping_add(1);
            self.finish_sprite_swap(st, tables);
            st.b[B_HITREACT] = 1;
            return;
        }
        if ty & 0x08 != 0 {
            self.defeat(st);
            return;
        }
        self.finish_sprite_swap(st, tables);
        st.b[B_HITREACT] = 1;
    }

    pub fn below_bonk(&mut self, bonk_y: u8, bonk_x: u8, scroll_y: u8, scroll_x: u8) -> bool {
        let by = (bonk_y & 0xF0)
            .wrapping_add(8)
            .wrapping_sub(scroll_y)
            .wrapping_sub(16);
        let bx = bonk_x.wrapping_add(8) & 0xF0;
        let bx = bx.wrapping_sub(scroll_x).wrapping_sub(8);
        let mut special = false;
        for s in self.slots.iter_mut() {
            if s.b[B_FLAGS] != 0 {
                continue;
            }
            let ty = s.b[B_TYPE];
            if ty == 0 {
                continue;
            }
            if s.b[B_SCRY].wrapping_sub(by) >= 0x10 {
                continue;
            }
            if s.b[B_SCRX].wrapping_sub(bx) >= 0x14 {
                continue;
            }
            s.b[B_FLAGS] |= 0x20;
            if ty == 0x80 {
                special = true;
            }
        }
        special
    }

    pub fn fire_hit(&mut self, sy: u8, sx: u8) -> FireHit {
        let mut result = FireHit::None;
        for s in self.slots.iter_mut() {
            if s.b[B_FLAGS] != 0 {
                continue;
            }
            let ty = s.b[B_TYPE];
            if ty == 0 || ty & 0x80 != 0 {
                continue;
            }
            let oy = s.b[B_SCRY];
            let y0 = s.b[B_HB_TOP].wrapping_sub(6).wrapping_add(oy);
            let h = s.b[B_HB_BOTTOM]
                .wrapping_add(6)
                .wrapping_add(oy)
                .wrapping_sub(y0);
            if sy.wrapping_sub(y0) >= h {
                continue;
            }
            let ox = s.b[B_SCRX];
            let x0 = s.b[B_HB_LEFT].wrapping_sub(6).wrapping_add(ox);
            let w = s.b[B_HB_RIGHT]
                .wrapping_add(6)
                .wrapping_add(ox)
                .wrapping_sub(x0);
            if sx.wrapping_sub(x0) >= w {
                continue;
            }
            if ty & 0x04 != 0 {
                s.b[B_FLAGS] |= 0x02;
                result = FireHit::Defeat;
            } else {
                result = FireHit::Die;
            }
        }
        result
    }

    pub(super) fn collide(&mut self, cam_x: u16, cam_y: u16, layout: &LevelLayout) {
        self.bounce = false;
        self.flutter_disarmed = false;
        self.stomp_jump = false;
        self.stomp_bounce = false;
        self.on_object = false;
        self.on_ground_cleared = false;
        self.mario_x_adjust = None;
        self.mario_y_adjust = None;
        if self.mario_dead != 0 {
            return;
        }
        let mario_sx = self.mario.x.wrapping_sub(cam_x).wrapping_add(96) as u8;
        let mario_sy = self.mario.y.wrapping_sub(cam_y).wrapping_add(98) as u8;
        for i in 0..SLOT_COUNT {
            if self.slots[i].b[B_FLAGS] & 0xF7 != 0 {
                continue;
            }
            // $2EDD: type gate.
            let ty = self.slots[i].b[B_TYPE];
            if ty & 0x80 != 0 {
                if ty & 0x7F == 0 {
                    continue;
                }
            } else if ty & 0x63 == 0 {
                continue;
            }
            // $2EF5: box test against the slot's screen position.
            let b = &self.slots[i].b;
            let obj_y = b[B_SCRY];
            let obj_x = b[B_SCRX];
            let mut e = if self.mario.powerup != 0 { 8u8 } else { 0 };
            if self.mario.ducking {
                e = if self.mario.powerup != 0 { 0 } else { 0xF8 };
            }
            let d = b[B_HB_TOP].wrapping_add(obj_y).wrapping_sub(20);
            let span = b[B_HB_BOTTOM]
                .wrapping_add(obj_y)
                .wrapping_add(e)
                .wrapping_sub(d);
            let y_delta = mario_sy.wrapping_sub(d);
            if y_delta >= span {
                continue;
            }
            let d = b[B_HB_LEFT]
                .wrapping_add(obj_x)
                .wrapping_add(11)
                .wrapping_sub(16);
            let span = b[B_HB_RIGHT]
                .wrapping_add(obj_x)
                .wrapping_add(20)
                .wrapping_sub(16)
                .wrapping_sub(d);
            if mario_sx.wrapping_sub(d) >= span {
                continue;
            }
            self.respond(i, y_delta, layout);
        }
        if self.bounce {
            // $2D7D tail: turn the stomp into a jump (75 with A held).
            self.stomp_jump = true;
        }
    }

    pub(super) fn respond(&mut self, i: usize, y_delta: u8, layout: &LevelLayout) {
        self.on_ground_cleared = true;
        self.slots[i].b[B_FLAGS] |= 0x10;
        let ty = self.slots[i].b[B_TYPE];
        if ty & 0x80 != 0 {
            // $2DC6 tail: `and $7F; ld [sPendingItemId], a; ret` — the id is
            // consumed by `$3451` at the tail of the next frame's player update.
            // (Note the engine does *not* clear the byte here, and `$3451`
            // itself returns without clearing for id 0, so a `$80` object — the
            // levitating item pose, the bobber at rest — has no effect at all.)
            self.pending_item = ty & 0x7F;
            return;
        }
        if y_delta >= 0x0C || (self.mario.flutter == 0 && self.mario.vvel >= 0x18) {
            self.harm_or_star(i);
            return;
        }
        let a = ty & 0x23;
        if a == 0 {
            self.harm_or_star(i);
            return;
        }
        if a & 0x20 != 0 {
            // Stompable-on-top: stand on it (nudge Mario up 1 px).
            self.on_object = true;
            self.slots[i].b[B_FLAGS] = 8;
            if y_delta < 3 {
                return;
            }
            self.mario_y_adjust = Some(self.mario.y.wrapping_sub(1));
            return;
        }
        // $2E23: clear the flutter latch before the star test. A star still
        // harms, but the latch is already down.
        self.flutter_disarmed = true;
        if self.mario.star_high != 0 {
            self.harm_or_star(i);
            return;
        }
        self.slots[i].b[B_FLAGS] = 1;
        let b = y_delta.wrapping_add(6);
        let y_new = self.mario.y.wrapping_sub(b as u16);
        self.mario_y_adjust = Some(y_new);
        self.bounce = true;
        // $1DFE ceiling check: it probes $FFC0, which the `y -= b` above has
        // already rewritten — so the probe runs from the *shifted* Y, and a
        // hit adds b back.
        let px = self.mario.x.wrapping_add(16);
        let py = y_new.wrapping_add(if self.mario.powerup != 0 { 18 } else { 28 });
        let t = Self::probe_bias(layout, px, py);
        if t == 0x7E || t < 0x38 {
            self.mario_y_adjust = Some(self.mario.y);
        }
        if self.mario.scene_state != 0 {
            self.stomp_bounce = true;
        }
    }

    pub(crate) fn harm_or_star(&mut self, i: usize) {
        if self.mario.star_high != 0 {
            // $2EAA: star defeat (type bit5 → immune).
            if self.slots[i].b[B_TYPE] & 0x20 != 0 {
                return;
            }
            self.slots[i].b[B_FLAGS] = 4;
            self.stomp_bounce_timer = 32;
            let (v, _) = bcd_add(self.stomp_chain, 1);
            self.stomp_chain = v;
            if v >= 5 {
                self.sfx0 = 9;
                self.award_lives = self.award_lives.saturating_add(1);
            }
            return;
        }
        if self.slots[i].b[B_TYPE] & 0x40 == 0 {
            return;
        }
        // $2E7C: moon item shield (bit 2 in sMoonPhysics): consume and grant invincibility
        if self.mario.moon_physics & 0x04 != 0 {
            self.mario.moon_physics &= !0x04;
            self.mario.invincibility = 0x80;
            return;
        }
        if self.mario.invincibility != 0 {
            return;
        }
        self.mario_dead = 15;
        self.sfx0 = if self.mario.powerup != 0 { 20 } else { 15 };
    }
}
