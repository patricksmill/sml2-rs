//! Native clock input for the Game Boy DIV register ($FF04).
//!
//! DIV samples the upper byte of a 16-bit counter clocked at 4,194,304 Hz
//! (Nintendo's Game Boy Programming Manual, section 2.4.2, printed page 24).
//! The caller supplies the initial phase: this does not infer console boot or
//! replay phase. The audited game does not write DIV, so no reset operation is
//! needed here. Runtime clock ownership and object input wiring live elsewhere.

use std::time::Duration;

/// A single divider counter with an explicitly chosen initial phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divider {
    counter: u16,
}

impl Divider {
    pub const CLOCK_HZ: u64 = 4_194_304;
    /// Cycles per console frame: 154 lines of 456 dots.
    pub const FRAME_CYCLES: u64 = 70_224;

    pub const fn new(initial_counter: u16) -> Self {
        Self {
            counter: initial_counter,
        }
    }

    /// Derive a counter from an initial phase and total elapsed time.
    ///
    /// Only the fractional part of a clock cycle is truncated; the counter's
    /// lower byte is retained. Pass total time since the initial phase, rather
    /// than repeatedly adding rounded duration deltas. Integer arithmetic fits
    /// in u128 even for Duration::MAX.
    pub fn from_elapsed(initial_counter: u16, elapsed: Duration) -> Self {
        let cycles = u128::from(elapsed.as_secs()) * u128::from(Self::CLOCK_HZ)
            + u128::from(elapsed.subsec_nanos()) * u128::from(Self::CLOCK_HZ) / 1_000_000_000;
        Self::new(initial_counter.wrapping_add(cycles as u16))
    }

    pub const fn counter(self) -> u16 {
        self.counter
    }

    /// The byte exposed by a read of $FF04.
    pub const fn sample(self) -> u8 {
        (self.counter >> 8) as u8
    }

    /// Advance by clock cycles, wrapping the full counter modulo 65,536.
    pub fn advance_cycles(&mut self, cycles: u64) {
        self.counter = self.counter.wrapping_add(cycles as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_seed_preserves_both_counter_bytes() {
        let divider = Divider::new(0xa57e);
        assert_eq!(divider.counter(), 0xa57e);
        assert_eq!(divider.sample(), 0xa5);
        assert_eq!(Divider::from_elapsed(0xa57e, Duration::ZERO), divider);
    }

    #[test]
    fn lower_byte_carry_changes_sample_and_covers_both_parities() {
        let mut divider = Divider::new(0x00fe);
        assert_eq!(divider.sample(), 0);
        divider.advance_cycles(1);
        assert_eq!(divider.counter(), 0x00ff);
        assert_eq!(divider.sample(), 0);
        divider.advance_cycles(1);
        assert_eq!(divider.counter(), 0x0100);
        assert_eq!(divider.sample(), 1);
        divider.advance_cycles(256);
        assert_eq!(divider.counter(), 0x0200);
        assert_eq!(divider.sample(), 2);
    }

    #[test]
    fn upper_byte_wrap_and_large_cycle_advances() {
        let mut divider = Divider::new(0xffff);
        divider.advance_cycles(1);
        assert_eq!(divider.counter(), 0);
        assert_eq!(divider.sample(), 0);
        divider.advance_cycles(65_536 * 10_000 + 0x1234);
        assert_eq!(divider.counter(), 0x1234);
        divider.advance_cycles(u64::MAX);
        assert_eq!(divider.counter(), 0x1233);
        divider.advance_cycles(0);
        assert_eq!(divider.counter(), 0x1233);
    }

    #[test]
    fn elapsed_duration_retains_cycle_and_lower_byte_phase() {
        // A clock cycle is about 238.4186 ns; DIV increments every
        // 61,035.15625 ns. These expectations are literal boundary values.
        for (nanos, counter, sample) in [
            (1, 0x0000, 0),
            (238, 0x0000, 0),
            (239, 0x0001, 0),
            (61_035, 0x00ff, 0),
            (61_036, 0x0100, 1),
            (122_070, 0x01ff, 1),
            (122_071, 0x0200, 2),
            (15_624_999, 0xffff, 0xff),
            (15_625_000, 0x0000, 0),
            (15_625_239, 0x0001, 0),
        ] {
            let divider = Divider::from_elapsed(0, Duration::from_nanos(nanos));
            assert_eq!(divider.counter(), counter, "elapsed {nanos} ns");
            assert_eq!(divider.sample(), sample, "elapsed {nanos} ns");
        }
    }

    #[test]
    fn elapsed_duration_adds_to_seed_including_lower_byte_carry() {
        assert_eq!(
            Divider::from_elapsed(0x00ff, Duration::from_nanos(238)).counter(),
            0x00ff
        );
        assert_eq!(
            Divider::from_elapsed(0x00ff, Duration::from_nanos(239)).counter(),
            0x0100
        );
        // One millisecond contains 4,194 whole cycles (0x1062).
        let divider = Divider::from_elapsed(0xf0ff, Duration::from_millis(1_001));
        assert_eq!(divider.counter(), 0x0161);
        assert_eq!(divider.sample(), 1);
    }

    #[test]
    fn maximum_duration_is_safe_and_preserves_seed_phase() {
        // Every whole second is exactly 64 full counter wraps. The largest
        // nanosecond remainder is one cycle short of another whole second.
        let divider = Divider::from_elapsed(0x1234, Duration::MAX);
        assert_eq!(divider.counter(), 0x1233);
        assert_eq!(divider.sample(), 0x12);
        assert_eq!(
            Divider::from_elapsed(0x1234, Duration::from_secs(u64::MAX)).counter(),
            0x1234
        );
    }
}
