//! Wario's second falling child, ROM `$0AB6E` (boss animation 25).

use crate::objects::{ObjectSim, Staging};

impl ObjectSim {
    pub(in crate::objects) fn boss_anim25(&mut self, st: &mut Staging) {
        super::child_fall::update_legacy(self, st, 0x71, 0xAB6E);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{B_FLASH, B_FSTATE, B_SCRX, B_SCRY, B_SPRITE};

    fn fixture() -> (ObjectSim, Staging) {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        sim.slots[0].b[B_SCRX] = 0xC3;
        sim.slots[0].b[B_SCRY] = 0x89;
        sim.boss_delay = 0x95;
        sim.boss_def_count = 3;
        sim.done = 0x12;
        let mut st = Staging {
            b: std::array::from_fn(|i| i as u8 + 0x30),
            px: 0x1234,
            py: 0x5678,
        };
        st.b[B_FSTATE] = 1;
        (sim, st)
    }

    #[test]
    fn handler_0x0ab6e_init_exact_bytes() {
        let (mut sim, mut st) = fixture();
        sim.fff0 = 1;
        st.b[B_FSTATE] = 0;
        let mut expected = st.b;
        expected[B_SPRITE] = 0x71;
        expected[B_SCRX] = 0xC3;
        expected[B_SCRY] = 0x10;
        expected[B_FSTATE] = 1;
        expected[B_FLASH] = 0x10;
        sim.boss_anim25(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert_eq!(
            (sim.boss_delay, sim.boss_def_count, sim.done),
            (0x95, 3, 0x12)
        );
    }

    #[test]
    fn handler_0x0ab6e_odd_freezes_even_progresses_and_removes() {
        let (mut sim, mut st) = fixture();
        st.b[B_SCRY] = 0x5E;
        sim.fff0 = 0xFF;
        let mut expected = st.b;
        sim.boss_anim25(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!(
            (sim.boss_delay, sim.boss_def_count, sim.done),
            (0x95, 3, 0x12)
        );
        sim.fff0 = 0xFE;
        sim.boss_anim25(&mut st);
        expected[B_SCRY] = 0x5F;
        assert_eq!(st.b, expected);
        assert_eq!(
            (sim.boss_delay, sim.boss_def_count, sim.done),
            (0x95, 3, 0x12)
        );
        sim.boss_anim25(&mut st);
        assert_eq!(st.b, [0; 32]);
        assert_eq!((sim.boss_delay, sim.boss_def_count, sim.done), (1, 2, 1));
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
    }

    #[test]
    fn handler_0x0ab6e_wraps_coordinates_and_count() {
        let (mut sim, mut st) = fixture();
        st.b[B_SCRY] = 0xFF;
        sim.slots[0].b[B_SCRY] = 0x2F;
        sim.boss_def_count = 0;
        sim.boss_anim25(&mut st);
        assert_eq!(st.b, [0; 32]);
        assert_eq!((sim.boss_delay, sim.boss_def_count, sim.done), (1, 0xFF, 1));
    }

    #[test]
    fn handler_0x0ab6e_unknown_state_reports_without_writes() {
        let (mut sim, mut st) = fixture();
        st.b[B_FSTATE] = 2;
        let expected = st.b;
        sim.boss_anim25(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!(
            (sim.boss_delay, sim.boss_def_count, sim.done),
            (0x95, 3, 0x12)
        );
        assert_eq!(sim.unported, vec![0xAB6E]);
    }
}
