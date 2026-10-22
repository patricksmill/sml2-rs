//! The runtime clock for sampled `$FF04`. Its explicit seed is a native or
//! replay origin, not a reconstructed console boot or capture phase.

use sml2_core::divider::Divider;

/// `rDIV` advanced by one console frame per game tick.
///
/// The ROM's random reads mask DIV (`& 3`, `& $3F`, `& $7E`, ...) and expect
/// every value to come up at its share of frames. A console frame is 70,224
/// cycles, which moves DIV by 274.31 and visits all 256 values. Wall time does
/// not: at exactly 60 Hz DIV moves by 273.07, so only 15 values ever appear
/// and a masked check fires four times too often or never. The clock counts
/// game ticks, not host time, so host frame pacing cannot change it either.
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

    /// A clock whose origin comes from the host's wall clock, so each native
    /// launch starts from its own DIV phase.
    pub fn seeded_from_time() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.subsec_nanos());
        Self::new(nanos as u16 ^ (nanos >> 16) as u16)
    }

    pub fn advance_tick(&mut self) {
        self.advance_ticks(1);
    }

    pub fn advance_ticks(&mut self, count: u64) {
        self.ticks = self.ticks.checked_add(count).expect("replay tick overflow");
    }

    pub fn sample(&self) -> u8 {
        let mut divider = Divider::new(self.initial_counter);
        divider.advance_cycles(self.ticks.wrapping_mul(Divider::FRAME_CYCLES));
        divider.sample()
    }

    /// Construct sampled map input from the virtual clock.
    pub fn map_input(&self, keys_held: u8, keys_pressed: u8) -> sml2_core::map_sim::MapInput {
        sml2_core::map_sim::MapInput::new(keys_held, keys_pressed, self.sample())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tick_advances_one_console_frame() {
        let mut clock = VirtualDividerClock::new(0);
        assert_eq!(clock.sample(), 0);
        clock.advance_tick();
        // 70,224 cycles = $11250.
        assert_eq!(clock.sample(), 0x12);
        clock.advance_tick();
        // 140,448 = $224A0.
        assert_eq!(clock.sample(), 0x24);
        // 4,096 frames is a whole number of counter wraps (70,224 * 4,096 =
        // 4,389 * 65,536).
        clock.advance_ticks(4094);
        assert_eq!(clock.sample(), 0);
        let mut seeded = VirtualDividerClock::new(0xA57E);
        assert_eq!(seeded.sample(), 0xA5);
        seeded.advance_ticks(4096 * 100);
        assert_eq!(seeded.sample(), 0xA5);
    }

    /// Every DIV byte appears equally often over the clock's 4,096-frame
    /// period, so each masked random check fires at the ROM's rate.
    #[test]
    fn the_clock_visits_every_divider_value_equally() {
        let mut clock = VirtualDividerClock::new(0x1234);
        let mut counts = [0u32; 256];
        for _ in 0..4096 {
            counts[clock.sample() as usize] += 1;
            clock.advance_tick();
        }
        assert!(counts.iter().all(|&count| count == 16), "{counts:?}");
        // The resting walker's `& $3F == 0` on even frames: 1 in 64.
        let mut clock = VirtualDividerClock::new(0);
        let mut rests = 0;
        for _ in 0..4096 {
            rests += u32::from(clock.sample() & 63 == 0);
            clock.advance_ticks(2);
        }
        assert_eq!(rests, 64);
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
        // 193 frames are 13,553,232 cycles: $CE50 after seed $80, so $CE.
        assert_eq!(clock.sample(), 0xCE);
        let before = clock.sample();
        let mut objects = sml2_core::objects::ObjectSim::new(vec![], 0);
        objects.reset_level_start();
        assert_eq!(clock.sample(), before);
        clock.advance_tick();
        assert_eq!(clock.ticks, 194);
    }

    #[test]
    fn the_clock_supplies_map_inputs_across_frames_and_game_transitions() {
        use sml2_core::map_sim::MapSim;

        let mut vclock = VirtualDividerClock::new(0);
        let mut map = MapSim::new();
        let mut saw_even = false;
        let mut saw_odd = false;
        let mut expected_xor = 0u8;

        for _ in 0..40 {
            vclock.advance_tick();
            let input = vclock.map_input(0, 0);
            if input.div & 1 == 0 {
                saw_even = true;
            } else {
                saw_odd = true;
            }
            expected_xor ^= input.div;
            assert_eq!(input.keys_held, 0);
            assert_eq!(input.keys_pressed, 0);
            map.bookkeeping(input.div);
        }
        assert!(saw_even, "virtual clock sampled even parity for map input");
        assert!(saw_odd, "virtual clock sampled odd parity for map input");
        assert_eq!(map.frame_counter, 40);
        assert_eq!(map.keys_xor, expected_xor);
        assert_eq!(map.rand_latch, vclock.sample());

        // Game transitions (player respawn, level reset) do NOT reset or mutate the clock
        let ticks_before = vclock.ticks;
        let mut player = sml2_core::player_sim::PlayerSim::new();
        player.respawn(100, 200);
        let mut objects = sml2_core::objects::ObjectSim::new(vec![], 0);
        objects.reset_level_start();
        assert_eq!(
            vclock.ticks, ticks_before,
            "game transitions do not reset clock"
        );
    }
}
