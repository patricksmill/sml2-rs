use super::*;

fn dirty(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    raw[15] = state;
    raw
}

#[test]
fn handler_0xa27e_idle_and_command_initialization_preserve_whole_slot() {
    for flip in 0u8..=255 {
        for sprite in [0, 14, 59, 255] {
            let mut expected = dirty(0);
            expected[11] = sprite;
            expected[13] = flip;
            // Initialization must return even on a movement tick.
            for frame in [0, 1] {
                let mut actor = Actor::from_raw_slot(&expected);
                let sound = update(&mut actor, FrameTick::new(frame)).unwrap();
                let mut after = expected;
                if sprite != 14 {
                    after[18] = 64;
                    after[15] = if flip & 32 == 0 { 1 } else { 2 };
                }
                assert_eq!(actor.to_raw_slot(), after);
                assert_eq!(sound, if sprite == 14 { None } else { Some(9) });
            }
        }
    }
}

#[test]
fn handler_0xa27e_flight_all_coordinates_cadences_and_directions() {
    for state in [1u8, 2] {
        for x in 0u8..=255 {
            for frame in 0..=255 {
                let mut expected = dirty(state);
                expected[10] = x;
                // Movement follows state even when the flip points elsewhere.
                expected[13] = if state == 1 { 0xFF } else { 0xDF };
                let mut actor = Actor::from_raw_slot(&expected);
                assert_eq!(update(&mut actor, FrameTick::new(frame)), Ok(None));
                if frame & 1 == 0 {
                    expected[10] = if state == 1 {
                        x.wrapping_sub(3)
                    } else {
                        x.wrapping_add(3)
                    };
                    if expected[10] >= 184 {
                        expected[11] = 14;
                        expected[15] = 0;
                    }
                }
                assert_eq!(
                    actor.to_raw_slot(),
                    expected,
                    "state {state}, x {x}, frame {frame}"
                );
            }
        }
    }
}

#[test]
fn handler_0xa27e_unsupported_states_preserve_bytes() {
    for state in 3u8..=255 {
        let raw = dirty(state);
        let mut actor = Actor::from_raw_slot(&raw);
        assert_eq!(update(&mut actor, FrameTick::new(0)), Err(state));
        assert_eq!(actor.to_raw_slot(), raw);
    }
}
