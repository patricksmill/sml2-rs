//! Shared Wario child fall at ROM `$AB4D`, including counted removal `$B157`.

use crate::actor::{Actor, FrameTick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FallEffect {
    None,
    CountedRemoval,
}

/// Screen movement wraps independently of the child's persistent world position.
fn update(actor: &mut Actor, frame: FrameTick, primary_screen_y: u8) -> FallEffect {
    if !frame.every(2) {
        return FallEffect::None;
    }

    actor.screen.move_y(1);
    let row_below_child = (actor.screen.y & 0xF0).wrapping_add(0x20);
    if row_below_child != primary_screen_y & 0xF0 {
        return FallEffect::None;
    }

    // `$B157` clears every persistent field, including reserved values.
    *actor = Actor::default();
    FallEffect::CountedRemoval
}

/// Slot compatibility and legacy world effects stay at this boundary.
pub(super) fn update_legacy(sim: &mut crate::objects::ObjectSim, st: &mut crate::objects::Staging) {
    let mut child = Actor::from_raw_slot(&st.b);
    let primary = Actor::from_raw_slot(&sim.slots[0].b);
    let effect = update(&mut child, FrameTick(u64::from(sim.fff0)), primary.screen.y);
    st.b = child.to_raw_slot();
    match effect {
        FallEffect::None => {}
        FallEffect::CountedRemoval => {
            sim.boss_delay = 1;
            sim.boss_def_count = sim.boss_def_count.wrapping_sub(1);
            sim.done = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_actor(y: u8) -> Actor {
        let mut actor = Actor::from_raw_slot(&std::array::from_fn(|i| {
            (i as u8).wrapping_mul(7).wrapping_add(3)
        }));
        actor.screen.y = y;
        actor
    }

    #[test]
    fn native_wario_fall_odd_frames_preserve_all_bytes_at_threshold() {
        for frame in [1, 255, 257, u64::MAX] {
            let mut child = seeded_actor(0x60);
            let expected = child.to_raw_slot();
            assert_eq!(update(&mut child, FrameTick(frame), 0x8F), FallEffect::None);
            assert_eq!(child.to_raw_slot(), expected);
        }
    }

    #[test]
    fn native_wario_fall_even_frames_move_only_screen_y() {
        for frame in [0, 254, 256, u64::MAX - 1] {
            for (start_y, end_y) in [(0x3F, 0x40), (0xFF, 0)] {
                let mut child = seeded_actor(start_y);
                let mut expected = child;
                expected.screen.y = end_y;
                assert_eq!(update(&mut child, FrameTick(frame), 0x80), FallEffect::None);
                assert_eq!(child.to_raw_slot(), expected.to_raw_slot());
            }
        }
    }

    #[test]
    fn native_wario_fall_removal_uses_new_row_and_clears_every_byte() {
        for (start_y, primary_y) in [
            (0x5F, 0x8F),
            (0x64, 0x81),
            (0xDF, 0x07),
            (0xFE, 0x1F),
            (0xFF, 0x2A),
        ] {
            let mut child = seeded_actor(start_y);
            assert_eq!(
                update(&mut child, FrameTick(0), primary_y),
                FallEffect::CountedRemoval
            );
            assert_eq!(child, Actor::default());
            assert_eq!(child.to_raw_slot(), [0; 32]);
        }
    }

    #[test]
    fn native_wario_fall_leaving_matching_row_does_not_remove() {
        let mut child = seeded_actor(0x6F);
        let mut expected = child;
        expected.screen.y = 0x70;
        assert_eq!(update(&mut child, FrameTick(0), 0x80), FallEffect::None);
        assert_eq!(child.to_raw_slot(), expected.to_raw_slot());
    }
}
