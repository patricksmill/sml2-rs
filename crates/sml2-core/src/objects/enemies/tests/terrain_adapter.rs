use super::*;

type Entry = fn(&mut ObjectSim, &mut Staging, &LevelLayout);

#[derive(Clone, Copy)]
enum Family {
    Landing,
    Projectile,
    Striking,
    Fallwalker,
}

fn entries() -> [(Entry, Family); 6] {
    [
        (ObjectSim::handler_0x59431, Family::Landing),
        (ObjectSim::handler_0x5945a, Family::Landing),
        (ObjectSim::handler_0x5a0aa, Family::Projectile),
        (ObjectSim::handler_0x5a179, Family::Striking),
        (ObjectSim::handler_fallwalker, Family::Fallwalker),
        (ObjectSim::handler_fallwalker52, Family::Fallwalker),
    ]
}

fn fixture(state: u8, x: u16, y: u16) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    sim.move_flags = 0x97;
    sim.fff0 = 3;
    sim.mario_ref_x = 200;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xa5);
    set_x(&mut st.b, x);
    set_y(&mut st.b, y);
    st.b[13] = 0;
    st.b[15] = state;
    st.b[16] = 5;
    st.b[25] = 10;
    st.b[26] = 0;
    st.b[28] = 2;
    st.px = 0x1234;
    st.py = 0x5678;
    (sim, st)
}

fn expected_patrol_tail(expected: &mut [u8; 32], family: Family, even: bool) {
    if even && !matches!(family, Family::Landing) {
        expected[11] = expected[11].wrapping_add(1);
        expected[26] = 1;
    }
    if matches!(family, Family::Projectile) {
        expected[25] = 9;
    }
    if matches!(family, Family::Striking) {
        expected[28] = 1;
    }
}

#[test]
fn terrain_adapter_wall_retry_flags_and_post_move_coordinates() {
    for (entry, family) in entries() {
        // $582FB/$5831A move first; $84EA/$843C probe Y-11, then Y-5.
        for (state, x, next_x, probe_x) in [
            (1, 23, 22, 15),
            (2, 8, 9, 16),
            (1, 0, 65535, 65528),
            (2, 65535, 0, 7),
        ] {
            for (first, retry, blocked, probe_y) in
                [(55, 56, true, 13), (56, 55, true, 19), (56, 56, false, 19)]
            {
                let (mut sim, mut st) = fixture(state, x, 24);
                sim.fff0 = 2;
                let mut layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
                let col = usize::from((probe_x >> 4) & 255u16);
                layout.cells[col] = first;
                layout.cells[256 + col] = retry;
                let mut expected = st.b;
                set_x(&mut expected, next_x);
                if blocked {
                    expected[15] = state ^ 3;
                    expected[13] ^= 0x20;
                }
                expected_patrol_tail(&mut expected, family, true);
                entry(&mut sim, &mut st, &layout);
                assert_eq!(st.b, expected);
                assert_eq!((st.px, st.py), (probe_x, probe_y));
                let flags = match (state, blocked) {
                    (1, true) => 0x22,
                    (1, false) => 0x20,
                    (_, true) => 0x11,
                    (_, false) => 0x10,
                };
                assert_eq!(sim.move_flags, flags);
                assert_eq!(sim.facing, 99);
                assert!(sim.unported.is_empty());
            }
        }
    }
}

#[test]
fn terrain_adapter_ground_alignment_preserves_stale_x_and_full_actor_y() {
    for (entry, family) in entries() {
        for state in [1, 2] {
            for (y, tile, ground, probe_x) in [
                (96, 71, true, 32),
                (96, 72, false, 32),
                (0x104, 0, false, 0x1234),
            ] {
                let (mut sim, mut st) = fixture(state, 32, y);
                let layout = LevelLayout::from_bytes(&[tile; 0x3000]).unwrap();
                let mut expected = st.b;
                if !ground {
                    expected[15] ^= 3;
                    expected[13] ^= 0x20;
                }
                expected_patrol_tail(&mut expected, family, false);
                entry(&mut sim, &mut st, &layout);
                assert_eq!(st.b, expected);
                assert_eq!((st.px, st.py), (probe_x, y));
                assert_eq!(sim.move_flags, if ground { 0x88 } else { 0x80 });
                assert_eq!(sim.facing, 99);
                assert!(sim.unported.is_empty());
            }
        }
    }
}

#[test]
fn terrain_adapter_fall_and_landing_keep_entry_specific_outputs() {
    for (entry, contact, flash, arms_in_air) in [
        (ObjectSim::handler_0x59431 as Entry, 77, false, false),
        (ObjectSim::handler_0x5945a, 72, true, false),
        (ObjectSim::handler_fallwalker, 0x56, false, true),
        (ObjectSim::handler_fallwalker52, 0x52, false, true),
    ] {
        for (y, tile, ground, probe_x) in [
            (96, 71, true, 32),
            (96, 72, false, 32),
            (0x104, 0, false, 0x1234),
        ] {
            let (mut sim, mut st) = fixture(0, 32, y);
            let layout = LevelLayout::from_bytes(&[tile; 0x3000]).unwrap();
            let mut expected = st.b;
            if flash {
                expected[14] = 16;
            }
            if ground || arms_in_air {
                expected[18] = contact;
            }
            if ground {
                expected[15] = 2;
                expected[13] = 0x20;
                expected[16] = 0;
            } else {
                // $5B451: phase 5 yields two pixels, then phase 6.
                set_y(&mut expected, y + 2);
                expected[16] = 6;
            }
            entry(&mut sim, &mut st, &layout);
            assert_eq!(st.b, expected);
            assert_eq!((st.px, st.py), (probe_x, y));
            assert_eq!(sim.move_flags, if ground { 0x88 } else { 0x80 });
            assert_eq!(sim.facing, if ground { 1 } else { 99 });
            assert!(sim.unported.is_empty());
        }
    }
}

#[test]
fn terrain_adapter_no_probe_paths_leave_scratch_and_flags_untouched() {
    let layout = LevelLayout::from_bytes(&[0; 0x3000]).unwrap();
    for (entry, projectile) in [
        (ObjectSim::handler_0x5a0aa as Entry, true),
        (ObjectSim::handler_0x5a179, false),
    ] {
        for state in [0, 3] {
            let (mut sim, mut st) = fixture(state, 32, 96);
            let mut expected = st.b;
            if state == 0 {
                expected[15] = 2;
                expected[13] = 0x20;
                expected[18] = 77;
                if projectile {
                    expected[25] = 48;
                }
            } else if projectile {
                expected[25] = 9;
                expected[28] = 1;
            }
            entry(&mut sim, &mut st, &layout);
            assert_eq!(st.b, expected);
            assert_eq!((st.px, st.py, sim.move_flags), (0x1234, 0x5678, 0x97));
            assert_eq!(sim.facing, if state == 0 { 1 } else { 99 });
            assert!(sim.unported.is_empty());
        }
    }
    let (mut sim, mut st) = fixture(1, 32, 96);
    st.b[25] = 1;
    let mut expected = st.b;
    expected[11] = 66;
    expected[15] = 3;
    expected[25] = 0;
    expected[28] = 16;
    sim.handler_0x5a0aa(&mut st, &layout);
    assert_eq!(st.b, expected);
    assert_eq!((st.px, st.py, sim.move_flags), (0x1234, 0x5678, 0x97));
    assert_eq!(sim.sfx1, 17);
    assert_eq!(sim.standby, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn terrain_adapter_patrol_contract_needs_no_fall_operation() {
    use crate::actor::{Actor, FrameTick};
    struct PatrolOnly;
    impl fallwalker::PatrolWorld for PatrolOnly {
        fn probe_left_wall(&mut self, actor: &Actor) -> bool {
            assert_eq!(actor.position.x, 65535);
            true
        }
        fn probe_right_wall(&mut self, actor: &Actor) -> bool {
            assert_eq!(actor.position.x, 0);
            true
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            false
        }
    }
    let mut actor = Actor::default();
    actor.state = 1;
    let mut world = PatrolOnly;
    fallwalker::patrol(&mut actor, FrameTick::new(2), &mut world);
    assert_eq!(actor.state, 2);
    fallwalker::patrol(&mut actor, FrameTick::new(4), &mut world);
    assert_eq!(actor.state, 1);
    fallwalker::patrol(&mut actor, FrameTick::new(5), &mut world);
    assert_eq!(actor.state, 2);
    assert_eq!(actor.position.x, 0);
}
