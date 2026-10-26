use super::*;

type Entry = fn(&mut ObjectSim, &mut Staging);

#[test]
fn platform_patrol_wrapping_reversals_and_riders() {
    for entry in [
        ObjectSim::handler_0x58c41 as Entry,
        ObjectSim::handler_0x58c52,
    ] {
        // ROM $58C85..$58CDE: increment before equality; long caps double as bytes.
        for (state, x, span, phase, next_state, next_phase, next_x) in [
            (1, 0, 0, 255, 4, 0, 65535),
            (2, 65535, 0, 255, 3, 0, 0),
            (3, 0, 128, 255, 4, 0, 65535),
            (4, 65535, 129, 1, 3, 0, 0),
            (3, 0, 129, 255, 3, 0, 65535),
            (4, 65535, 129, 2, 4, 3, 0),
        ] {
            for rider in [false, true] {
                let mut sim = ObjectSim::new(vec![], 0);
                sim.fff0 = 3;
                sim.facing = 99;
                sim.mario.x = x;
                let mut st = Staging::default();
                st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xa5);
                set_x(&mut st.b, x);
                st.b[15] = state;
                st.b[16] = phase;
                st.b[23] = if rider { 8 } else { 0 };
                st.b[24] = span;
                st.px = 0x1234;
                st.py = 0x5678;
                let mut expected = st.b;
                set_x(&mut expected, next_x);
                expected[15] = next_state;
                expected[16] = next_phase;
                entry(&mut sim, &mut st);
                assert_eq!(st.b, expected);
                assert_eq!(sim.mario_x_adjust, rider.then_some(next_x));
                assert_eq!(sim.facing, 99);
                assert_eq!((st.px, st.py), (0x1234, 0x5678));
                assert!(sim.unported.is_empty());
            }
        }
    }
}

#[test]
fn platform_patrol_entry_animation_precedes_movement_and_invalid_state() {
    for (entry, address, toggles) in [
        (ObjectSim::handler_0x58c41 as Entry, 0x58c41, false),
        (ObjectSim::handler_0x58c52 as Entry, 0x58c52, true),
    ] {
        for state in [1, 5] {
            let mut sim = ObjectSim::new(vec![], 0);
            sim.fff0 = 4;
            let mut st = Staging::default();
            st.b[11] = 32;
            st.b[15] = state;
            st.b[24] = 36;
            let mut expected = st.b;
            if toggles {
                expected[11] ^= 1;
            }
            if state == 1 {
                set_x(&mut expected, 65535);
                expected[16] = 1;
            }
            entry(&mut sim, &mut st);
            assert_eq!(st.b, expected);
            assert_eq!(
                sim.unported,
                if state == 5 { vec![address] } else { vec![] }
            );
        }
    }
}
