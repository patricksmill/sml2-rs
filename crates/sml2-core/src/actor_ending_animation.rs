//! Final-Wario sprite animation, including its reset lookup quirk.

use super::{Actor, FrameTick};

/// The source selected data beyond the supported four-byte sprite delta table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndingBobError {
    pub lookup_index: u8,
}

impl std::fmt::Display for EndingBobError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "ending bob lookup index {} exceeds the four-entry ROM table",
            self.lookup_index
        )
    }
}

impl std::error::Error for EndingBobError {}

impl Actor {
    /// ROM `$B35A`: advance every fourth frame, bob the sprite, and turn at phase 3.
    /// On reset, the source indexes with the full flip byte after clearing X flip.
    /// Unsupported indices retain those phase/flip writes but leave the sprite intact.
    pub fn animate_ending_bob(&mut self, frame: FrameTick) -> Result<(), EndingBobError> {
        if !frame.every(4) {
            return Ok(());
        }
        self.animation_frame = self.animation_frame.wrapping_add(1);
        let lookup_index = if self.animation_frame == 4 {
            self.animation_frame = 0;
            self.sprite_flip.set_horizontal(false);
            self.sprite_flip.bits()
        } else {
            self.animation_frame
        };
        // ROM `$B388..$B38B`: FF 01 FF 01.
        const DELTAS: [i8; 4] = [-1, 1, -1, 1];
        let delta = DELTAS
            .get(usize::from(lookup_index))
            .ok_or(EndingBobError { lookup_index })?;
        self.sprite = self.sprite.wrapping_add_signed(*delta);
        if self.animation_frame == 3 {
            self.sprite_flip.set_horizontal(true);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::SpriteFlipFlags;

    fn slot(phase: u8, sprite: u8, flags: u8) -> [u8; 32] {
        let mut slot = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(11));
        slot[26] = phase;
        slot[11] = sprite;
        slot[13] = flags;
        slot
    }

    #[test]
    fn ending_bob_matches_literal_rom_phases_cadence_and_signed_wrap() {
        // Independent ROM `$B388..$B38B`: FF 01 FF 01. Increment precedes lookup.
        for (phase, sprite, flags, next_phase, next_sprite, next_flags) in [
            (0, 255, 0xD7, 1, 0, 0xD7),
            (1, 0, 0xF7, 2, 255, 0xF7),
            (2, 255, 0xD7, 3, 0, 0xF7),
            (3, 0, 0x20, 0, 255, 0),
            (255, 0, 0xF7, 0, 255, 0xF7),
        ] {
            for frame in 0..=260 {
                let before = slot(phase, sprite, flags);
                let mut actor = Actor::from_raw_slot(&before);
                assert_eq!(actor.animate_ending_bob(FrameTick(frame)), Ok(()));
                let mut expected = before;
                if frame & 3 == 0 {
                    expected[26] = next_phase;
                    expected[11] = next_sprite;
                    expected[13] = next_flags;
                }
                assert_eq!(
                    actor.to_raw_slot(),
                    expected,
                    "phase {phase}, frame {frame}"
                );
            }
        }
    }

    #[test]
    fn ending_bob_reset_uses_remaining_full_flip_byte_as_table_index() {
        // Both horizontal orientations select literal FF/01/FF/01 by remaining bits.
        for (index, expected_sprite) in [(0, 99), (1, 101), (2, 99), (3, 101)] {
            for horizontal in [0, 0x20] {
                let mut expected = slot(3, 100, index | horizontal);
                let mut actor = Actor::from_raw_slot(&expected);
                assert_eq!(actor.animate_ending_bob(FrameTick(4)), Ok(()));
                expected[26] = 0;
                expected[13] = index;
                expected[11] = expected_sprite;
                assert_eq!(actor.to_raw_slot(), expected);
            }
        }
    }

    #[test]
    fn ending_bob_rejects_unsupported_phase_after_source_prefix_writes() {
        for phase in 4..=254 {
            let mut expected = slot(phase, 0xA5, 0xF7);
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(actor.animate_ending_bob(FrameTick(1)), Ok(()));
            assert_eq!(actor.to_raw_slot(), expected);
            assert_eq!(
                actor.animate_ending_bob(FrameTick(4)),
                Err(EndingBobError {
                    lookup_index: phase + 1
                })
            );
            expected[26] = phase + 1;
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }

    #[test]
    fn ending_bob_rejects_unsupported_reset_flags_after_clearing_horizontal() {
        for flags in 0..=255 {
            let index = flags & !0x20;
            if index < 4 {
                continue;
            }
            let mut expected = slot(3, 0xA5, flags);
            let mut actor = Actor::from_raw_slot(&expected);
            assert_eq!(
                actor.animate_ending_bob(FrameTick(4)),
                Err(EndingBobError {
                    lookup_index: index
                })
            );
            expected[26] = 0;
            expected[13] = index;
            assert_eq!(actor.to_raw_slot(), expected);
        }
        assert!(EndingBobError { lookup_index: 64 }
            .to_string()
            .contains("64"));
    }

    #[test]
    fn ending_bob_horizontal_setter_preserves_all_other_bits() {
        for value in 0..=255 {
            let mut flags = SpriteFlipFlags::from_bits(value);
            flags.set_horizontal(true);
            assert_eq!(flags.bits(), value | 0x20);
            flags.set_horizontal(true);
            assert_eq!(flags.bits(), value | 0x20);
            flags.set_horizontal(false);
            assert_eq!(flags.bits(), value & !0x20);
            flags.set_horizontal(false);
            assert_eq!(flags.bits(), value & !0x20);
        }
    }
}
