//! Slot conversion for ROM `$5B0D5` (mode B anim 90), the square patroller.
//! The extracted profile's sixth "state" is the scan reading one word past
//! the table (`$5B0E5`, mid-instruction); only states 1-4 are ever written
//! after init, so past-the-table values report through `note_unported`.
use super::{handler_0x5b0d5, *};
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5b0d5(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x5b0d5::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5b0d5::Effect::Continue) => {}
            Ok(handler_0x5b0d5::Effect::Faced(side)) => {
                self.facing = match side {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Err(_) => self.note_unported(0x5B0D5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(state: u8) -> (ObjectSim, Staging) {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        sim.fff0 = 1; // odd: the entry's $5B521 sprite toggle stays closed
        let mut st = Staging {
            b: std::array::from_fn(|i| i as u8 + 0x30),
            px: 0x1234,
            py: 0x5678,
        };
        st.b[B_FSTATE] = state;
        (sim, st)
    }

    fn expect_step(sim: &mut ObjectSim, st: &mut Staging, writes: &[(usize, u8)]) {
        let mut expected = st.b;
        for &(offset, value) in writes {
            expected[offset] = value;
        }
        sim.handler_0x5b0d5(st);
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
    }

    #[test]
    fn handler_0x5b0d5_init_arms_type_timers_facing_and_loop_side() {
        // $70E5: type $4D, +25 = $18, +24 = $30, then $73C6's facing write
        // ($5B3E0 compare: right of the object -> 2/$20, else 1/0) and
        // +16 = the fresh +15.
        for (mario, facing, next, flip) in [
            (0x3032u16, 1u8, 2u8, 0x20u8), // Mario right: state 2, flip $20
            (0x3030, 0xFF, 1, 0),          // Mario left: state 1, flip 0
            (0x3031, 0, 1, 0),             // level: the `ret z` keeps facing 0
        ] {
            let (mut sim, mut st) = fixture(0);
            sim.mario_ref_x = mario;
            expect_step(
                &mut sim,
                &mut st,
                &[
                    (B_TYPE, 0x4D),
                    (B_T25, 0x18),
                    (B_T24, 0x30),
                    (B_FSTATE, next),
                    (B_FLIP, flip),
                    (B_PHASE, next),
                ],
            );
            assert_eq!(sim.facing, facing, "mario x {mario:#06x}");
        }
        // Even $FFF0: the $5B521 prefix toggles +11 bit 0 first; state 0
        // writes nothing else.
        let (mut sim, mut st) = fixture(0);
        sim.fff0 = 2;
        sim.mario_ref_x = 0x3032;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_SPRITE, 0x3A),
                (B_TYPE, 0x4D),
                (B_T25, 0x18),
                (B_T24, 0x30),
                (B_FSTATE, 2),
                (B_FLIP, 0x20),
                (B_PHASE, 2),
            ],
        );
    }

    #[test]
    fn handler_0x5b0d5_state1_walks_left_and_turns_on_timer_expiry() {
        // Nonzero +25 after the step: `ret nz`, no turn.
        let (mut sim, mut st) = fixture(1);
        st.b[B_T25] = 2;
        expect_step(&mut sim, &mut st, &[(B_XLO, 0x30), (B_T25, 1)]);
        // 16-bit borrow: $3000 - 1 = $2FFF.
        let (mut sim, mut st) = fixture(1);
        st.b[B_T25] = 0x18;
        st.b[B_XHI] = 0x30;
        st.b[B_XLO] = 0x00;
        expect_step(
            &mut sim,
            &mut st,
            &[(B_XHI, 0x2F), (B_XLO, 0xFF), (B_T25, 0x17)],
        );
        // Expiry: +25 reloads from +24; $710A sends +16 == 1 to $7115
        // (state 3, sprite $78) and everything else to $710C (state 4, $76).
        let (mut sim, mut st) = fixture(1);
        st.b[B_T25] = 1;
        st.b[B_T24] = 0x30;
        st.b[B_PHASE] = 1;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_XLO, 0x30),
                (B_T25, 0x30),
                (B_FSTATE, 3),
                (B_SPRITE, 0x78),
            ],
        );
        let (mut sim, mut st) = fixture(1);
        st.b[B_T25] = 1;
        st.b[B_T24] = 0x30;
        st.b[B_PHASE] = 2;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_XLO, 0x30),
                (B_T25, 0x30),
                (B_FSTATE, 4),
                (B_SPRITE, 0x76),
            ],
        );
    }

    #[test]
    fn handler_0x5b0d5_state2_walks_right_and_clears_flip_on_expiry() {
        // Nonzero +25 after the step: `ret nz`, flip untouched.
        let (mut sim, mut st) = fixture(2);
        st.b[B_T25] = 0x18;
        expect_step(&mut sim, &mut st, &[(B_XLO, 0x32), (B_T25, 0x17)]);
        // Carry: $30FF + 1 = $3100.
        let (mut sim, mut st) = fixture(2);
        st.b[B_T25] = 0x18;
        st.b[B_XHI] = 0x30;
        st.b[B_XLO] = 0xFF;
        expect_step(
            &mut sim,
            &mut st,
            &[(B_XHI, 0x31), (B_XLO, 0x00), (B_T25, 0x17)],
        );
        // Expiry: `xor a` clears +13 first; $7130 sends +16 == 1 to $710C
        // (state 4, $76) and everything else to $7115 (state 3, $78).
        for (phase, next, sprite) in [(1u8, 4u8, 0x76u8), (2, 3, 0x78)] {
            let (mut sim, mut st) = fixture(2);
            st.b[B_T25] = 1;
            st.b[B_T24] = 0x30;
            st.b[B_PHASE] = phase;
            expect_step(
                &mut sim,
                &mut st,
                &[
                    (B_XLO, 0x32),
                    (B_T25, 0x30),
                    (B_FLIP, 0),
                    (B_FSTATE, next),
                    (B_SPRITE, sprite),
                ],
            );
        }
    }

    #[test]
    fn handler_0x5b0d5_states3_and_4_move_y_and_latch_the_turn() {
        // $7137 state 3: y += 1, then `ret nz` on a nonzero +25.
        let (mut sim, mut st) = fixture(3);
        st.b[B_T25] = 2;
        expect_step(&mut sim, &mut st, &[(B_YLO, 0x35), (B_T25, 1)]);
        // Carry: $30FF + 1 = $3100.
        let (mut sim, mut st) = fixture(3);
        st.b[B_T25] = 2;
        st.b[B_YHI] = 0x30;
        st.b[B_YLO] = 0xFF;
        expect_step(
            &mut sim,
            &mut st,
            &[(B_YHI, 0x31), (B_YLO, 0x00), (B_T25, 1)],
        );
        // State 3 expiry: sprite $74, then +16 == 1 -> $7154 (flip $20,
        // state 2); else $714F (state 1, flip untouched).
        let (mut sim, mut st) = fixture(3);
        st.b[B_T25] = 1;
        st.b[B_T24] = 0x30;
        st.b[B_PHASE] = 1;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_YLO, 0x35),
                (B_T25, 0x30),
                (B_SPRITE, 0x74),
                (B_FLIP, 0x20),
                (B_FSTATE, 2),
            ],
        );
        let (mut sim, mut st) = fixture(3);
        st.b[B_T25] = 1;
        st.b[B_T24] = 0x30;
        st.b[B_PHASE] = 2;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_YLO, 0x35),
                (B_T25, 0x30),
                (B_SPRITE, 0x74),
                (B_FSTATE, 1),
            ],
        );
        // $715D state 4: y -= 1 with a borrow through the high byte.
        let (mut sim, mut st) = fixture(4);
        st.b[B_T25] = 2;
        st.b[B_YLO] = 0x00;
        expect_step(
            &mut sim,
            &mut st,
            &[(B_YHI, 0x32), (B_YLO, 0xFF), (B_T25, 1)],
        );
        // State 4 expiry is state 3's mirror: +16 == 1 -> $714F (state 1,
        // no flip); else $7154 (flip $20, state 2).
        let (mut sim, mut st) = fixture(4);
        st.b[B_T25] = 1;
        st.b[B_T24] = 0x30;
        st.b[B_PHASE] = 1;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_YLO, 0x33),
                (B_T25, 0x30),
                (B_SPRITE, 0x74),
                (B_FSTATE, 1),
            ],
        );
        let (mut sim, mut st) = fixture(4);
        st.b[B_T25] = 1;
        st.b[B_T24] = 0x30;
        st.b[B_PHASE] = 2;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_YLO, 0x33),
                (B_T25, 0x30),
                (B_SPRITE, 0x74),
                (B_FLIP, 0x20),
                (B_FSTATE, 2),
            ],
        );
    }

    #[test]
    fn handler_0x5b0d5_unknown_state_reports_without_staging_writes() {
        // Five targets at $5B0DB; nothing past them is reachable (only 1-4
        // are written after init), so 5+ is a pure report on an odd tick.
        for state in [5u8, 6, 0x7F, 0xFF] {
            let (mut sim, mut st) = fixture(state);
            expect_step(&mut sim, &mut st, &[]);
            assert_eq!(sim.unported, vec![0x5B0D5], "state {state}");
        }
        // On an even tick the entry prefix still toggles +11 before reporting.
        let (mut sim, mut st) = fixture(5);
        sim.fff0 = 2;
        expect_step(&mut sim, &mut st, &[(B_SPRITE, 0x3A)]);
        assert_eq!(sim.unported, vec![0x5B0D5]);
    }
}
