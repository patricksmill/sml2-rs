//! Object boss routines.

use super::*;

impl ObjectSim {
    pub(super) fn boss_damage_pass(&mut self, st: &mut Staging) {
        let flags = st.b[B_FLAGS];
        let stomp = flags & 0x01 != 0;
        let other = flags & 0x02 != 0;
        if !stomp && !other {
            return;
        }
        self.done = 1;
        if stomp {
            self.boss_hit_latch = 1;
        }
        if self.boss_slot_ptr_lo != 0 {
            return; // not the primary slot
        }
        if stomp {
            self.sfx0 = 0x13;
        } else {
            self.sfx1 = 9;
        }
        let n = if stomp { 2 } else { 1 };
        let (hits, borrow) = self.boss_hits_remaining.overflowing_sub(n);
        self.boss_hits_remaining = hits;
        if hits != 0 && !borrow {
            return;
        }
        // $92C4: arm removal, and for defs 0-5 the boss-clear fade.
        self.remove_mode = 1;
        if self.boss_def_index >= 6 {
            return;
        }
        self.fade_timer_set = Some(0x0280);
        self.boss_clear_flag = 1;
    }

    pub(super) fn fade_idle(&self) -> bool {
        let t = self.fade_timer;
        ((t & 0xFF) | ((t >> 8) & 0xFF)) as u8 == 0
    }

    pub(super) fn boss_finish_blank(&mut self, st: &mut Staging) {
        st.b = [0; 32];
        self.done = 1;
    }

    pub(super) fn boss_finish_counted(&mut self, st: &mut Staging) {
        self.boss_def_count = self.boss_def_count.wrapping_sub(1);
        self.boss_finish_blank(st);
    }

    pub(super) fn scr_x_sub(&self, st: &mut Staging, b: u8) {
        st.b[B_SCRX] = st.b[B_SCRX].wrapping_sub(b);
    }

    pub(super) fn scr_x_add(&self, st: &mut Staging, b: u8) {
        st.b[B_SCRX] = st.b[B_SCRX].wrapping_add(b);
    }

    pub(super) fn scr_y_sub(&self, st: &mut Staging, b: u8) {
        st.b[B_SCRY] = st.b[B_SCRY].wrapping_sub(b);
    }

    pub(super) fn scr_y_add(&self, st: &mut Staging, b: u8) {
        st.b[B_SCRY] = st.b[B_SCRY].wrapping_add(b);
    }

    pub(super) fn sprite_blink(&self, st: &mut Staging, mask: u8, xor: u8) {
        if self.fff0 & mask == 0 {
            st.b[B_SPRITE] ^= xor;
        }
    }

    pub(super) fn boss_spawn_child(&mut self, st: &Staging) {
        let n = self.boss_def_count.wrapping_add(1);
        if n == 0x10 || n as usize >= SLOT_COUNT {
            return;
        }
        let child = n as usize;
        {
            let s = &mut self.slots[child];
            s.b[B_DEF] = self.spawn_anim;
            s.b[6..11].copy_from_slice(&st.b[6..11]);
            s.b[B_SPRITE] = self.spawn_sprite;
            s.b[B_FLIP] = self.spawn_attr;
        }
        self.boss_def_count = n;
        // $B259: the spawn staging is cleared for the next caller.
        self.spawn_anim = 0;
        self.spawn_sprite = 0;
        self.spawn_attr = 0;
    }

    pub(super) fn boss_hit_save(&mut self, st: &mut Staging) {
        self.spawn_sprite = st.b[B_SPRITE];
        self.saved_frame_state = st.b[B_FSTATE];
        self.saved_type = st.b[B_TYPE];
        if self.boss_hit_latch != 0 {
            st.b[B_SPRITE] = 0;
            st.b[B_T28] = self.boss_timer_reload;
        }
        st.b[B_TYPE] = 0;
        st.b[B_FSTATE] = 1;
    }

    pub(super) fn boss_type0_state1(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = 0;
        st.b[B_FSTATE] = 1;
    }

    pub(super) fn boss_defeat_fall(&mut self, st: &mut Staging) {
        st.b[B_FSTATE] = 2;
        st.b[B_T28] = 0x20;
        st.b[B_SPRITE] = self.boss_sprite;
    }

    pub(super) fn boss_morph_to_anim3(&mut self, st: &mut Staging) {
        self.boss_flash_b40e(st); // $9443 -> $B40E
                                           // The ROM decrements first and tests the NEW value (`dec a / ldh
                                           // [$FFEC],a / ret nz`),
                                           // so a 1 falls through to the hand-off on that same call.
        st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
        if st.b[B_T28] != 0 {
            return;
        }
        st.b[B_FSTATE] = 0;
        st.b[B_DEF] = 3;
        st.b[B_ANIMFRAME] = 0;
        st.b[B_FLASH] = 0;
        if self.boss_slot_ptr_lo == 0 {
            self.music_request = Some(0xFF);
        }
    }

    pub(super) fn handler_boss_def0_main(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let _ = (layout, tables);
        self.boss_damage_pass(st); // $93A7
        if self.done != 0 {
            self.boss_hit_save(st); // $93AA
            return;
        }
        if st.b[B_FSTATE] == 0 {
            // $93D5: the intro. Blink, then wait out the boss-clear fade before
            // arming the fight.
            self.sprite_blink(st, 0x03, 0x06); // $7309
            if !self.fade_idle() {
                return; // $714E: `ret nz` while a fade runs
            }
            st.b[B_TYPE] = 0x45;
            st.b[B_SPRITE] = 3;
            st.b[B_FSTATE] = 3;
            return;
        }
        // $93EE: dec + `rst $28` over $93F0's seven targets.
        match st.b[B_FSTATE] {
            2 => self.boss_morph_to_anim3(st), // $9443
            s => self.note_unported(0x93EF + (s as u32) * 2),
        }
    }

    pub(super) fn flip_blink(&self, st: &mut Staging) {
        if self.fff0 & 0x0F == 0 {
            st.b[B_FLIP] ^= 0x20;
        }
    }

    pub(super) fn sprite_cycle8(&self, st: &mut Staging) {
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

    pub(super) fn boss_hp_hit(&mut self, st: &mut Staging) {
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

    pub(super) fn boss_hit_stun(&mut self, st: &mut Staging) {
        self.boss_flash_b40e(st); // $B40E
        if st.b[B_T28] != 0 {
            st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
            if st.b[B_T28] != 0 {
                return;
            }
        }
        if self.remove_mode != 0 {
            self.boss_defeat_fall(st);
            return;
        }
        st.b[B_FLASH] = 0;
        st.b[B_SPRITE] = self.spawn_sprite;
        st.b[B_FSTATE] = self.saved_frame_state;
        st.b[B_TYPE] = self.saved_type;
    }

    pub(super) fn boss_fade_done(&mut self, st: &mut Staging) {
        st.b[B_SPRITE] = 0x20;
        st.b[B_T28] = 0;
        st.b[B_FSTATE] = 3;
        st.b[B_TYPE] = 0x45;
        self.sfx1 = 4;
    }

    pub(super) fn boss_bob(&mut self, st: &mut Staging, tables: &ObjectTables) {
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
        st.b[B_FSTATE] = 4;
        st.b[B_FLIP] = 0x20;
    }

    pub(super) fn boss_end_flip(&mut self, st: &mut Staging) {
        st.b[B_FSTATE] ^= 1;
        st.b[B_FLIP] ^= 0x20;
        self.sfx1 = st.b[B_DEF];
    }

    pub(super) fn handler_boss_def2_part(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
        left: bool,
    ) {
        let _ = (layout, tables);
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
            if !left {
                self.boss_fade_done(st); // $9C8B
                return;
            }
            // $9D63: the left part waits until exactly one child is left.
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
                if left {
                    self.scr_x_sub(st, 1);
                    st.b[B_T28] = 8;
                } else {
                    self.scr_x_add(st, 1);
                }
                self.boss_bob(st, tables); // $9CDF
            }
            4 | 5 => {
                let hp = st.b[B_STATE] >> 1;
                let b = 4u8.wrapping_sub(hp);
                // anim 11's state 4 spends a `$28` sprite countdown first.
                if left && st.b[B_T28] != 0 {
                    st.b[B_SPRITE] = if st.b[B_T28] == 1 { 0x20 } else { 0x28 };
                    st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                    if st.b[B_T28] != 0 {
                        return;
                    }
                }
                if st.b[B_FSTATE] == 4 {
                    self.sprite_cycle8(st); // $B3D7
                    self.scr_x_add(st, b);
                    if st.b[B_SCRX] >= 0xA0 {
                        self.boss_end_flip(st); // $9D3D
                    }
                } else {
                    self.sprite_cycle8(st);
                    self.scr_x_sub(st, b);
                    if st.b[B_SCRX] < 0x10 {
                        self.boss_end_flip(st);
                    }
                }
            }
            s => self.note_unported(0x9CA1 + (s as u32) * 2),
        }
    }

    pub(super) fn handler_boss_def2_controller(
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
                self.boss_bob(st, tables);
            }
            s => self.note_unported(0x9E5C + (s as u32) * 2),
        }
    }

    pub(super) fn handler_boss_anim3(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_FSTATE] == 0 {
            self.scr_y_sub(st, 32); // $B270
            st.b[B_FLIP] ^= 0x40; // $FFDD ^= $40
            st.b[B_PHASE] = 0; // $FFE0
            st.b[B_TYPE] = 0; // $FFE2
            st.b[B_FSTATE] = 1;
            // `sSfxRequest0`: 13, or 23 for a slot that is not the primary one.
            self.sfx0 = if self.boss_slot_ptr_lo == 0 { 13 } else { 23 };
            return;
        }
        match st.b[B_FSTATE] {
            // $5354
            1 => {
                let (b, c) = self.step_picker(st, &tables.boss_picker_profile_15, 15);
                self.scr_y_sub(st, b);
                if st.b[B_PHASE] != c {
                    return;
                }
                st.b[B_FSTATE] = 2;
                st.b[B_PHASE] = 0;
            }
            // $5366
            2 => {
                let (b, _) = self.step_picker(st, &tables.boss_picker_profile, 21);
                self.scr_y_add(st, b);
                if st.b[B_SCRY] < 0xD0 {
                    return;
                }
                self.remove_mode = 2;
                if self.boss_slot_ptr_lo != 0 {
                    self.boss_finish_counted(st); // $9399 -> $B157
                    return;
                }
                st.b[B_FSTATE] = 3;
                st.b[B_T28] = 208;
                self.music_request = Some(12);
            }
            // $5385
            3 => {
                st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                if st.b[B_T28] != 0 {
                    return;
                }
                self.boss_finish_blank(st); // $B15B
            }
            s => self.note_unported(0x5354 + (s as u32) * 2),
        }
    }

    pub(super) fn handler_boss_def2(&mut self, st: &mut Staging) {
        if st.b[B_FSTATE] != 0 {
            return; // $9606
        }
        if self.remove_mode != 2 {
            return; // $960A
        }
        st.b[B_TYPE] = 0; // $9610
        st.b[B_FSTATE] = 1; // $9613
        self.spawn_anim = 4; // $9617
        self.spawn_sprite = 9; // $961C
        self.boss_spawn_child(st); // $9621
        st.b[B_SPRITE] = 7; // $9624
    }

    pub fn boss_engine(
        &mut self,
        _cam_x: u16,
        _cam_y: u16,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        // $8DA9's head: `ld hl,$FFF0 / inc [hl]` — unconditional, unlike
        // `$8000`'s phase-toggle increment (so the odd/even slot parity flips
        // every frame here).
        self.fff0 = self.fff0.wrapping_add(1);
        self.odd = self.fff0 & 1 == 1;
        // The frame's OAM output is rebuilt from scratch: `$66000` writes into
        // the shadow OAM through `$FF8D`, which the frame's own draw reset.
        self.boss_oam.clear();

        let mut remaining = self.boss_def_count.wrapping_add(1);
        let mut page = 0xAD00usize;
        loop {
            // $8DBE: `xor a` -> sObjectDone, sBossHitLatch, $5290.
            self.done = 0;
            self.boss_hit_latch = 0;
            self.boss_5290 = 0;
            let i = (page & 0xFFF0).saturating_sub(0xAD00) / 0x20;
            if i >= SLOT_COUNT {
                return;
            }
            let mut st = Staging {
                b: self.slots[i].b,
                ..Default::default()
            };
            self.boss_slot_ptr_lo = (i * 0x20) as u8; // `sObjectSlotPtrLo`
            if st.b[B_DEF] != 0 {
                // $92DF: `ld a,[sMarioDead] / cp $FF / ret z` — a dead Mario
                // freezes *every* boss slot: the handler is not called at all,
                // so the staged record keeps its last values. The driver still
                // renders it and writes it back (the `ret z` returns to the
                // driver, not out of it), which is why a frozen house stays on
                // screen instead of vanishing. Console-verified: on the level
                // 13 tape every slot record stops dead on the frame Mario dies
                // and holds that value for the rest of the death animation.
                if self.mario.dead != 0xFF {
                    // $92DF: `rst $28` through the *boss* table ($92E8) — bank-2
                    // addresses, see `ObjectTables::boss_anim_handlers` — not the
                    // level's bank-22 profiles; see `dispatch_handler`.
                    match tables.boss_anim_handler(st.b[B_DEF]) {
                        Some(h) => self.dispatch_handler(h, &mut st, layout, tables),
                        // The anim is past the table: the ROM would read the next
                        // routine's data, so report it rather than guess.
                        None => self.note_unported(0x92DF),
                    }
                }
                self.boss_render(&st, tables); // $3A14 -> bank 25 $66000
                st.b[B_FLAGS] = 0; // $FFE7 = 0
                self.refresh_hitbox(&mut st, tables, true); // $9048
                                                            // NO `$C090` screen staging here: in the boss path
                                                            // `B_SCRY`/`B_SCRX` are the object's *screen* position, written
                                                            // by the def image itself (def 0's `$AD05` image carries
                                                            // $7C/$58 = `B_SCRY`/`B_SCRX`) and then moved by the handlers'
                                                            // `$B264`/`$B26A`/`$B270`/`$B276`; `$66000` consumes them as-is.
                                                            // Deriving them from world coords here would clobber every boss
                                                            // that moved. `sSpriteFlipFlags` is written *inside* the render
                                                            // pass (`$6087`), which is also the ROM's order.
                self.slots[i].b = st.b; // $81BD
            }
            remaining = remaining.wrapping_sub(1);
            if remaining == 0 {
                return;
            }
            page = page.wrapping_add(0x20);
        }
    }

    pub(super) fn boss_render(&mut self, st: &Staging, tables: &ObjectTables) {
        // $66000's head: `ld a,[sObjectDone] / and a / ret nz` — a slot whose
        // handler marked it done this frame (a hit books `sObjectDone`) is not
        // drawn on that frame.
        if self.done != 0 {
            return;
        }
        // $66087/$6087: `sSpriteFlipFlags = $FFDC ^ $FFDD ^ $FFDE`.
        let flags = st.b[B_DEFEAT] ^ st.b[B_FLIP] ^ st.b[B_FLASH];
        self.sprite_flip_flags = flags;
        let Some(rec) = tables.boss_oam_list(st.b[B_SPRITE]) else {
            // Past the table the ROM reads the following record bytes as a
            // pointer; report rather than serve garbage.
            self.note_unported(0x66092);
            return;
        };
        let base_x = st.b[B_SCRX];
        let mut base_y = st.b[B_SCRY];
        let mut i = 0usize;
        while i < rec.len() {
            let first = rec[i];
            if first == 0x80 {
                break; // $66039: `cp $80 / ret z`
            }
            if first == 0x88 {
                if i + 1 >= rec.len() {
                    break;
                }
                if flags & 0x40 != 0 {
                    base_y = base_y.wrapping_add(rec[i + 1]);
                }
                i += 2;
                continue;
            }
            if i + 3 >= rec.len() {
                break;
            }
            let (dy, dx, tile, attr) = (rec[i], rec[i + 1], rec[i + 2], rec[i + 3]);
            i += 4;
            let y = if flags & 0x40 != 0 {
                (!dy).wrapping_sub(7).wrapping_add(base_y)
            } else {
                dy.wrapping_add(base_y)
            };
            if y >= 0xA0 {
                continue; // $6605F: off the screen vertically — the cell is skipped
            }
            let x = if flags & 0x20 != 0 {
                (!dx).wrapping_sub(7).wrapping_add(base_x)
            } else {
                dx.wrapping_add(base_x)
            };
            if x >= 0xA8 {
                // $66065: `dec l / ld [hl],0` — the Y byte is zeroed and the
                // other three OAM bytes are left as they were. A fresh entry
                // draws nothing either way, so the port keeps the record's
                // remaining bytes rather than inventing stale OAM.
                self.boss_oam.push(BossOamCell {
                    y: 0,
                    x,
                    tile,
                    attr,
                });
                continue;
            }
            self.boss_oam.push(BossOamCell {
                y,
                x,
                tile,
                attr: attr ^ flags,
            });
        }
    }
}
