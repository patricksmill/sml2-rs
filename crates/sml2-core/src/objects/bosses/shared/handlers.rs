//! Object boss routines.

use super::*;

impl ObjectSim {
    pub(super) fn boss_damage_pass(&mut self, st: &mut Staging) {
        self.apply_boss_damage(st);
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

    #[cfg(test)]
    pub(super) fn scr_x_sub(&self, st: &mut Staging, b: u8) {
        st.b[B_SCRX] = st.b[B_SCRX].wrapping_sub(b);
    }

    #[cfg(test)]
    pub(super) fn scr_x_add(&self, st: &mut Staging, b: u8) {
        st.b[B_SCRX] = st.b[B_SCRX].wrapping_add(b);
    }

    #[cfg(test)]
    pub(super) fn scr_y_sub(&self, st: &mut Staging, b: u8) {
        st.b[B_SCRY] = st.b[B_SCRY].wrapping_sub(b);
    }

    #[cfg(test)]
    pub(super) fn scr_y_add(&self, st: &mut Staging, b: u8) {
        st.b[B_SCRY] = st.b[B_SCRY].wrapping_add(b);
    }

    #[cfg(test)]
    pub(super) fn sprite_blink(&self, st: &mut Staging, mask: u8, xor: u8) {
        if self.fff0 & mask == 0 {
            st.b[B_SPRITE] ^= xor;
        }
    }

    pub(super) fn boss_spawn_child(&mut self, st: &Staging) {
        self.apply_boss_spawn(st);
    }

    #[cfg(test)]
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

    pub(super) fn boss_defeat_fall(&mut self, st: &mut Staging) {
        st.b[B_FSTATE] = 2;
        st.b[B_T28] = 0x20;
        st.b[B_SPRITE] = self.boss_sprite;
    }

    pub(super) fn boss_morph_to_anim3(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let morphed = boss_morph::update(&mut actor, crate::actor::FrameTick(u64::from(self.fff0)));
        st.b = actor.to_raw_slot();
        if morphed && self.boss_slot_ptr_lo == 0 {
            self.music_request = Some(0xFF);
        }
    }

    #[cfg(test)]
    pub(super) fn boss_hit_stun(&mut self, st: &mut Staging) {
        self.apply_boss_hit_stun(st);
    }

    pub fn boss_engine(
        &mut self,
        cam_x: u16,
        cam_y: u16,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        // Bank 1 $54A7 projects the player before the boss pass; $A23C
        // retains the wrapped low byte even across world-coordinate pages.
        self.player_screen_x = self.mario.x.wrapping_sub(cam_x).wrapping_add(96) as u8;
        // $54C8 stores $A23B before $54E9 applies pan only to draw staging.
        self.player_screen_y = self.mario.y.wrapping_sub(cam_y).wrapping_add(98) as u8;
        // $8DA9's head: `ld hl,$FFF0 / inc [hl]` — unconditional, unlike
        // `$8000`'s phase-toggle increment (so the odd/even slot parity flips
        // every frame here).
        self.fff0 = self.fff0.wrapping_add(1);
        self.odd = self.fff0 & 1 == 1;
        // The frame's OAM output is rebuilt from scratch: `$66000` writes into
        // the shadow OAM through `$FF8D`, which the frame's own draw reset.
        self.boss_oam.clear();
        self.boss_draws.clear();

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
        let draw = BossDraw {
            sprite: st.b[B_SPRITE],
            x: st.b[B_SCRX],
            y: st.b[B_SCRY],
            flags,
        };
        self.boss_draws.push(draw);
        expand_boss_list(rec, draw, &mut self.boss_oam);
    }
}

#[cfg(test)]
#[path = "tests/fanfare.rs"]
mod fanfare_tests;
