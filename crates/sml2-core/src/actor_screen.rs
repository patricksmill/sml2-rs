//! Screen-space targeting shared by native actors.

use super::{Actor, Facing, ScreenPosition, SpriteFlipFlags};

impl ScreenPosition {
    /// ROM `$B2DB`: magnitude of the signed, wrapping screen displacement.
    pub fn horizontal_distance_to(self, target_x: u8) -> u8 {
        (target_x.wrapping_sub(self.x) as i8).unsigned_abs()
    }
}

impl Actor {
    /// ROM `$B27C`: face an unsigned screen coordinate, including right on ties.
    /// Replaces all sprite flip bits; caller-specific state is left to callers.
    pub fn face_toward_screen(&mut self, target_x: u8) -> Facing {
        let facing = if target_x >= self.screen.x {
            Facing::RIGHT
        } else {
            Facing::LEFT
        };
        self.sprite_flip = SpriteFlipFlags::from_bits(if facing.is_right() { 0x20 } else { 0 });
        facing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_targeting_distance_source_edges() {
        for (x, target_x, expected) in [
            (0, 0, 0),
            (255, 255, 0),
            (0, 255, 1),
            (255, 0, 1),
            (127, 128, 1),
            (128, 127, 1),
            (0, 127, 127),
            (127, 0, 127),
            (0, 128, 128),
            (128, 0, 128),
            (255, 127, 128),
            (127, 255, 128),
            (0, 129, 127),
            (129, 0, 127),
            (254, 1, 3),
            (1, 254, 3),
        ] {
            let screen = ScreenPosition { x, y: 173 };
            assert_eq!(screen.horizontal_distance_to(target_x), expected);
        }
    }

    #[test]
    fn screen_targeting_distance_matches_shortest_circle_arc_for_all_coordinates() {
        for x in 0..=u8::MAX {
            for target_x in 0..=u8::MAX {
                let direct = u16::from(x.abs_diff(target_x));
                let expected = direct.min(256 - direct) as u8;
                let screen = ScreenPosition { x, y: 219 };
                assert_eq!(screen.horizontal_distance_to(target_x), expected);
            }
        }
    }

    #[test]
    fn screen_targeting_face_source_edges_preserve_other_slot_bytes() {
        for (x, target_x, expected_facing, expected_flip) in [
            (0, 0, Facing::RIGHT, 0x20),
            (255, 255, Facing::RIGHT, 0x20),
            (0, 255, Facing::RIGHT, 0x20),
            (255, 0, Facing::LEFT, 0),
            (127, 128, Facing::RIGHT, 0x20),
            (128, 127, Facing::LEFT, 0),
            (0, 128, Facing::RIGHT, 0x20),
            (128, 0, Facing::LEFT, 0),
            (254, 1, Facing::LEFT, 0),
            (1, 254, Facing::RIGHT, 0x20),
        ] {
            for old_flip in 0..=u8::MAX {
                let mut raw = core::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
                raw[10] = x;
                raw[13] = old_flip;
                let mut actor = Actor::from_raw_slot(&raw);
                assert_eq!(actor.face_toward_screen(target_x), expected_facing);
                raw[13] = expected_flip;
                assert_eq!(
                    actor.to_raw_slot(),
                    raw,
                    "x={x}, target={target_x}, flip={old_flip}"
                );
            }
        }
    }
}
