//! Runtime clocks for sampled `$FF04`; their explicit seeds are native/replay
//! origins, not reconstructed console boot or capture phases.

use sml2_core::divider::Divider;
use std::time::{Duration, Instant};

pub struct NativeDividerClock {
    origin: Instant,
    initial_counter: u16,
}

impl NativeDividerClock {
    pub fn new(initial_counter: u16) -> Self {
        Self {
            origin: Instant::now(),
            initial_counter,
        }
    }

    pub fn sample(&self) -> u8 {
        self.sample_elapsed(self.origin.elapsed())
    }

    fn sample_elapsed(&self, elapsed: Duration) -> u8 {
        Divider::from_elapsed(self.initial_counter, elapsed).sample()
    }
}

/// Replay time advances independently of object/player update gates. The
/// repository's nominal 60 Hz loop defines this clock, not console timing.
pub struct VirtualDividerClock {
    initial_counter: u16,
    ticks: u64,
}

impl VirtualDividerClock {
    pub fn new(initial_counter: u16) -> Self {
        Self {
            initial_counter,
            ticks: 0,
        }
    }

    pub fn advance_tick(&mut self) {
        self.advance_ticks(1);
    }

    pub fn advance_ticks(&mut self, count: u64) {
        self.ticks = self.ticks.checked_add(count).expect("replay tick overflow");
    }

    pub fn sample(&self) -> u8 {
        // Convert total ticks once, retaining the fractional-second phase;
        // repeated rounded 1/60-second additions would accumulate drift.
        let elapsed = Duration::new(
            self.ticks / 60,
            ((self.ticks % 60) * 1_000_000_000 / 60) as u32,
        );
        Divider::from_elapsed(self.initial_counter, elapsed).sample()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_elapsed_sampling_retains_seed_and_crosses_both_divider_parities() {
        let clock = NativeDividerClock::new(0x12FF);
        assert_eq!(clock.sample_elapsed(Duration::ZERO), 0x12);
        // 238 ns is less than one 4,194,304 Hz cycle, 239 ns exceeds it.
        assert_eq!(clock.sample_elapsed(Duration::from_nanos(238)), 0x12);
        assert_eq!(clock.sample_elapsed(Duration::from_nanos(239)), 0x13);
        assert_eq!(clock.sample_elapsed(Duration::from_secs(1)), 0x12);
        // Sampling and changing game-owned objects cannot reset the origin.
        let mut objects = sml2_core::objects::ObjectSim::new(vec![], 0);
        objects.reset_level_start();
        assert_eq!(clock.sample_elapsed(Duration::from_nanos(239)), 0x13);
        assert_eq!(clock.initial_counter, 0x12FF);
    }

    #[test]
    fn virtual_total_time_has_explicit_snapshot_phase_and_both_parities() {
        let mut clock = VirtualDividerClock::new(0);
        assert_eq!(clock.sample(), 0);
        clock.advance_tick();
        // floor(4,194,304 * 0.016666666) = 69,905 = $11111.
        assert_eq!(clock.sample(), 0x11);
        clock.advance_tick();
        assert_eq!(clock.sample(), 0x22);
        clock.advance_ticks(13);
        // Exactly a quarter second: 1,048,576 cycles, a complete counter wrap.
        // Rounding each tick before summing would leave DIV at $FF instead.
        assert_eq!(clock.sample(), 0);
        clock.advance_ticks(45);
        assert_eq!(clock.sample(), 0);
        let mut seeded = VirtualDividerClock::new(0xA57E);
        assert_eq!(seeded.sample(), 0xA5);
        seeded.advance_ticks(60 * 3600);
        assert_eq!(seeded.sample(), 0xA5);
    }

    #[test]
    fn virtual_time_counts_frozen_frames_and_entire_replay_gaps_once() {
        let mut clock = VirtualDividerClock::new(0x0080);
        // A trigger record already counts one tick. The 193-tick gap includes
        // that frame and all 192 fade/map ticks, even without object updates.
        clock.advance_tick();
        let total_gap: u64 = 193;
        clock.advance_ticks(total_gap.saturating_sub(1));
        assert_eq!(clock.ticks, 193);
        // 193/60 seconds yields 13,491,677 cycles: $DE5D after seed $80.
        assert_eq!(clock.sample(), 0xDE);
        let before = clock.sample();
        let mut objects = sml2_core::objects::ObjectSim::new(vec![], 0);
        objects.reset_level_start();
        assert_eq!(clock.sample(), before);
        clock.advance_tick();
        assert_eq!(clock.ticks, 194);
    }
}
