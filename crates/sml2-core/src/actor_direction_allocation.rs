use super::Actor;

impl Actor {
    /// ROM `$59136`: reserve the first free direction, searching circularly
    /// from the divider's low three bits. A full occupancy mask is unchanged.
    pub fn allocate_direction(&mut self, divider: u8) -> Option<u8> {
        let start = divider & 7;
        for offset in 0..8 {
            let direction = (start + offset) & 7;
            let bit = 1 << direction;
            if self.visibility_mask & bit == 0 {
                self.visibility_mask |= bit;
                return Some(direction);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent instruction-level oracle for ROM $59136..$59179, checked
    // against baserom.gb. RRCA at $5914D/$59156 exposes the occupied bit;
    // SET 7,A at $59167 reserves it before $5916E restores the mask rotation.
    // Only the exhaustion branch ($5915D) requests removal; the adapter owns it.
    fn source_59136(mask: u8, divider: u8) -> (Option<u8>, u8) {
        let mut a = mask;
        let mut e = divider & 7;
        let mut b = 8u8;
        let c = 8 - e;
        let mut carry = false;
        for _ in 0..=e {
            carry = a & 1 != 0;
            a = a.rotate_right(1);
        }
        if carry {
            b -= 1;
            loop {
                e = e.wrapping_add(1);
                carry = a & 1 != 0;
                a = a.rotate_right(1);
                if !carry {
                    break;
                }
                b -= 1;
                if b == 0 {
                    return (None, mask);
                }
            }
        }
        b -= 1;
        a |= 0x80;
        for _ in 0..b + c {
            a = a.rotate_right(1);
        }
        (Some(e & 7), a)
    }

    fn dirty_slot(mask: u8) -> [u8; 32] {
        let mut slot = std::array::from_fn(|index| (index as u8).wrapping_mul(37) ^ 0xA5);
        slot[31] = mask;
        slot
    }

    #[test]
    fn direction_allocation_matches_source_for_every_mask_and_start() {
        for mask in 0..=255u8 {
            for divider in 0..8 {
                let slot = dirty_slot(mask);
                let mut actor = Actor::from_raw_slot(&slot);
                assert_eq!(actor.to_raw_slot(), slot, "input boundary mask={mask:#04x}");
                let (direction, expected_mask) = source_59136(mask, divider);
                assert_eq!(
                    actor.allocate_direction(divider),
                    direction,
                    "mask={mask:#04x} divider={divider}"
                );
                let mut expected_slot = slot;
                expected_slot[31] = expected_mask;
                assert_eq!(
                    actor.to_raw_slot(),
                    expected_slot,
                    "mask={mask:#04x} divider={divider}"
                );
            }
        }
    }

    #[test]
    fn direction_allocation_rom_branch_witnesses_and_divider_aliases() {
        // Literal outcomes of the source's initial clear-bit branch, repeated
        // occupied-bit branch, wrap across bit 7, and eight-bit exhaustion.
        let cases = [
            (0x00, 0, Some(0), 0x01),
            (0x00, 7, Some(7), 0x80),
            (0x38, 3, Some(6), 0x78),
            (0xC1, 6, Some(1), 0xC3),
            (0xFE, 7, Some(0), 0xFF),
            (0x7F, 0, Some(7), 0xFF),
            (0xFF, 3, None, 0xFF),
        ];
        for (mask, start, selected, result) in cases {
            for high_bits in (0..=0xF8).step_by(8) {
                let slot = dirty_slot(mask);
                let mut actor = Actor::from_raw_slot(&slot);
                assert_eq!(actor.allocate_direction(high_bits | start), selected);
                let mut expected = slot;
                expected[31] = result;
                assert_eq!(actor.to_raw_slot(), expected);
            }
        }
    }

    #[test]
    fn direction_allocation_reservations_persist_until_full() {
        let mut actor = Actor::from_raw_slot(&dirty_slot(0));
        for (direction, mask) in [
            (6, 0x40),
            (7, 0xC0),
            (0, 0xC1),
            (1, 0xC3),
            (2, 0xC7),
            (3, 0xCF),
            (4, 0xDF),
            (5, 0xFF),
        ] {
            assert_eq!(actor.allocate_direction(6), Some(direction));
            assert_eq!(actor.to_raw_slot()[31], mask);
        }
        let full = actor;
        assert_eq!(actor.allocate_direction(6), None);
        assert_eq!(actor, full);
    }
}
