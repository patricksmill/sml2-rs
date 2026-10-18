//! ROM `$96FE` (boss anim 6): the door boss.
//!
//! The shared damage pass runs first. A hit banks the sprite and parks state 1,
//! using sprite 17 when the hit was a stomp. State 0 waits out the fade, counts
//! to `$70`, then drops into state 3 on a random side. The later states flash,
//! morph, spawn the child, count the mouth open, and return to state 0.
use super::*;

impl ObjectSim {
    pub(super) fn handler_0x96fe(&mut self, st: &mut Staging) {
        self.apply_boss_damage(st);
        if self.done != 0 {
            self.spawn_sprite = st.b[B_SPRITE];
            self.saved_frame_state = st.b[B_FSTATE];
            self.saved_type = st.b[B_TYPE];
            if self.boss_hit_latch != 0 {
                st.b[B_SPRITE] = 17;
                st.b[B_T28] = self.boss_timer_reload;
            }
            st.b[B_TYPE] = 0;
            st.b[B_FSTATE] = 1;
            return;
        }
        if st.b[B_FSTATE] == 0 {
            self.wait_for_the_door(st);
            return;
        }
        match st.b[B_FSTATE] {
            1 => self.recover_from_the_hit(st),
            2 => self.boss_morph_to_anim3(st),
            3 => self.open_and_spawn(st),
            4 => self.pause_after_the_spawn(st),
            5 | 6 => self.advance_door_boss_mouth(st),
            7 => st.b[B_FSTATE] = 0,
            _ => self.note_unported(0x96FE),
        }
    }

    fn wait_for_the_door(&mut self, st: &mut Staging) {
        if !self.fade_idle() {
            return;
        }
        st.b[B_T28] = st.b[B_T28].wrapping_add(1);
        if st.b[B_T28] != 0x70 {
            return;
        }
        st.b[B_T28] = 0;
        st.b[B_FSTATE] = 3;
        st.b[B_ANIMFRAME] = 8;
        self.place_door(st);
        self.sfx1 = 2;
    }

    fn place_door(&mut self, st: &mut Staging) {
        st.b[B_SCRY] = 112;
        if self.div & 3 != 0 {
            self.door_parity ^= 1;
        }
        if self.door_parity == 0 {
            st.b[B_SCRX] = 152;
            st.b[B_FLIP] = 0;
        } else {
            st.b[B_SCRX] = 24;
            st.b[B_FLIP] = 0x20;
        }
    }

    fn recover_from_the_hit(&mut self, st: &mut Staging) {
        self.boss_flash_b40e(st);
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
        st.b[B_SPRITE] = 20;
        st.b[B_FSTATE] = 5;
        st.b[B_TYPE] = 0;
    }

    fn open_and_spawn(&mut self, st: &mut Staging) {
        bob_mouth(st, self.fff0);
        if st.b[B_ANIMFRAME] != 0 {
            return;
        }
        st.b[B_FSTATE] = 4;
        st.b[B_SPRITE] = 16;
        st.b[B_TYPE] = 69;
        st.b[B_T28] = 112;
        self.door_phase = self.div & 3;
        self.spawn_anim = 9;
        self.spawn_sprite = 26;
        self.spawn_attr = st.b[B_FLIP];
        self.boss_spawn_child(st);
        self.sfx2 = 6;
    }

    fn pause_after_the_spawn(&mut self, st: &mut Staging) {
        st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
        if st.b[B_T28] != 0 {
            return;
        }
        st.b[B_SPRITE] = 20;
        st.b[B_FSTATE] = 5;
        st.b[B_TYPE] = 0;
    }

    fn advance_door_boss_mouth(&mut self, st: &mut Staging) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        door_boss_mouth::update(&mut actor, crate::actor::FrameTick(u64::from(self.fff0)));
        st.b = actor.to_raw_slot();
    }
}

fn bob_mouth(st: &mut Staging, frame: u8) {
    let low = frame & 7;
    if low == 0 {
        st.b[B_ANIMFRAME] = st.b[B_ANIMFRAME].wrapping_sub(1);
        st.b[B_SPRITE] &= !1;
        return;
    }
    if low != st.b[B_ANIMFRAME] {
        return;
    }
    st.b[B_SPRITE] ^= 1;
}

#[cfg(test)]
#[path = "handler_0x96FE_tests.rs"]
mod tests;
