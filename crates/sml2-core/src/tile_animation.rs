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
    /// `$3873` runs in VBlank before `$02FF` increments the byte timer.
    /// A skipped copy still advances time, including while animation is off.
    pub fn tick(&mut self, config: u8, copy_blocked: bool) -> Option<TileAnim> {
        let anim = TileAnim::from_table_value(config.wrapping_sub(1));
        let copy = !copy_blocked && anim.enabled && self.timer >= anim.speed * 2;
        if copy {
            self.timer = 0;
            self.phase = self.phase.wrapping_add(1) & 3;
        }
        self.timer = self.timer.wrapping_add(1);
        copy.then_some(anim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pumpkin_animation_uses_inherited_bytes_and_checks_before_incrementing() {
        // `$35EA[6] = $FE`: config $FF, row 14, period 30.
        let mut clock = TileAnimationClock {
            timer: 29,
            phase: 2,
        };
        assert_eq!(clock.tick(0xFF, false), None);
        assert_eq!((clock.timer, clock.phase), (30, 2));
        assert_eq!(clock.tick(0xFF, false).unwrap().row, 14);
        assert_eq!((clock.timer, clock.phase), (1, 3));
        for _ in 0..29 {
            assert_eq!(clock.tick(0xFF, false), None);
        }
        assert!(clock.tick(0xFF, false).is_some());
        assert_eq!((clock.timer, clock.phase), (1, 0));
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
