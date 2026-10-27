//! Door boss mouth states 5/6: ROM `$97CF..$97F5`, bank 2.
use crate::actor::{Actor, FrameTick};

pub(super) fn update(actor: &mut Actor, frame: FrameTick) {
    match actor.state {
        5 if frame.every(4) => {
            actor.sprite = actor.sprite.wrapping_add(1);
            if actor.sprite == 25 {
                actor.sprite = 14;
                actor.state = 6;
                actor.motion_leg = 48;
            }
        }
        6 => {
            actor.motion_leg = actor.motion_leg.wrapping_sub(2);
            if actor.motion_leg == 0 {
                actor.state = 7;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn door_boss_mouth_frame_gate_sprite_wrap_and_motion_distance_byte() {
        for frame in 0..=255 {
            for sprite in [0u8, 20, 23, 24, 25, 255] {
                let mut expected = [0xA5; 32];
                expected[15] = 5;
                expected[11] = sprite;
                let mut actor = Actor::from_raw_slot(&expected);
                update(&mut actor, FrameTick::new(frame));
                if frame % 4 == 0 {
                    expected[11] = sprite.wrapping_add(1);
                    if sprite == 24 {
                        expected[11] = 14;
                        expected[15] = 6;
                        expected[24] = 48; // `$97E6`: ldh [$FFE8], a.
                    }
                }
                assert_eq!(actor.to_raw_slot(), expected);
            }
        }
    }

    #[test]
    fn door_boss_mouth_hold_reads_motion_distance_preserving_hit_reaction() {
        for distance in 0..=255u8 {
            let mut expected = [0xA5; 32];
            expected[15] = 6;
            expected[24] = distance;
            let mut actor = Actor::from_raw_slot(&expected);
            update(&mut actor, FrameTick::new(1));
            expected[24] = distance.wrapping_sub(2);
            if distance == 2 {
                expected[15] = 7;
            }
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}
