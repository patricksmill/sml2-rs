use super::{Actor, FrameTick};

impl Actor {
    /// ROM `$B3B9/$B3C0`: advance sequential sprites every second/fourth tick.
    /// Only an incremented phase equal to four resets the sprite sequence.
    pub fn animate_four_frame_cycle(&mut self, frame: FrameTick, period: u64) {
        if !frame.every(period) {
            return;
        }
        self.animation_frame = self.animation_frame.wrapping_add(1);
        if self.animation_frame == 4 {
            self.animation_frame = 0;
            self.sprite = self.sprite.wrapping_sub(3);
        } else {
            self.sprite = self.sprite.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirty_slot(phase: u8, sprite: u8) -> [u8; 32] {
        let mut slot = [
            0x91, 0x82, 0xF3, 0x74, 0x65, 0xD6, 0x47, 0x38, 0xE9, 0x1A, 0xAB, 0xBC, 0xCD, 0xFF,
            0xEF, 0xF0, 0x81, 0x92, 0xA3, 0xB4, 0xC5, 0xD6, 0xE7, 0xF8, 0x89, 0x9A, 0xAB, 0xBC,
            0xCD, 0xDE, 0xEF, 0xF1,
        ];
        slot[11] = sprite;
        slot[26] = phase;
        slot
    }

    #[test]
    fn four_frame_cycle_phase_and_sprite_boundaries_preserve_dirty_slot() {
        // Literal outcomes from INC phase / CP 4 / SUB 3 or INC sprite.
        let phases = [
            (0, 1, false),
            (1, 2, false),
            (2, 3, false),
            (3, 0, true),
            (4, 5, false),
            (127, 128, false),
            (254, 255, false),
            (255, 0, false),
        ];
        let sprites = [
            (0, 1, 253),
            (1, 2, 254),
            (2, 3, 255),
            (3, 4, 0),
            (254, 255, 251),
            (255, 0, 252),
        ];
        for period in [2, 4] {
            for (phase, next_phase, resets) in phases {
                for (sprite, incremented, subtracted) in sprites {
                    let slot = dirty_slot(phase, sprite);
                    let mut actor = Actor::from_raw_slot(&slot);
                    actor.animate_four_frame_cycle(FrameTick(256), period);
                    let mut expected = slot;
                    expected[26] = next_phase;
                    expected[11] = if resets { subtracted } else { incremented };
                    assert_eq!(
                        actor.to_raw_slot(),
                        expected,
                        "period={period} phase={phase} sprite={sprite}"
                    );
                }
            }
        }
    }

    #[test]
    fn four_frame_cycle_second_and_fourth_tick_gates() {
        let ticks = [
            (0, true, true),
            (1, false, false),
            (2, true, false),
            (3, false, false),
            (4, true, true),
            (5, false, false),
            (6, true, false),
            (7, false, false),
            (255, false, false),
            (256, true, true),
            (u64::MAX, false, false),
        ];
        for (tick, cadence_two, cadence_four) in ticks {
            for (period, advances) in [(2, cadence_two), (4, cadence_four)] {
                for (phase, sprite, next_phase, next_sprite) in [(3, 0, 0, 253), (255, 255, 0, 0)] {
                    let slot = dirty_slot(phase, sprite);
                    let mut actor = Actor::from_raw_slot(&slot);
                    actor.animate_four_frame_cycle(FrameTick(tick), period);
                    let mut expected = slot;
                    if advances {
                        expected[26] = next_phase;
                        expected[11] = next_sprite;
                    }
                    assert_eq!(
                        actor.to_raw_slot(),
                        expected,
                        "period={period} tick={tick} phase={phase}"
                    );
                }
            }
        }
    }

    #[test]
    fn four_frame_cycle_returns_to_start_after_four_advances() {
        for period in [2, 4] {
            let slot = dirty_slot(0, 254);
            let mut actor = Actor::from_raw_slot(&slot);
            for (index, phase, sprite) in [(0, 1, 255), (1, 2, 0), (2, 3, 1), (3, 0, 254)] {
                actor.animate_four_frame_cycle(FrameTick(index * period), period);
                let mut expected = slot;
                expected[26] = phase;
                expected[11] = sprite;
                assert_eq!(actor.to_raw_slot(), expected);
            }
        }
    }
}
