//! Handler ROM `0x586EB` (bank `$16`, CPU `$46EB`) — anim 30 in both profile tables.
//!
//! `$5B6EB`: `call $7684; ldh a,[$FFD5]; add $66; ldh [$FFE2],a; ret`.
//! `$7684` is the 4-frame flash toggle (`$FFDE ^= $10` when `$FFF0 & 3 == 0`);
//! it is *not* `helpers.rs::flash_toggle`, whose gate is mask 1.

use super::*;

impl ObjectSim {
    pub(super) fn handler_0x586eb(&mut self, st: &mut Staging) {
        // $5B684: `ldh a,[$FFF0]; and $03; ret nz` — the 4-frame gate.
        if self.fff0 & 3 == 0 {
            st.b[B_FLASH] ^= 0x10;
        }
        // `ldh a,[$FFD5]; add $66; ldh [$FFE2],a`.
        st.b[B_TYPE] = st.b[B_DEF].wrapping_add(0x66);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_0x586eb_toggles_flash_on_the_four_frame_gate_only() {
        // ROM `$5B684`: `ldh a,[$FFF0]; and $03; ret nz` guards the `$FFDE ^= $10`.
        // These cases pin every mask: 1/2/3 stay closed, 4 opens (so reverting
        // `& 3` to `& 1` or `& 2` also fails).
        let mut sim = ObjectSim::new(vec![], 0);
        for (fff0, flash) in [(1u8, 0xA5u8), (2, 0xA5), (3, 0xA5), (4, 0xA5 ^ 0x10)] {
            sim.fff0 = fff0;
            let mut st = Staging::default();
            st.b = [0x5A; 32];
            st.b[B_FLASH] = 0xA5;
            sim.handler_0x586eb(&mut st);
            let mut expected = [0x5A; 32];
            expected[B_FLASH] = flash;
            // The `add $66` tail is unconditional; its byte is the type test's
            // subject, so exempt it and compare every other staged byte exactly.
            expected[B_TYPE] = st.b[B_TYPE];
            assert_eq!(st.b, expected, "fff0={fff0}");
        }
    }

    #[test]
    fn handler_0x586eb_sets_type_from_the_staged_anim_byte() {
        // `add $66` wraps: $FFD5 = $B0 -> $FFE2 = $16.
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1; // blink gate closed: only the type write is observed
        let mut st = Staging::default();
        st.b = [0x5A; 32];
        st.b[B_DEF] = 0xB0;
        sim.handler_0x586eb(&mut st);
        let mut expected = [0x5A; 32];
        expected[B_DEF] = 0xB0;
        expected[B_TYPE] = 0x16;
        assert_eq!(st.b, expected);
    }
}
