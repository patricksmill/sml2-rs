//! Bank 2 `$7339` sprite cycle shared by the Wario handlers.

use super::{ObjectSim, Staging, B_ANIMFRAME, B_SPRITE};

/// ROM `$B356..$B359` (CPU `$7356..$7359`).
const SPRITE_DELTAS: [u8; 4] = [0xFE, 0x01, 0xFF, 0x02];

impl ObjectSim {
    /// `$B339`: on every fourth frame, advance the cycle and add its sprite delta.
    /// The caller supplies the ROM's reachable cycle index, `B_ANIMFRAME < 4`.
    pub(super) fn boss_sprite_cycle_b339(&self, s: &mut Staging) {
        if self.fff0 & 3 != 0 {
            return;
        }
        s.b[B_ANIMFRAME] = s.b[B_ANIMFRAME].wrapping_add(1);
        if s.b[B_ANIMFRAME] == 4 {
            s.b[B_ANIMFRAME] = 0;
        }
        s.b[B_SPRITE] = s.b[B_SPRITE].wrapping_add(SPRITE_DELTAS[s.b[B_ANIMFRAME] as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_0x0b339_frame_gate_preserves_staging() {
        let mut sim = ObjectSim::new(vec![], 0);
        // All 192 non-multiple-of-four frame bytes take the ROM's early return.
        for frame in 0..=u8::MAX {
            if frame & 3 == 0 {
                continue;
            }
            sim.fff0 = frame;
            let mut s = Staging::default();
            s.b = [0xA5; 32];
            s.b[B_ANIMFRAME] = 3;
            let before = s.b;
            sim.boss_sprite_cycle_b339(&mut s);
            assert_eq!(s.b, before, "frame {frame:#04x}");
        }
    }

    #[test]
    fn helper_0x0b339_four_step_cycle_and_wrap() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut s = Staging::default();
        s.b = [0xA5; 32];
        s.b[B_ANIMFRAME] = 0;
        s.b[B_SPRITE] = 0x30;
        // The increment precedes indexing $7356: +1, -1, +2, -2.
        for (frame, index, sprite) in [
            (0x00, 1, 0x31),
            (0x04, 2, 0x30),
            (0x80, 3, 0x32),
            (0xFC, 0, 0x30),
        ] {
            sim.fff0 = frame;
            let mut expected = s.b;
            expected[B_ANIMFRAME] = index;
            expected[B_SPRITE] = sprite;
            sim.boss_sprite_cycle_b339(&mut s);
            assert_eq!(s.b, expected, "frame {frame:#04x}");
        }
    }

    #[test]
    fn helper_0x0b339_sprite_arithmetic_wraps() {
        let sim = ObjectSim::new(vec![], 0);
        // Exercise both carry directions for each literal table delta.
        for (index, sprite, next_index, next_sprite) in [
            (0, 0xFF, 1, 0x00),
            (1, 0x00, 2, 0xFF),
            (2, 0xFF, 3, 0x01),
            (3, 0x00, 0, 0xFE),
        ] {
            let mut s = Staging::default();
            s.b[B_ANIMFRAME] = index;
            s.b[B_SPRITE] = sprite;
            sim.boss_sprite_cycle_b339(&mut s);
            assert_eq!(s.b[B_ANIMFRAME], next_index);
            assert_eq!(s.b[B_SPRITE], next_sprite);
        }
    }
}
