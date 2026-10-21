use super::pig2_motion::update;
use crate::actor::Actor;

// Literal bank-2 `$B1A3` and `$B214` source curves.
const FALL: [u8; 22] = [
    0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3,
];
const RISE: [u8; 15] = [3, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0];

fn slot(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(13));
    raw[5] = 11;
    raw[8] = 6; // Three HP pairs: horizontal speed 1.
    raw[9] = 144;
    raw[10] = 88;
    raw[11] = 32;
    raw[15] = state;
    raw[16] = 0;
    raw[24] = 0;
    raw[28] = 0;
    raw
}

#[test]
fn pig2_hop_uses_both_15_step_curves_and_delays_crouch_sprite_until_next_update() {
    for (state, dx) in [(4, 255u8), (5, 1)] {
        let mut expected = slot(state);
        let mut actor = Actor::from_raw_slot(&expected);
        for (leg, curve) in [(0, &RISE[..]), (1, &FALL[..15])] {
            for (phase, &dy) in curve.iter().enumerate() {
                expected[10] = expected[10].wrapping_add(dx);
                expected[9] = if leg == 0 {
                    expected[9].wrapping_sub(dy)
                } else {
                    expected[9].wrapping_add(dy)
                };
                expected[16] = (phase as u8 + 1) % 15;
                let complete = phase == 14;
                expected[24] = if complete { leg ^ 1 } else { leg };
                if complete && leg == 1 {
                    expected[28] = 8;
                }
                assert_eq!(
                    update(&mut actor, &FALL),
                    Ok((complete && leg == 1).then_some(11))
                );
                assert_eq!(
                    actor.to_raw_slot(),
                    expected,
                    "state {state}, leg {leg}, phase {phase}"
                );
            }
        }
        // The ROM completes by phase, without clamping Y to $90.
        assert_eq!(actor.screen.y, 144);
        for remaining in (1..=7).rev() {
            expected[11] = 40;
            expected[28] = remaining;
            assert_eq!(update(&mut actor, &FALL), Ok(None));
            assert_eq!(actor.to_raw_slot(), expected);
        }
        expected[11] = 32;
        expected[28] = 0;
        expected[10] = expected[10].wrapping_add(dx);
        expected[9] -= 3;
        expected[16] = 1;
        assert_eq!(update(&mut actor, &FALL), Ok(None));
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn pig2_wall_turn_returns_before_advancing_vertical_motion() {
    for (state, x, next_x) in [(4, 16, 15), (5, 159, 160)] {
        let mut expected = slot(state);
        expected[10] = x;
        expected[16] = 7;
        let mut actor = Actor::from_raw_slot(&expected);
        expected[10] = next_x;
        expected[15] ^= 1;
        expected[13] ^= 0x20;
        assert_eq!(update(&mut actor, &FALL), Ok(Some(11)));
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn pig2_descent_finishes_by_phase_without_floor_clamping() {
    let mut expected = slot(4);
    expected[9] = 150;
    expected[16] = 14;
    expected[24] = 1;
    let mut actor = Actor::from_raw_slot(&expected);
    expected[9] = 153;
    expected[10] -= 1;
    expected[16] = 0;
    expected[24] = 0;
    expected[28] = 8;
    assert_eq!(update(&mut actor, &FALL), Ok(Some(11)));
    assert_eq!(actor.to_raw_slot(), expected);
}
