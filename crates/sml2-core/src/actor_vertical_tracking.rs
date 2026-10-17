//! Shared player Y tracking, ROM `$590E9` and its `$59125` profile.
use super::Actor;

/// Signed bytes at ROM `$59125..$59135`, indexed after acceleration.
pub const VERTICAL_TRACKING_Y_SPEED: [i8; 17] =
    [-3, -2, -2, -1, -2, -1, -1, 0, 0, 0, 1, 1, 2, 1, 2, 2, 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalTrackingError {
    /// Advanced index that would read beyond the documented ROM profile.
    pub leg: u8,
}

impl Actor {
    /// Accelerate toward player Y, including downward acceleration on equality.
    /// Only exact endpoints stop acceleration; other values wrap as bytes.
    /// Return the unsigned movement magnitude (also consumed by ROM `$5902D`).
    /// Invalid profile indices preserve the advanced leg but do not move Y.
    pub fn track_player_y(&mut self, player_y: u16) -> Result<u8, VerticalTrackingError> {
        if player_y < self.position.y {
            if self.motion_leg != 0 {
                self.motion_leg = self.motion_leg.wrapping_sub(1);
            }
        } else if self.motion_leg != 16 {
            self.motion_leg = self.motion_leg.wrapping_add(1);
        }
        let speed = *VERTICAL_TRACKING_Y_SPEED
            .get(usize::from(self.motion_leg))
            .ok_or(VerticalTrackingError {
                leg: self.motion_leg,
            })? as u8;
        let magnitude = if self.motion_leg < 8 {
            speed.wrapping_neg()
        } else {
            speed
        };
        if self.motion_leg < 8 {
            self.position.y = self.position.y.wrapping_sub(u16::from(magnitude));
        } else {
            self.position.y = self.position.y.wrapping_add(u16::from(magnitude));
        }
        Ok(magnitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(y: u16, leg: u8) -> [u8; 32] {
        let mut slot = std::array::from_fn(|i| (i as u8).wrapping_mul(37));
        [slot[3], slot[4]] = y.to_be_bytes();
        slot[24] = leg;
        slot
    }

    #[test]
    fn vertical_tracking_all_rom_profile_bytes_at_actor_boundary() {
        // Independent literal ROM `$59125..$59135`, checked after acceleration.
        let rom = [
            0xfd, 0xfe, 0xfe, 0xff, 0xfe, 0xff, 0xff, 0, 0, 0, 1, 1, 2, 1, 2, 2, 3,
        ];
        for (leg, &byte) in rom.iter().enumerate() {
            for above in [false, true] {
                let initial_leg = if above {
                    leg as u8 + 1
                } else {
                    (leg as u8).wrapping_sub(1)
                };
                let mut expected = slot(0x1200, initial_leg);
                let mut actor = Actor::from_raw_slot(&expected);
                let magnitude = if leg < 8 {
                    (byte as u8).wrapping_neg()
                } else {
                    byte
                };
                assert_eq!(
                    actor.track_player_y(if above { 0x11ff } else { 0x1201 }),
                    Ok(magnitude)
                );
                expected[24] = leg as u8;
                [expected[3], expected[4]] = 0x1200u16
                    .wrapping_add_signed(i16::from(byte as i8))
                    .to_be_bytes();
                assert_eq!(actor.to_raw_slot(), expected, "leg {leg}, above {above}");
            }
        }
    }

    #[test]
    fn vertical_tracking_equality_caps_wrap_and_magnitude() {
        for (y, player, leg, next, next_y, magnitude) in [
            (100, 100, 9, 10, 101, 1), // Equality accelerates downward.
            (100, 99, 0, 0, 97, 3),
            (100, 101, 16, 16, 103, 3),
            (100, 100, 255, 0, 97, 3), // Byte increment wraps before lookup.
            (0, 0, 255, 0, 65533, 3),
            (65535, 65535, 16, 16, 2, 3),
            (256, 255, 1, 0, 253, 3), // Full world coordinate comparison.
            (255, 256, 15, 16, 258, 3),
        ] {
            let mut expected = slot(y, leg);
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(actor.track_player_y(player), Ok(magnitude));
            expected[24] = next;
            [expected[3], expected[4]] = (next_y as u16).to_be_bytes();
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }

    #[test]
    fn vertical_tracking_rejects_advanced_out_of_profile_without_clamping() {
        for (player, leg, advanced) in [(99, 255, 254), (101, 17, 18), (100, 254, 255)] {
            let mut expected = slot(100, leg);
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(
                actor.track_player_y(player),
                Err(VerticalTrackingError { leg: advanced })
            );
            expected[24] = advanced;
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}
