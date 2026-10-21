//! Wario's falling child, bank 2 ROM `$0AB30` (boss animation 22).

use crate::objects::{ObjectSim, Staging};

impl ObjectSim {
    pub(in crate::objects) fn boss_anim22(&mut self, st: &mut Staging) {
        super::child_fall::update_legacy(self, st, 0x70, 0xAB30);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{B_FLASH, B_FSTATE, B_SCRX, B_SCRY, B_SPRITE};

    fn fixture() -> (ObjectSim, Staging) {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        sim.slots[0].b[B_SCRX] = 0xB7;
        sim.slots[0].b[B_SCRY] = 0x86;
        sim.boss_delay = 0xA3;
        sim.boss_def_count = 4;
        sim.done = 0x57;
        let mut st = Staging {
            b: std::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(3)),
            px: 0x1234,
            py: 0x5678,
        };
        st.b[B_FSTATE] = 1;
        (sim, st)
    }

    #[test]
    fn handler_0x0ab30_init_preserves_unrelated_bytes() {
        let (mut sim, mut st) = fixture();
        st.b[B_FSTATE] = 0;
        sim.fff0 = 1; // Initialization is independent of frame parity.
        let mut expected = st.b;
        expected[B_SPRITE] = 0x70;
        expected[B_SCRX] = 0xB7;
        expected[B_SCRY] = 0x10;
        expected[B_FSTATE] = 1;
        expected[B_FLASH] = 0x10;
        sim.boss_anim22(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert_eq!(
            (sim.boss_delay, sim.boss_def_count, sim.done),
            (0xA3, 4, 0x57)
        );
    }

    #[test]
    fn handler_0x0ab30_odd_frame_freezes_even_at_threshold() {
        let (mut sim, mut st) = fixture();
        sim.fff0 = 0xFF;
        st.b[B_SCRY] = 0x6F;
        let expected = st.b;
        sim.boss_anim22(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!(
            (sim.boss_delay, sim.boss_def_count, sim.done),
            (0xA3, 4, 0x57)
        );
    }

    #[test]
    fn handler_0x0ab30_even_frame_moves_one_without_other_writes() {
        let (mut sim, mut st) = fixture();
        sim.fff0 = 0xFE;
        st.b[B_SCRY] = 0x3F;
        let mut expected = st.b;
        expected[B_SCRY] = 0x40;
        sim.boss_anim22(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!(
            (sim.boss_delay, sim.boss_def_count, sim.done),
            (0xA3, 4, 0x57)
        );
    }

    #[test]
    fn handler_0x0ab30_compares_new_y_nibbles_with_wrapping_addition() {
        // Crossing into $60 must remove immediately. Low nibbles of both
        // coordinates are ignored, and $E0 + $20 wraps to $00.
        for (start_y, primary_y) in [(0x5F, 0x8F), (0x64, 0x81), (0xDF, 0x07), (0xFE, 0x1F)] {
            let (mut sim, mut st) = fixture();
            st.b[B_SCRY] = start_y;
            sim.slots[0].b[B_SCRY] = primary_y;
            sim.boss_anim22(&mut st);
            assert_eq!(st.b, [0; 32], "start_y={start_y:02x}");
            assert_eq!((sim.boss_delay, sim.boss_def_count, sim.done), (1, 3, 1));
            assert_eq!((st.px, st.py), (0x1234, 0x5678));
        }
    }

    #[test]
    fn handler_0x0ab30_coordinate_and_count_underflow() {
        let (mut sim, mut st) = fixture();
        st.b[B_SCRY] = 0xFF;
        sim.slots[0].b[B_SCRY] = 0x2A;
        sim.boss_def_count = 0;
        sim.boss_anim22(&mut st);
        assert_eq!(st.b, [0; 32]);
        assert_eq!((sim.boss_delay, sim.boss_def_count, sim.done), (1, 0xFF, 1));
    }

    #[test]
    fn handler_0x0ab30_unknown_state_preserves_bytes_and_reports() {
        let (mut sim, mut st) = fixture();
        st.b[B_FSTATE] = 2;
        let expected = st.b;
        sim.boss_anim22(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert_eq!(
            (sim.boss_delay, sim.boss_def_count, sim.done),
            (0xA3, 4, 0x57)
        );
        assert_eq!(sim.unported, vec![0xAB30]);
    }
}
