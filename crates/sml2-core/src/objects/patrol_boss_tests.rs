use super::*;

fn input(frame: u64) -> Inputs {
    Inputs {
        frame: FrameTick(frame),
        div: 0,
        fade_idle: true,
        hit: None,
        primary: true,
        stun: HitStunContext {
            removal_pending: false,
            definition_sprite: 48,
            saved_sprite: 45,
            saved_state: 5,
            saved_contact_kind: ContactKind::from_raw(69),
        },
        children_busy: [false; 2],
        scene_slot_parity: 0,
    }
}

fn slot(state: u8) -> [u8; 32] {
    let mut raw = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    raw[15] = state;
    raw
}

#[test]
fn patrol_boss_init_fade_gate_and_two_distance_fields_preserve_slot() {
    for idle in [false, true] {
        let mut expected = slot(0);
        let mut actor = Actor::from_raw_slot(&expected);
        let mut inputs = input(0);
        inputs.fade_idle = idle;
        assert_eq!(update(&mut actor, inputs), Ok(Effects::default()));
        if idle {
            expected[15] = 3;
            expected[18] = 69;
            expected[24] = 32;
            expected[25] = 32;
        }
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn patrol_boss_hits_snapshot_before_changes_and_only_stomps_set_timer() {
    for stomp in [false, true] {
        let mut expected = slot(7);
        let saved = SavedHit {
            sprite: expected[11],
            state: 7,
            contact: ContactKind::from_raw(expected[18]),
        };
        let mut actor = Actor::from_raw_slot(&expected);
        let mut inputs = input(0);
        inputs.hit = Some(stomp);
        let effects = update(&mut actor, inputs).unwrap();
        assert_eq!(effects.saved_hit, Some(saved));
        expected[15] = 1;
        expected[18] = 0;
        if stomp {
            expected[11] = 48;
            expected[28] = 104;
        }
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn patrol_boss_shared_stun_recovery_and_defeat_use_native_helpers() {
    for removing in [false, true] {
        let mut expected = slot(1);
        expected[28] = 1;
        let mut actor = Actor::from_raw_slot(&expected);
        let mut inputs = input(4);
        inputs.stun.removal_pending = removing;
        update(&mut actor, inputs).unwrap();
        if removing {
            expected[14] ^= 0x10;
            expected[15] = 2;
            expected[28] = 32;
            expected[11] = 48;
        } else {
            expected[14] = 0;
            expected[15] = 5;
            expected[28] = 0;
            expected[11] = 45;
            expected[18] = 69;
        }
        assert_eq!(actor.to_raw_slot(), expected);
    }
    for primary in [false, true] {
        let mut expected = slot(2);
        expected[28] = 1;
        let mut actor = Actor::from_raw_slot(&expected);
        let mut inputs = input(4);
        inputs.primary = primary;
        let effects = update(&mut actor, inputs).unwrap();
        expected[5] = 3;
        expected[15] = 0;
        expected[26] = 0;
        expected[14] = 0;
        expected[28] = 0;
        assert_eq!(actor.to_raw_slot(), expected);
        assert_eq!(effects.silence_music, primary);
    }
}

#[test]
fn patrol_boss_initial_rise_wraps_and_unarms_only_when_distance_expires() {
    for distance in [0u8, 1, 2, 32, 255] {
        let mut expected = slot(3);
        expected[9] = 1;
        expected[24] = distance;
        expected[25] = 32;
        let mut actor = Actor::from_raw_slot(&expected);
        let effects = update(&mut actor, input(1)).unwrap();
        expected[9] = 255;
        expected[24] = distance.wrapping_sub(2);
        if distance == 2 {
            expected[12] = 0;
            expected[15] = 4;
            expected[24] = 32;
            expected[11] = 46;
        }
        assert_eq!(actor.to_raw_slot(), expected);
        assert_eq!(effects.sound1, if distance == 2 { Some(7) } else { None });
    }
}

#[test]
fn patrol_boss_rise_cadence_distance_divider_and_selected_child_busy_gates() {
    for state in [4, 6] {
        for frame in 0..8 {
            for distance in [0u8, 1, 2] {
                for parity in [0u8, 1, 2, 255] {
                    for div in [0u8, 1, 4, 255] {
                        for busy in [[false, false], [true, false], [false, true], [true, true]] {
                            let mut expected = slot(state);
                            expected[9] = 0;
                            expected[24] = distance;
                            expected[25] = 32;
                            let mut actor = Actor::from_raw_slot(&expected);
                            let mut inputs = input(frame);
                            inputs.div = div;
                            inputs.scene_slot_parity = parity;
                            inputs.children_busy = busy;
                            let effects = update(&mut actor, inputs).unwrap();
                            let selected = usize::from(parity != 0);
                            let spawn =
                                frame % 2 == 0 && distance == 1 && div & 3 != 0 && !busy[selected];
                            if frame % 2 == 0 {
                                expected[9] = 255;
                                expected[24] = distance.wrapping_sub(1);
                                if distance == 1 {
                                    expected[15] = state ^ 1;
                                    expected[24] = 32;
                                    expected[11] = if spawn { 47 } else { 44 };
                                    if spawn {
                                        expected[28] = 48;
                                    }
                                }
                            }
                            assert_eq!(actor.to_raw_slot(), expected);
                            assert_eq!(
                                effects.scene_slot_parity,
                                if spawn { Some(parity ^ 1) } else { None }
                            );
                            assert!(effects.sound1.is_none());
                            if spawn {
                                assert_eq!(
                                    effects.child,
                                    Some(ChildCommand {
                                        slot: if parity == 0 {
                                            ChildSlot::First
                                        } else {
                                            ChildSlot::Second
                                        },
                                        screen: ScreenPosition {
                                            x: expected[10],
                                            y: 255
                                        },
                                        sprite: 49
                                    })
                                );
                            } else {
                                assert!(effects.child.is_none());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn patrol_boss_patrol_thresholds_expiry_order_and_frame_gates() {
    // Literal `$9F82/$9FB1`: expiry sets sprite 44 before the cadence toggle;
    // turns consume neither vertical distance nor the movement budget.
    for (
        state,
        x,
        frame,
        timer,
        distance,
        next_state,
        next_x,
        next_y,
        next_distance,
        next_sprite,
        turn,
        sound,
    ) in [
        (5, 88, 4, 2, 32, 5, 88, 100, 32, 46, false, None),
        (5, 88, 3, 1, 32, 5, 88, 100, 32, 44, false, None),
        (5, 88, 4, 1, 32, 5, 89, 102, 30, 45, false, None),
        (5, 159, 4, 0, 32, 5, 160, 102, 30, 47, false, None),
        (5, 160, 4, 0, 32, 7, 160, 100, 32, 47, true, None),
        (7, 16, 4, 0, 32, 7, 15, 102, 30, 47, false, None),
        (7, 15, 4, 0, 32, 5, 15, 100, 32, 47, true, None),
        (5, 88, 4, 0, 2, 4, 89, 102, 32, 46, false, Some(7)),
        (7, 88, 4, 0, 2, 6, 87, 102, 32, 46, false, Some(7)),
        (5, 88, 4, 0, 0, 5, 89, 102, 254, 47, false, None),
    ] {
        let mut expected = slot(state);
        expected[9] = 100;
        expected[10] = x;
        expected[11] = 46;
        expected[24] = distance;
        expected[25] = 32;
        expected[28] = timer;
        let mut actor = Actor::from_raw_slot(&expected);
        let effects = update(&mut actor, input(frame)).unwrap();
        expected[9] = next_y;
        expected[10] = next_x;
        expected[11] = next_sprite;
        expected[15] = next_state;
        expected[24] = next_distance;
        expected[28] = timer.saturating_sub(1);
        if turn {
            expected[13] ^= 0x20;
        }
        assert_eq!(
            actor.to_raw_slot(),
            expected,
            "state={state}, x={x}, frame={frame}, timer={timer}, distance={distance}"
        );
        assert_eq!(effects.sound1, sound);
    }
}

#[test]
fn patrol_boss_invalid_state_is_reported_without_fabricated_behavior() {
    let raw = slot(8);
    let mut actor = Actor::from_raw_slot(&raw);
    assert_eq!(update(&mut actor, input(4)), Err(8));
    assert_eq!(actor.to_raw_slot(), raw);
}
