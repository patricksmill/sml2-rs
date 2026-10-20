//! Object items routines.

use super::*;

impl ObjectSim {
    pub(super) fn handler_itempose20(&mut self, st: &mut Staging) {
        self.flash_toggle(st);
        self.itempose_body(st);
    }

    pub(super) fn itempose_body(&mut self, st: &mut Staging) {
        match st.b[B_FSTATE] {
            1 => {
                // $58787
                if self.fff0 & 1 != 0 {
                    return;
                }
                self.mv_y_sub(st, 1);
                st.b[B_T24] = st.b[B_T24].wrapping_add(1);
                if st.b[B_T24] != 8 {
                    return;
                }
                st.b[B_FSTATE] = 2;
                st.b[B_DEFEAT] = 0;
            }
            2 => {}
            _ => {
                st.b[B_TYPE] = 0x80;
                if st.b[B_MARKER] == 0x10 {
                    // $58777 — the activated pose.
                    st.b[B_TYPE] = st.b[B_DEF].wrapping_add(0x70);
                    st.b[B_FSTATE] = 2;
                    self.mv_y_add(st, 8);
                    return;
                }
                st.b[B_DEFEAT] = 0x80;
                self.done = 1;
                st.b[B_FSTATE] = 2;
            }
        }
    }

    pub(super) fn handler_pose86(&mut self, st: &mut Staging) {
        if st.b[B_FSTATE] == 1 {
            self.spin_step(st);
            return;
        }
        st.b[B_TYPE] = 0x86;
        if st.b[B_FLAGS] & 0x10 == 0 {
            return;
        }
        st.b[B_FSTATE] = 1;
        st.b[B_TYPE] = 0;
    }

    pub(super) fn handler_pose87(&mut self, st: &mut Staging, tables: &ObjectTables) {
        match st.b[B_FSTATE] {
            1 => self.spin_step(st),
            2 => {
                if st.b[B_FLAGS] & 0x10 == 0 {
                    return;
                }
                st.b[B_FSTATE] = 1;
                st.b[B_TYPE] = 0;
            }
            _ => {
                if self.secret_route {
                    // $587FB
                    self.remove_mode = 2;
                    self.remove_object(st, tables);
                    return;
                }
                st.b[B_TYPE] = 0x87;
                st.b[B_FSTATE] = 2;
            }
        }
    }

    pub(super) fn handler_marker85(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = 0x85;
    }

    pub(super) fn handler_marker66(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = st.b[B_DEF].wrapping_add(0x66);
    }

    pub(super) fn handler_pickup16(&mut self, st: &mut Staging, tables: &ObjectTables) {
        st.b[B_FSTATE] = 1;
        self.flash_toggle(st);
        self.mv_y_sub(st, 2);
        st.b[B_T24] = st.b[B_T24].wrapping_add(2);
        if st.b[B_T24] != 0x20 {
            return;
        }
        self.remove_mode = 2;
        self.remove_object(st, tables);
    }

    pub(super) fn handler_squashed(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let effect = squashed::update(&mut actor);
        st.b = actor.to_raw_slot();
        if let squashed::SquashedEffect::Despawn = effect {
            self.remove_mode = 1;
            self.remove_object(st, tables);
        }
    }

    pub(super) fn handler_grower92(&mut self, st: &mut Staging) {
        match st.b[B_FSTATE] {
            1 => {
                // $71E4: b = 1; `+16` picks the direction.
                let b = 1;
                if st.b[B_PHASE] == 0 {
                    self.mv_y_sub(st, b);
                } else {
                    self.mv_y_add(st, b);
                }
                st.b[B_T24] = st.b[B_T24].wrapping_sub(b);
                if st.b[B_T24] != 0 {
                    return;
                }
                // $71F3: a 3-px segment ended.
                st.b[B_ANIMFRAME] = st.b[B_ANIMFRAME].wrapping_sub(1);
                if st.b[B_ANIMFRAME] == 0 {
                    // $720B: the last segment — harmless from here on.
                    st.b[B_TYPE] = 0;
                    st.b[B_FSTATE] = 2;
                    return;
                }
                st.b[B_T24] = st.b[B_T25];
                st.b[B_PHASE] ^= 1;
            }
            2 => {
                // $7213: sprite id 122 -> 124, one step every 4th frame.
                if self.fff0 & 3 != 0 {
                    return;
                }
                st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_add(1);
                if st.b[B_SPRITE] != 0x7C {
                    return;
                }
                st.b[B_T28] = 0x20;
                st.b[B_FSTATE] = 3;
            }
            3 => {
                // $7229: a 32-call hold, then the id walks back to 122.
                if st.b[B_T28] != 0 {
                    st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                    return;
                }
                if self.fff0 & 3 != 0 {
                    return;
                }
                st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_sub(1);
                if st.b[B_SPRITE] != 0x7A {
                    return;
                }
                self.grower92_init(st); // $18 $92: jr $71D3
            }
            _ => self.grower92_init(st),
        }
    }

    pub(super) fn grower92_init(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = 0x4C;
        st.b[B_FSTATE] = 1;
        st.b[B_T24] = 3;
        st.b[B_T25] = 3;
        st.b[B_ANIMFRAME] = 3;
    }

    pub(super) fn step_picker(&self, st: &mut Staging, table: &[u8], c: u8) -> (u8, u8) {
        // Bound-safe: this consumer's `+16` never exceeds the sentinel, and a
        // past-the-end read cannot answer from another table's bytes.
        let b = table.get(st.b[B_PHASE] as usize).copied().unwrap_or(0);
        if st.b[B_PHASE] != c {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        (b, c)
    }

    pub(super) fn floater_pick(&self, st: &mut Staging) -> (u8, u8) {
        if st.b[B_T24] == 0 {
            self.step_picker(st, &STEP_74A1, 0x12)
        } else {
            self.step_picker(st, &FALL_SPEED, 0x12)
        }
    }

    pub(super) fn floater_tail(&mut self, st: &mut Staging, c: u8) {
        if st.b[B_PHASE] != c {
            return;
        }
        st.b[B_PHASE] = 0;
        st.b[B_T24] ^= 1;
        if st.b[B_T24] == 0 {
            return;
        }
        st.b[B_FSTATE] ^= 3; // $5D33
    }

    pub(super) fn floater_init(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = 0x48;
        st.b[B_FSTATE] = 1;
    }

    pub(super) fn handler_floater94(&mut self, st: &mut Staging) {
        match st.b[B_FSTATE] {
            1 => {
                let (b, c) = self.floater_pick(st);
                self.mv_y_sub(st, b);
                self.floater_tail(st, c);
            }
            2 => {
                let (b, c) = self.floater_pick(st);
                self.mv_y_add(st, b);
                self.floater_tail(st, c);
            }
            0 => self.floater_init(st),
            _ => {}
        }
    }

    pub(super) fn handler_floater95(&mut self, st: &mut Staging) {
        match st.b[B_FSTATE] {
            1 => {
                let (b, c) = self.floater_pick(st);
                self.mv_x_sub(st, b);
                self.floater_tail(st, c);
            }
            2 => {
                let (b, c) = self.floater_pick(st);
                self.mv_x_add(st, b);
                self.floater_tail(st, c);
            }
            0 => self.floater_init(st),
            _ => {}
        }
    }
}
