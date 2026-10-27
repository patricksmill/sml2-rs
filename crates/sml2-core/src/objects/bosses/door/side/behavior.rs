//! Boss anim 7, ROM `$9824`: choose a side, then run shared door motion.
use super::central_door::{self, Effects, Inputs, UpdateError};
use crate::actor::Actor;

pub(super) fn update(
    actor: &mut Actor,
    input: Inputs<'_>,
    parity: u8,
    slot: u8,
) -> Result<Effects, UpdateError> {
    if actor.state == 0 {
        if input.sequence_active && input.door_phase & 1 != 0 {
            let matching_slot = if (input.door_phase == 1) == (parity == 0) {
                0x20
            } else {
                0x60
            };
            if slot == matching_slot {
                actor.state = 1;
                actor.motion_leg = 80;
                actor.timer = 64;
                actor.sprite = 25;
            }
        }
        return Ok(Effects::default());
    }
    central_door::update_motion(
        actor,
        input.remove_mode,
        input.frame,
        input.fall_profile,
        132,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::FrameTick;

    fn input(phase: u8, active: bool) -> Inputs<'static> {
        Inputs {
            remove_mode: 0,
            frame: FrameTick::new(1),
            fall_profile: &[0; 22],
            door_phase: phase,
            sequence_active: active,
        }
    }

    #[test]
    fn side_door_exact_side_selection_and_motion_distance_preserve_slot_bytes() {
        for phase in 0..=255 {
            for parity in [0, 1, 255] {
                for slot in [0, 0x20, 0x40, 0x60] {
                    let mut expected = [0xA5; 32];
                    expected[15] = 0;
                    let mut actor = Actor::from_raw_slot(&expected);
                    update(&mut actor, input(phase, true), parity, slot).unwrap();
                    let wanted = if phase == 1 {
                        if parity == 0 {
                            0x20
                        } else {
                            0x60
                        }
                    } else if parity == 0 {
                        0x60
                    } else {
                        0x20
                    };
                    if phase & 1 != 0 && slot == wanted {
                        expected[15] = 1;
                        expected[24] = 80;
                        expected[28] = 64;
                        expected[11] = 25;
                    }
                    assert_eq!(actor.to_raw_slot(), expected);
                }
            }
        }
        let raw = [0; 32];
        let mut actor = Actor::from_raw_slot(&raw);
        update(&mut actor, input(1, false), 0, 0x20).unwrap();
        assert_eq!(actor.to_raw_slot(), raw);
    }

    #[test]
    fn side_door_rise_uses_motion_distance_not_hit_reaction() {
        let mut raw = [0xA5; 32];
        raw[15] = 2;
        raw[24] = 2;
        raw[9] = 1;
        let mut actor = Actor::from_raw_slot(&raw);
        let result = update(&mut actor, input(1, true), 0, 0x20).unwrap();
        raw[9] = 255;
        raw[24] = 0;
        raw[15] = 3;
        raw[28] = 32;
        assert_eq!(actor.to_raw_slot(), raw);
        assert_eq!(result.carry_dy, None);
    }

    #[test]
    fn side_door_has_its_own_sill_and_no_reward_override() {
        for y in [116, 131, 132, 255] {
            let mut raw = [0xA5; 32];
            raw[15] = 4;
            raw[9] = y;
            raw[16] = 0;
            let mut actor = Actor::from_raw_slot(&raw);
            let mut inputs = input(1, false);
            inputs.remove_mode = 2;
            update(&mut actor, inputs, 0, 0x20).unwrap();
            if y >= 132 {
                raw[9] = 132;
                raw[15] = 0;
            } else {
                raw[16] = 1;
            }
            assert_eq!(actor.to_raw_slot(), raw);
        }
    }
}
