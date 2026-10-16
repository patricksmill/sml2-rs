//! Bank 2 reverse movement profiles, read by `$71D6` and `$71E4`.

use super::{ObjectSim, Staging};

/// ROM `$B20E..$B223` (CPU `$720E..$7223`).
const REVERSE_STEPS: [u8; 22] = [
    3, 2, 3, 3, 2, 3, 3, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0, 0,
];

impl ObjectSim {
    /// `$B1D6`: read the full profile, then advance `B_PHASE` unless it equals 21.
    /// The caller supplies the ROM's reachable phase range `0..=21`.
    pub(super) fn boss_reverse_picker_b1d6(&self, st: &mut Staging) -> (u8, u8) {
        self.step_picker(st, &REVERSE_STEPS, 21)
    }

    /// `$B1E4`: start nine bytes into the same profile, with phase cap 12.
    /// The caller supplies the ROM's reachable phase range `0..=12`.
    pub(super) fn boss_reverse_picker_b1e4(&self, st: &mut Staging) -> (u8, u8) {
        self.step_picker(st, &REVERSE_STEPS[9..], 12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{B_PHASE, B_SCRY};

    #[test]
    fn helper_0x0b1d6_full_profile_reads_before_increment_and_caps() {
        let sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        st.b = [0xA5; 32];
        st.b[B_PHASE] = 0;
        // Independently transcribed ROM bytes $B20E..$B223.
        for (phase, step) in [
            0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02,
            0x01, 0x02, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00,
        ]
        .into_iter()
        .enumerate()
        {
            let mut expected = st.b;
            expected[B_PHASE] = (phase as u8 + 1).min(21);
            assert_eq!(sim.boss_reverse_picker_b1d6(&mut st), (step, 21));
            assert_eq!(st.b, expected, "phase {phase}");
        }
        let capped = st.b;
        for _ in 0..3 {
            assert_eq!(sim.boss_reverse_picker_b1d6(&mut st), (0, 21));
            assert_eq!(st.b, capped);
        }
    }

    #[test]
    fn helper_0x0b1e4_offset_profile_reads_before_increment_and_caps() {
        let sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        st.b = [0x5A; 32];
        st.b[B_PHASE] = 0;
        // Entry $B1E4 chooses $B217: the ninth byte offset of $B20E.
        for (phase, step) in [3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0, 0]
            .into_iter()
            .enumerate()
        {
            let mut expected = st.b;
            expected[B_PHASE] = (phase as u8 + 1).min(12);
            assert_eq!(sim.boss_reverse_picker_b1e4(&mut st), (step, 12));
            assert_eq!(st.b, expected, "phase {phase}");
        }
        let capped = st.b;
        for _ in 0..3 {
            assert_eq!(sim.boss_reverse_picker_b1e4(&mut st), (0, 12));
            assert_eq!(st.b, capped);
        }
    }

    #[test]
    fn helper_0x0b1d6_and_0x0b1e4_feed_screen_y_sub_with_wrap() {
        let sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        st.b = [0xA5; 32];
        st.b[B_PHASE] = 0;
        st.b[B_SCRY] = 1;
        let (step, cap) = sim.boss_reverse_picker_b1d6(&mut st);
        // The loader already advanced phase when its caller consumes B.
        assert_eq!((st.b[B_PHASE], step, cap), (1, 3, 21));
        sim.scr_y_sub(&mut st, step);
        assert_eq!(st.b[B_SCRY], 0xFE);

        st.b[B_PHASE] = 2;
        st.b[B_SCRY] = 0;
        let (step, cap) = sim.boss_reverse_picker_b1e4(&mut st);
        assert_eq!((st.b[B_PHASE], step, cap), (3, 1, 12));
        sim.scr_y_sub(&mut st, step);
        let mut expected = [0xA5; 32];
        expected[B_PHASE] = 3;
        expected[B_SCRY] = 0xFF;
        assert_eq!(st.b, expected);
    }
}
