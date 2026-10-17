use super::*;
use crate::object::ObjectTables;

fn layout(tile: u8) -> LevelLayout {
    LevelLayout::from_bytes(&[tile; 0x3000]).unwrap()
}
fn fixture(state: u8) -> Staging {
    let mut st = Staging::default();
    st.b = [0xa5; 32];
    st.b[0..2].copy_from_slice(&0x100u16.to_be_bytes());
    st.b[3..5].copy_from_slice(&0x100u16.to_be_bytes());
    st.b[15] = state;
    st.b[26] = 0;
    st.b[28] = 0;
    st.px = 0x1234;
    st.py = 0x5678;
    st
}
fn sim() -> ObjectSim {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 19;
    sim.move_flags = 29;
    sim.div = 1;
    sim
}
fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn handler_0x58a6f_retarget_full_slots_preserve_counters_and_timer() {
    for state in [0, 1, 2] {
        for (player, face, next, flip) in [(0xff, 255, 3, 0), (0x100, 0, 3, 0), (0x101, 1, 4, 32)] {
            let mut sim = sim();
            sim.div = 64;
            sim.mario_ref_x = player;
            let mut st = fixture(state);
            st.b[26] = 255;
            st.b[28] = if state == 0 { 0xa5 } else { 0 };
            let mut expected = st.b;
            expected[11] = 68;
            expected[13] = flip;
            expected[14] = 0;
            expected[15] = next;
            expected[18] = 77;
            if state != 0 {
                expected[26] = 0;
            }
            sim.handler_0x58a6f(&mut st, &layout(0x38));
            assert_eq!(st.b, expected);
            assert_eq!(
                (sim.facing, sim.move_flags, st.px, st.py),
                (face, 29, 0x1234, 0x5678)
            );
            assert!(sim.unported.is_empty());
        }
    }
}

#[test]
fn handler_0x58a6f_stationary_strict_counter_and_cadence() {
    for state in [3, 4] {
        for (counter, next, reset) in [
            (0, 1, false),
            (38, 39, false),
            (39, 0, true),
            (40, 41, false),
            (254, 255, false),
            (255, 0, false),
        ] {
            for tick in [0, 1, 2, 255] {
                let mut sim = sim();
                sim.fff0 = tick;
                let mut st = fixture(state);
                st.b[25] = counter;
                st.b[26] = 0xa5;
                let mut expected = st.b;
                if tick % 2 == 0 {
                    expected[11] ^= 1;
                    expected[25] = next;
                    if reset {
                        expected[26] = 0;
                        expected[15] -= 2;
                        expected[28] = 36;
                    }
                }
                sim.handler_0x58a6f(&mut st, &layout(0));
                assert_eq!(st.b, expected, "state={state} count={counter} tick={tick}");
                assert_eq!(
                    (sim.facing, sim.move_flags, st.px, st.py),
                    (19, 29, 0x1234, 0x5678)
                );
            }
        }
    }
}

#[test]
fn handler_0x58a6f_timer_flash_order_and_release_animation() {
    for state in [1, 2] {
        for tick in [0, 1, 2, 255] {
            for timer in [0, 1, 2, 36] {
                let mut sim = sim();
                sim.fff0 = tick;
                let mut st = fixture(state);
                st.b[28] = timer;
                let mut expected = st.b;
                if tick % 2 == 0 {
                    expected[14] ^= 16;
                }
                if timer != 0 {
                    expected[28] -= 1;
                }
                if timer == 1 {
                    expected[11] = 64;
                    expected[18] = 76;
                }
                if timer <= 1 && tick % 2 == 0 {
                    expected[26] = 1;
                    expected[11] = expected[11].wrapping_add(1);
                    expected[0..2].copy_from_slice(
                        &(if state == 1 { 0xffu16 } else { 0x101u16 }).to_be_bytes(),
                    );
                }
                sim.handler_0x58a6f(&mut st, &layout(0x38));
                assert_eq!(st.b, expected, "state={state} tick={tick} timer={timer}");
                if timer > 1 || tick % 2 != 0 {
                    assert_eq!((st.px, st.py, sim.move_flags), (0x1234, 0x5678, 29));
                }
            }
        }
    }
    let mut sim = sim();
    sim.fff0 = 1;
    let mut st = fixture(1);
    st.b[28] = 36;
    for remaining in (0..36).rev() {
        sim.handler_0x58a6f(&mut st, &layout(0));
        assert_eq!(st.b[28], remaining);
        assert_eq!(st.b[11], if remaining == 0 { 64 } else { 0xa5 });
    }
}

#[test]
fn handler_0x58a6f_divider_mask_animation_boundaries_and_post_move_wall() {
    for state in [1, 2] {
        for divider in [0, 63, 64, 128, 192, 255] {
            for (phase, sprite, next_phase, next_sprite) in [
                (0, 255, 1, 0),
                (2, 255, 3, 0),
                (3, 0, 0, 253),
                (255, 0, 0, 253),
            ] {
                for tile in [0x37, 0x38] {
                    let mut sim = sim();
                    sim.div = divider;
                    let mut st = fixture(state);
                    st.b[26] = phase;
                    st.b[11] = sprite;
                    let mut expected = st.b;
                    expected[26] = next_phase;
                    expected[11] = next_sprite;
                    expected[14] ^= 16;
                    if divider & 63 == 0 {
                        expected[15] = 3;
                        expected[11] = 68;
                        expected[13] = 0;
                        expected[14] = 0;
                        expected[18] = 77;
                    } else {
                        expected[0..2].copy_from_slice(
                            &(if state == 1 { 0xffu16 } else { 0x101u16 }).to_be_bytes(),
                        );
                        if tile == 0x37 {
                            expected[15] = if state == 1 { 2 } else { 1 };
                            expected[13] ^= 32;
                        }
                    }
                    sim.handler_0x58a6f(&mut st, &layout(tile));
                    assert_eq!(st.b, expected);
                    if divider & 63 != 0 {
                        assert_eq!(st.px, if state == 1 { 0xf8 } else { 0x108 });
                        assert_eq!(st.py, if tile == 0x37 { 0xf5 } else { 0xfb });
                        assert_eq!(
                            sim.move_flags,
                            if state == 1 {
                                if tile == 0x37 {
                                    0x22
                                } else {
                                    0x20
                                }
                            } else if tile == 0x37 {
                                0x11
                            } else {
                                0x10
                            }
                        );
                    } else {
                        assert_eq!((st.px, st.py, sim.move_flags), (0x1234, 0x5678, 29));
                    }
                }
            }
        }
    }
}

#[test]
fn handler_0x58a6f_unsupported_state_and_animation_diagnostics() {
    for (state, phase) in [(5, 0), (255, 0), (1, 4), (2, 254)] {
        let mut sim = sim();
        let mut st = fixture(state);
        st.b[26] = phase;
        let mut expected = st.b;
        if state <= 2 {
            expected[14] ^= 16;
            expected[26] = phase.wrapping_add(1);
        }
        sim.handler_0x58a6f(&mut st, &layout(0));
        assert_eq!(st.b, expected);
        assert_eq!(sim.unported, [0x58A6F]);
        assert_eq!((st.px, st.py, sim.move_flags), (0x1234, 0x5678, 29));
    }
}

#[test]
fn handler_0x58a6f_real_anim33_dispatch_step_and_death_gate() {
    let tables = tables();
    let level = layout(0x60);
    assert_eq!(tables.profile_for(0, 33).unwrap().handler, 0x58A6F);
    for dead in [0, 255] {
        let mut sim = sim();
        sim.phase = false;
        sim.mario_dead = dead;
        let mut actor = Actor::default();
        actor.position = Position { x: 100, y: 100 };
        actor.activity = 2;
        actor.definition = 33;
        sim.slots[1].b = actor.to_raw_slot();
        let mario = MarioContact {
            x: 100,
            travelling: true,
            ..Default::default()
        };
        sim.step(100, 100, &level, &tables, &mario, false);
        assert_eq!(
            (sim.slots[1].b[15], sim.slots[1].b[11], sim.slots[1].b[18]),
            if dead == 0 { (4, 68, 77) } else { (0, 0, 0) }
        );
        assert!(sim.unported.is_empty());
    }
}
