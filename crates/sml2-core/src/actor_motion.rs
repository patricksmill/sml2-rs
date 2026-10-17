//! Shared boss rise distances from bank 2.

use super::{Actor, MotionProfileError};

/// ROM `$B1FF..$B223` (CPU `$71FF..$7223`).
const RISE_STEPS: [u8; 37] = [
    3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1,
    1, 1, 0, 0, 0,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiseProfile {
    /// ROM `$B1C8`: the complete curve at `$B1FF`.
    Full,
    /// ROM `$B1D6`: the curve starting at `$B20E`.
    Medium,
    /// ROM `$B1E4`: the curve starting at `$B217`.
    Short,
}

impl RiseProfile {
    pub const fn cap(self) -> u8 {
        match self {
            Self::Full => 36,
            Self::Medium => 21,
            Self::Short => 12,
        }
    }

    fn steps(self) -> &'static [u8] {
        let start = match self {
            Self::Full => 0,
            Self::Medium => 15,
            Self::Short => 24,
        };
        &RISE_STEPS[start..]
    }
}

impl Actor {
    /// Read the current rise distance before advancing to the profile cap.
    pub fn rise_step(&mut self, profile: RiseProfile) -> Result<u8, MotionProfileError> {
        self.motion_step(profile.steps(), profile.cap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rise_profiles_match_rom_distances_and_preserve_slot_bytes() {
        // Independently transcribed ROM `$B1FF..$B223`.
        let expected_steps = [
            0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02,
            0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02,
            0x02, 0x01, 0x02, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00,
        ];
        for (profile, start, cap) in [
            (RiseProfile::Full, 0, 36),
            (RiseProfile::Medium, 15, 21),
            (RiseProfile::Short, 24, 12),
        ] {
            assert_eq!(profile.cap(), cap);
            let mut expected = [0xA5; 32];
            expected[16] = 0;
            let mut actor = Actor::from_raw_slot(&expected);
            for phase in 0..=cap {
                assert_eq!(
                    actor.rise_step(profile),
                    Ok(expected_steps[start + usize::from(phase)]),
                    "{profile:?} phase {phase}"
                );
                expected[16] = (phase + 1).min(cap);
                assert_eq!(actor.to_raw_slot(), expected);
            }
            for _ in 0..3 {
                assert_eq!(actor.rise_step(profile), Ok(0));
                assert_eq!(actor.to_raw_slot(), expected);
            }
        }
    }

    #[test]
    fn rise_profiles_reject_out_of_table_phases_without_mutation() {
        for (profile, cap) in [
            (RiseProfile::Full, 36),
            (RiseProfile::Medium, 21),
            (RiseProfile::Short, 12),
        ] {
            for phase in [cap + 1, 255] {
                let mut slot = [0xA5; 32];
                slot[16] = phase;
                let mut actor = Actor::from_raw_slot(&slot);
                assert_eq!(
                    actor.rise_step(profile),
                    Err(MotionProfileError {
                        phase,
                        cap,
                        profile_len: usize::from(cap) + 1,
                    })
                );
                assert_eq!(actor.to_raw_slot(), slot);
            }
        }
    }
}
