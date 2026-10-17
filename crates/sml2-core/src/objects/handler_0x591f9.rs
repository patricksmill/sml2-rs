//! Handler ROM `0x591F9` (bank `$16`, CPU `$51F9`) — anim 58 in both profile tables.
//!
//! Vertical bobber / patrol state machine:
//! - Every frame: `sprite_toggle1` (mask-1 sprite bit 0 toggle on even `fff0`).
//! - State 0: Initial spawn. Sets `B_TYPE = 0x56 (86)`, `B_FLASH = 0x10 (16)`.
//!   Compares Mario's Y with the object's Y via `$5B407`: if Mario is below
//!   (`mario_ref_y > obj_y`), state becomes 2 (move down); otherwise 1 (move up).
//! - States 1-4: Gated by `fff0 & 3 == 0` (runs every 4th frame).
//!   - State 1: Moves UP by `BOUNCE_SPEED[B_PHASE]` (`mv_y_sub`).
//!     When `B_PHASE == 15`: resets `B_PHASE = 0`, state += 2 -> State 3.
//!   - State 2: Moves DOWN by `BOUNCE_SPEED[B_PHASE]` (`mv_y_add`).
//!     When `B_PHASE == 15`: resets `B_PHASE = 0`, state += 2 -> State 4.
//!   - State 3: Pauses while `B_T28 != 0` (decrements `B_T28`).
//!     Once `B_T28 == 0`: moves DOWN by `FALL_SPEED[B_PHASE]` (cap 15, `mv_y_add`).
//!     When `B_PHASE == 15`: resets `B_PHASE = 0`, state -= 1 -> State 2.
//!   - State 4: Pauses while `B_T28 != 0` (decrements `B_T28`).
//!     Once `B_T28 == 0`: moves UP by `FALL_SPEED[B_PHASE]` (cap 15, `mv_y_sub`).
//!     When `B_PHASE == 15`: resets `B_PHASE = 0`, state = 1.

use super::*;

impl ObjectSim {
    pub(super) fn handler_0x591f9(&mut self, st: &mut Staging) {
        // $591F9: CD 21 75 call $5B521 — mask-1 sprite toggle on even $FFF0.
        self.sprite_toggle1(st);

        // State 0: initial setup.
        if st.b[B_FSTATE] == 0 {
            // $59201: ld a, 86; ldh [$FFE2], a
            st.b[B_TYPE] = 0x56;
            // $59205: ld a, 16; ldh [$FFDE], a
            st.b[B_FLASH] = 0x10;
            // $59209: call $5B407 — compare sMarioRefY with object Y.
            self.face_mario_y(st);
            return;
        }

        // $5920D: ldh a, [$FFF0]; and $03; ret nz — 4-frame gate for movement.
        if self.fff0 & 3 != 0 {
            return;
        }

        // rst $28 dispatch over states 1-4.
        match st.b[B_FSTATE] {
            // $521E: state 1 (move UP on bounce curve, transition to state 3).
            1 => {
                let b = self.bounce_step(st);
                self.mv_y_sub(st, b);
                if st.b[B_PHASE] == 15 {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = 3;
                }
            }
            // $5232: state 2 (move DOWN on bounce curve, transition to state 4).
            2 => {
                let b = self.bounce_step(st);
                self.mv_y_add(st, b);
                if st.b[B_PHASE] == 15 {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = 4;
                }
            }
            // $523A: state 3 (delay on B_T28, move DOWN on fall curve, transition to state 2).
            3 => {
                if st.b[B_T28] != 0 {
                    st.b[B_T28] -= 1;
                    return;
                }
                let b = self.curve_step_15(st, &FALL_SPEED);
                self.mv_y_add(st, b);
                if st.b[B_PHASE] == 15 {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = 2;
                }
            }
            // $525F: state 4 (delay on B_T28, move UP on fall curve, transition to state 1).
            4 => {
                if st.b[B_T28] != 0 {
                    st.b[B_T28] -= 1;
                    return;
                }
                let b = self.curve_step_15(st, &FALL_SPEED);
                self.mv_y_sub(st, b);
                if st.b[B_PHASE] == 15 {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = 1;
                }
            }
            _ => self.note_unported(0x591F9),
        }
    }

    /// `$5B407`: Compare `sMarioRefY` with object's Y.
    /// Sets `facing` to 1 if `mario_ref_y > obj_y`, 0xFF if `< obj_y`, 0 if `==`.
    /// Sets `st.b[B_FSTATE]` to 2 if `facing == 1`, else 1.
    pub(super) fn face_mario_y(&mut self, st: &mut Staging) {
        let obj_y = st.y();
        self.facing = if self.mario_ref_y > obj_y {
            1
        } else if self.mario_ref_y < obj_y {
            0xFF
        } else {
            0
        };
        st.b[B_FSTATE] = if self.facing == 1 { 2 } else { 1 };
    }

    /// `$5B456` with `c = 15`: read speed from curve at `B_PHASE`, increment `B_PHASE` if `< 15`.
    fn curve_step_15(&self, st: &mut Staging, curve: &[u8]) -> u8 {
        let idx = (st.b[B_PHASE] as usize).min(15);
        let b = curve[idx];
        if st.b[B_PHASE] != 15 {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state0_initialization_sets_type_flash_and_faces_mario_y() {
        let mut sim = ObjectSim::new(vec![], 0);

        // Case 1: Mario below object (mario_ref_y > obj_y) -> state 2, facing 1
        sim.mario_ref_y = 200;
        let mut st = Staging::default();
        set_y(&mut st.b, 100);
        st.b[B_FSTATE] = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_TYPE], 0x56, "type set to 86");
        assert_eq!(st.b[B_FLASH], 0x10, "flash set to 16");
        assert_eq!(st.b[B_FSTATE], 2, "state set to 2 when Mario below");
        assert_eq!(sim.facing, 1, "facing set to 1");

        // Case 2: Mario above object (mario_ref_y < obj_y) -> state 1, facing 0xFF
        sim.mario_ref_y = 50;
        let mut st = Staging::default();
        set_y(&mut st.b, 100);
        st.b[B_FSTATE] = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_TYPE], 0x56);
        assert_eq!(st.b[B_FLASH], 0x10);
        assert_eq!(st.b[B_FSTATE], 1, "state set to 1 when Mario above");
        assert_eq!(sim.facing, 0xFF, "facing set to 255");

        // Case 3: Mario at same Y (mario_ref_y == obj_y) -> state 1, facing 0
        sim.mario_ref_y = 100;
        let mut st = Staging::default();
        set_y(&mut st.b, 100);
        st.b[B_FSTATE] = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 1, "state set to 1 when Mario level");
        assert_eq!(sim.facing, 0, "facing set to 0");
    }

    #[test]
    fn sprite_toggle1_toggles_on_even_frames() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        st.b[B_SPRITE] = 0x10;
        st.b[B_FSTATE] = 1;

        // Even frame (fff0 = 0): toggles bit 0
        sim.fff0 = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_SPRITE], 0x11, "even frame toggled sprite bit 0");

        // Odd frame (fff0 = 1): does not toggle
        sim.fff0 = 1;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_SPRITE], 0x11, "odd frame preserved sprite bit 0");
    }

    #[test]
    fn movement_gated_to_fff0_mod_4() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 200);
        st.b[B_FSTATE] = 1;
        st.b[B_PHASE] = 0;

        // Frames 1, 2, 3: movement gate closed, Y does not change
        for frame in [1u8, 2, 3] {
            sim.fff0 = frame;
            sim.handler_0x591f9(&mut st);
            assert_eq!(st.y(), 200, "Y unchanged on frame {frame}");
            assert_eq!(st.b[B_PHASE], 0, "phase unchanged on frame {frame}");
        }

        // Frame 4 (4 & 3 == 0): movement gate opens, Y moves UP by BOUNCE_SPEED[0] (3)
        sim.fff0 = 4;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.y(), 197, "Y moved up by 3");
        assert_eq!(st.b[B_PHASE], 1, "phase incremented to 1");
    }

    #[test]
    fn state1_bounce_up_reaches_cap_and_transitions_to_state3() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 500);
        st.b[B_FSTATE] = 1;
        st.b[B_PHASE] = 14;

        // At phase 14, step reads BOUNCE_SPEED[14] (0), phase reaches 15 -> resets to 0, state becomes 3
        sim.fff0 = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_PHASE], 0, "phase reset to 0");
        assert_eq!(st.b[B_FSTATE], 3, "state transitioned 1 -> 3");
        assert_eq!(st.y(), 500 - (BOUNCE_SPEED[14] as u16));
    }

    #[test]
    fn state2_bounce_down_reaches_cap_and_transitions_to_state4() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 500);
        st.b[B_FSTATE] = 2;
        st.b[B_PHASE] = 14;

        // At phase 14, step reads BOUNCE_SPEED[14] (0), phase reaches 15 -> resets to 0, state becomes 4
        sim.fff0 = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_PHASE], 0, "phase reset to 0");
        assert_eq!(st.b[B_FSTATE], 4, "state transitioned 2 -> 4");
        assert_eq!(st.y(), 500 + (BOUNCE_SPEED[14] as u16));
    }

    #[test]
    fn state3_t28_delay_then_fall_down_and_transitions_to_state2() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 100);
        st.b[B_FSTATE] = 3;
        st.b[B_T28] = 2;
        st.b[B_PHASE] = 0;

        // Tick 1 with fff0 & 3 == 0: decrements T28 from 2 to 1, no move
        sim.fff0 = 4;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_T28], 1);
        assert_eq!(st.y(), 100);
        assert_eq!(st.b[B_PHASE], 0);

        // Tick 2: decrements T28 from 1 to 0, no move
        sim.fff0 = 8;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_T28], 0);
        assert_eq!(st.y(), 100);
        assert_eq!(st.b[B_PHASE], 0);

        // Tick 3: T28 is 0 -> moves DOWN by FALL_SPEED[0] (0)
        sim.fff0 = 12;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.y(), 100 + (FALL_SPEED[0] as u16));
        assert_eq!(st.b[B_PHASE], 1);
        assert_eq!(st.b[B_FSTATE], 3);

        // Fast forward to phase 14
        st.b[B_PHASE] = 14;
        sim.fff0 = 16;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_PHASE], 0, "phase reset to 0");
        assert_eq!(st.b[B_FSTATE], 2, "state transitioned 3 -> 2");
    }

    #[test]
    fn state4_t28_delay_then_fall_up_and_transitions_to_state1() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 200);
        st.b[B_FSTATE] = 4;
        st.b[B_T28] = 1;
        st.b[B_PHASE] = 0;

        // Decrements T28 to 0
        sim.fff0 = 4;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_T28], 0);
        assert_eq!(st.y(), 200);

        // Fast forward to phase 14
        st.b[B_PHASE] = 14;
        sim.fff0 = 8;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_PHASE], 0, "phase reset to 0");
        assert_eq!(st.b[B_FSTATE], 1, "state transitioned 4 -> 1");
    }

    #[test]
    fn complete_closed_cycle_1_to_3_to_2_to_4_to_1() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        sim.mario_ref_y = 50; // above
        set_y(&mut st.b, 200);
        st.b[B_FSTATE] = 0;

        // Init -> state 1
        sim.fff0 = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 1);

        // State 1 completes -> state 3
        st.b[B_PHASE] = 14;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 3);

        // State 3 completes -> state 2
        st.b[B_T28] = 0;
        st.b[B_PHASE] = 14;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 2);

        // State 2 completes -> state 4
        st.b[B_PHASE] = 14;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 4);

        // State 4 completes -> state 1
        st.b[B_T28] = 0;
        st.b[B_PHASE] = 14;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 1);
    }

    #[test]
    fn dispatch_and_unported_handling() {
        let mut sim = ObjectSim::new(vec![], 0);
        let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
        let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        )))
        .unwrap();

        // Valid dispatch: runs without flagging unported
        let mut st = Staging::default();
        st.b[B_DEF] = 58;
        st.b[B_FSTATE] = 0;
        sim.dispatch_handler(0x591F9, &mut st, &layout, &tables);
        assert!(sim.unported.is_empty(), "0x591F9 is implemented");
        assert_eq!(st.b[B_TYPE], 0x56);

        // Invalid state flags unported
        st.b[B_FSTATE] = 99;
        sim.dispatch_handler(0x591F9, &mut st, &layout, &tables);
        assert_eq!(sim.unported, vec![0x591F9]);
    }
}
