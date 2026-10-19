//! ROM `$B40E..$B419`, bank 2 CPU `$740E`: toggle `$FFDE` every fourth frame.

use super::{ObjectSim, Staging, B_FLASH};

impl ObjectSim {
    pub(super) fn boss_flash_b40e(&self, st: &mut Staging) {
        if self.fff0 & 3 == 0 {
            st.b[B_FLASH] ^= 0x10;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_0x0b40e_all_frame_gates_preserve_other_staging_bytes() {
        let mut sim = ObjectSim::new(vec![], 0);
        for frame in 0..=u8::MAX {
            sim.fff0 = frame;
            let mut st = Staging::default();
            st.b = [0xA5; 32];
            st.px = 0x1234;
            st.py = 0x5678;
            let mut expected = st.b;
            if frame & 3 == 0 {
                expected[B_FLASH] = 0xB5;
            }
            sim.boss_flash_b40e(&mut st);
            assert_eq!(st.b, expected, "frame {frame:#04x}");
            assert_eq!((st.px, st.py), (0x1234, 0x5678));
        }
    }

    #[test]
    fn helper_0x0b40e_repeated_toggles_preserve_all_other_flash_bits() {
        let mut sim = ObjectSim::new(vec![], 0);
        for initial in 0..=u8::MAX {
            let mut st = Staging::default();
            st.b[B_FLASH] = initial;
            sim.fff0 = 0xFC;
            sim.boss_flash_b40e(&mut st);
            assert_eq!(st.b[B_FLASH], initial ^ 0x10);
            sim.fff0 = 0;
            sim.boss_flash_b40e(&mut st);
            assert_eq!(st.b[B_FLASH], initial);
        }
    }
}
