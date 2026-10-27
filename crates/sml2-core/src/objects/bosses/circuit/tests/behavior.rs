use super::*;
fn input() -> Inputs {
    Inputs {
        frame: FrameTick::new(0),
        div: 0,
        fade_idle: true,
        hit: None,
        primary: true,
        stun: HitStunContext {
            removal_pending: false,
            definition_sprite: 75,
            saved_sprite: 66,
            saved_state: 17,
            saved_contact_kind: ContactKind::from_raw(69),
        },
    }
}
fn raw(state: u8) -> [u8; 32] {
    let mut b = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(19));
    b[15] = state;
    b[26] = 0;
    b
}

#[test]
fn handler_0xa372_hit_prologue_and_fade_gate_preserve_whole_slot() {
    for state in [0, 3, 17, 31, 255] {
        for stomp in [false, true] {
            let mut expected = raw(state);
            let mut actor = Actor::from_raw_slot(&expected);
            let effects = update(
                &mut actor,
                Inputs {
                    hit: Some(stomp),
                    ..input()
                },
            )
            .unwrap();
            assert_eq!(
                effects.saved_hit,
                Some((expected[11], state, ContactKind::from_raw(expected[18])))
            );
            expected[15] = 1;
            expected[18] = 0;
            if stomp {
                expected[11] = 74;
                expected[28] = 48;
            }
            assert_eq!(actor.to_raw_slot(), expected);
            assert_eq!((effects.sound1, effects.sound2), (None, None));
        }
    }
    for fade_idle in [false, true] {
        let mut expected = raw(0);
        let mut actor = Actor::from_raw_slot(&expected);
        update(
            &mut actor,
            Inputs {
                fade_idle,
                ..input()
            },
        )
        .unwrap();
        if fade_idle {
            expected[15] = 3;
            expected[18] = 69;
            expected[25] = 36;
            expected[14] = 16;
        }
        assert_eq!(actor.to_raw_slot(), expected);
    }
}

#[test]
fn handler_0xa372_live_animation_and_sounds_use_old_defeat_byte_all_cadences() {
    for frame in 0..=255 {
        for div in 0..=255 {
            for defeat in [0, 1, 128, 255] {
                let mut expected = raw(17);
                expected[25] = 8;
                expected[12] = defeat;
                let mut actor = Actor::from_raw_slot(&expected);
                let effects = update(
                    &mut actor,
                    Inputs {
                        frame: FrameTick::new(frame),
                        div,
                        ..input()
                    },
                )
                .unwrap();
                expected[14] = 16;
                if frame % 4 == 0 {
                    expected[26] = 1;
                    expected[11] = expected[11].wrapping_add(1);
                }
                expected[10] = expected[10].wrapping_add(2);
                expected[25] = 6;
                assert_eq!(actor.to_raw_slot(), expected);
                let audible = defeat == 0 && frame % 2 == 0;
                assert_eq!(effects.sound2, if audible { Some(7) } else { None });
                assert_eq!(
                    effects.sound1,
                    if audible && div & 63 == 0 {
                        Some(3)
                    } else {
                        None
                    }
                );
            }
        }
    }
}

#[test]
fn handler_0xa372_shared_stun_and_death_force_flash_after_terminal_morph() {
    for remove in [false, true] {
        let mut expected = raw(1);
        expected[28] = 1;
        let mut actor = Actor::from_raw_slot(&expected);
        let mut inp = input();
        inp.stun.removal_pending = remove;
        update(&mut actor, inp).unwrap();
        expected[28] = if remove { 32 } else { 0 };
        expected[14] = if remove { expected[14] ^ 16 } else { 0 };
        expected[11] = if remove { 75 } else { 66 };
        expected[15] = if remove { 2 } else { 17 };
        if !remove {
            expected[18] = 69;
        }
        assert_eq!(actor.to_raw_slot(), expected);
    }
    for frame in 0..4 {
        for primary in [false, true] {
            for timer in [0u8, 1, 2, 255] {
                let mut expected = raw(2);
                expected[28] = timer;
                let mut actor = Actor::from_raw_slot(&expected);
                let effects = update(
                    &mut actor,
                    Inputs {
                        frame: FrameTick::new(frame),
                        primary,
                        ..input()
                    },
                )
                .unwrap();
                if frame == 0 {
                    expected[11] ^= 1;
                }
                expected[28] = timer.wrapping_sub(1);
                expected[14] = 16;
                if timer == 1 {
                    expected[5] = 3;
                    expected[15] = 0;
                    expected[26] = 0;
                }
                assert_eq!(actor.to_raw_slot(), expected);
                assert_eq!(effects.silence_music, timer == 1 && primary);
            }
        }
    }
}

#[test]
fn handler_0xa372_invalid_animation_and_state_preserve_source_prefix() {
    let mut expected = raw(3);
    expected[26] = 4;
    let mut actor = Actor::from_raw_slot(&expected);
    assert!(update(&mut actor, input()).is_err());
    expected[14] = 16;
    expected[26] = 5;
    assert_eq!(actor.to_raw_slot(), expected);
    let mut expected = raw(255);
    let mut actor = Actor::from_raw_slot(&expected);
    assert!(update(
        &mut actor,
        Inputs {
            frame: FrameTick::new(1),
            ..input()
        }
    )
    .is_err());
    expected[14] = 16;
    assert_eq!(actor.to_raw_slot(), expected);
}
