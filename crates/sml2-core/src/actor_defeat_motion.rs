//! Shared defeat hop and rise from bank $16.

use super::{Actor, ContactKind, MotionProfileError};

/// ROM `$5B4AB..$5B4B4`, selected by `$5B465` with cap eight.
pub const DEFEAT_RISE_STEPS: [u8; 9] = [2, 1, 2, 1, 1, 1, 0, 0, 0];

impl Actor {
    /// ROM `$581B9`: lift and invert the defeated actor, then begin rising.
    pub fn begin_defeat_hop(&mut self) {
        self.position.y = self.position.y.wrapping_sub(20);
        self.sprite_flip.toggle_vertical();
        self.motion_phase = 0;
        self.contact_kind = ContactKind::NONE;
        self.state = 1;
    }

    /// ROM `$581CE`: double the source byte before subtracting from world Y.
    /// The transition tests the phase after the picker has advanced it.
    pub fn advance_defeat_rise(&mut self) -> Result<(), MotionProfileError> {
        let distance = self.motion_step(&DEFEAT_RISE_STEPS, 8)?;
        self.position.y = self
            .position
            .y
            .wrapping_sub(u16::from(distance.wrapping_mul(2)));
        if self.motion_phase == 8 {
            self.state = 2;
            self.motion_phase = 0;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirty_slot() -> [u8; 32] {
        std::array::from_fn(|i| (i as u8).wrapping_mul(19).wrapping_add(0xA7))
    }

    #[test]
    fn defeat_hop_all_flip_bits_and_coordinate_wrap_preserve_other_bytes() {
        for flip in 0..=255 {
            for y in [0u16, 1, 19, 20, 255, 256, 65535] {
                let mut expected = dirty_slot();
                expected[3..5].copy_from_slice(&y.to_be_bytes());
                expected[13] = flip;
                let mut actor = Actor::from_raw_slot(&expected);
                actor.begin_defeat_hop();
                expected[3..5].copy_from_slice(&y.wrapping_sub(20).to_be_bytes());
                expected[13] ^= 0x40;
                expected[15] = 1;
                expected[16] = 0;
                expected[18] = 0;
                assert_eq!(actor.to_raw_slot(), expected, "flip={flip:#x}, y={y:#x}");
            }
        }
    }

    #[test]
    fn defeat_rise_exact_rom_profile_and_every_phase_postincrement_transition() {
        // Independent ROM witness, including the terminal index eight byte.
        assert_eq!(DEFEAT_RISE_STEPS, [2, 1, 2, 1, 1, 1, 0, 0, 0]);
        for (phase, distance) in [4u16, 2, 4, 2, 2, 2, 0, 0, 0].into_iter().enumerate() {
            for y in [0u16, 1, 3, 255, 256, 65535] {
                let mut expected = dirty_slot();
                expected[3..5].copy_from_slice(&y.to_be_bytes());
                expected[15] = 1;
                expected[16] = phase as u8;
                let mut actor = Actor::from_raw_slot(&expected);
                actor.advance_defeat_rise().unwrap();
                expected[3..5].copy_from_slice(&y.wrapping_sub(distance).to_be_bytes());
                expected[15] = if phase >= 7 { 2 } else { 1 };
                expected[16] = if phase >= 7 { 0 } else { phase as u8 + 1 };
                assert_eq!(actor.to_raw_slot(), expected, "phase={phase}, y={y:#x}");
            }
        }
    }

    #[test]
    fn defeat_rise_unsupported_phases_return_diagnostic_without_mutation() {
        for phase in 9..=255 {
            let mut slot = dirty_slot();
            slot[16] = phase;
            let mut actor = Actor::from_raw_slot(&slot);
            assert_eq!(
                actor.advance_defeat_rise(),
                Err(MotionProfileError {
                    phase,
                    cap: 8,
                    profile_len: 9,
                })
            );
            assert_eq!(actor.to_raw_slot(), slot);
        }
    }
}
