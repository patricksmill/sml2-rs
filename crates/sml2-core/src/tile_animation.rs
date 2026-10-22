//! Persistent tile animation time: `$A298` / `$A2CD`.
//!
//! Startup clears these bytes; `LoadMarioGFX` changes only the config.
//! `$3042D` disables animation on map return without resetting either byte.

use crate::level::TileAnim;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileAnimationClock {
    pub timer: u8,
    pub phase: u8,
}

impl TileAnimationClock {
    /// One frame that runs the `$0300` frame wait: `$02FF` increments the
    /// byte timer, then the frame's `$3873` (VBlank) copies once it reaches
    /// the period and restarts it at 0. Measured in PyBoy: the frame that
    /// copies reads timer 0, the next 1. A skipped copy still advances time,
    /// including while animation is off.
    pub fn tick(&mut self, config: u8, copy_blocked: bool) -> Option<TileAnim> {
        self.timer = self.timer.wrapping_add(1);
        self.vblank(config, copy_blocked)
    }

    /// `$3873` alone: a frame whose VBlank runs but whose main loop never
    /// reaches the `$0300` wait (a level's first frame) can still copy.
    pub fn vblank(&mut self, config: u8, copy_blocked: bool) -> Option<TileAnim> {
        let anim = TileAnim::from_table_value(config.wrapping_sub(1));
        let copy = !copy_blocked && anim.enabled && self.timer >= anim.speed * 2;
        if copy {
            self.timer = 0;
            self.phase = self.phase.wrapping_add(1) & 3;
        }
        copy.then_some(anim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pumpkin_animation_uses_inherited_bytes_and_increments_before_copying() {
        // `$35EA[6] = $FE`: config $FF, row 14, period 30.
        let mut clock = TileAnimationClock {
            timer: 28,
            phase: 2,
        };
        assert_eq!(clock.tick(0xFF, false), None);
        assert_eq!((clock.timer, clock.phase), (29, 2));
        assert_eq!(clock.tick(0xFF, false).unwrap().row, 14);
        assert_eq!((clock.timer, clock.phase), (0, 3), "the copy frame reads 0");
        for _ in 0..29 {
            assert_eq!(clock.tick(0xFF, false), None);
        }
        assert!(clock.tick(0xFF, false).is_some());
        assert_eq!((clock.timer, clock.phase), (0, 0));
    }

    /// Level 1's first `$04` frame, PyBoy row 586: VBlank copies at timer 109
    /// with no `$0300` wait, so the timer reads 0 and the next frame 1.
    #[test]
    fn a_vblank_without_the_frame_wait_copies_without_advancing() {
        let mut clock = TileAnimationClock {
            timer: 109,
            phase: 0,
        };
        // Level 1's `$4D` config: period 8.
        assert!(clock.vblank(0x4E, false).is_some());
        assert_eq!((clock.timer, clock.phase), (0, 1));
        assert_eq!(clock.tick(0x4E, false), None);
        assert_eq!((clock.timer, clock.phase), (1, 1));
    }

    #[test]
    fn disabled_and_blocked_animation_keep_phase_and_wrap_the_timer_byte() {
        for (config, blocked) in [(0, false), (0xFF, true)] {
            let mut clock = TileAnimationClock {
                timer: 255,
                phase: 3,
            };
            assert_eq!(clock.tick(config, blocked), None);
            assert_eq!((clock.timer, clock.phase), (0, 3));
            assert_eq!(clock.tick(0xFF, false), None);
            assert_eq!((clock.timer, clock.phase), (1, 3));
        }
    }
}
