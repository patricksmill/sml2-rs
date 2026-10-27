use super::*;
use crate::actor::Position;

fn tick(actor: &mut Actor, frame: u64) -> Result<BobberEffect, BobberError> {
    update(
        actor,
        BobberContext {
            frame: FrameTick::new(frame),
            player_ref_y: 0,
        },
    )
}

#[test]
fn initialization_compares_full_world_y_and_preserves_other_actor_fields() {
    for (object_y, player_y, relation, state) in [
        (0x100, 0xFF, Ordering::Less, 1),
        (0xFF, 0x100, Ordering::Greater, 2),
        (0xFFFF, 0xFFFF, Ordering::Equal, 1),
        (0, 0xFFFF, Ordering::Greater, 2),
        (0xFFFF, 0, Ordering::Less, 1),
    ] {
        let mut actor = Actor::from_raw_slot(&[0xA5; 32]);
        actor.state = 0;
        actor.position.y = object_y;
        let mut expected = actor;
        expected.state = state;
        expected.contact_kind = ContactKind::from_raw(86);
        expected.flash = FlashFlags::from_bits(16);
        assert_eq!(
            update(
                &mut actor,
                BobberContext {
                    frame: FrameTick::new(255),
                    player_ref_y: player_y,
                }
            ),
            Ok(BobberEffect::PlayerRelation(relation))
        );
        assert_eq!(actor, expected);
    }
}

#[test]
fn complete_source_profiles_and_transition_order() {
    // Literal samples from ROM $5B4A4 and $5B488, independent of production tables.
    let bounce = [3, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0, 0];
    let fall = [
        0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3,
    ];
    for (state, samples, direction, next) in [
        (1, &bounce[..], -1i16, 3),
        (2, &bounce[..], 1, 4),
        (3, &fall[..], 1, 2),
        (4, &fall[..], -1, 1),
    ] {
        for (phase, distance) in samples.iter().copied().enumerate() {
            let mut actor = Actor::default();
            actor.state = state;
            actor.motion_phase = phase as u8;
            actor.position = Position {
                x: 0xCAFE,
                y: 0x100,
            };
            tick(&mut actor, 0).unwrap();
            assert_eq!(
                actor.position,
                Position {
                    x: 0xCAFE,
                    y: 0x100u16.wrapping_add_signed(direction * distance),
                },
                "state {state}, phase {phase}"
            );
            let transitioned = matches!(phase, 14 | 15);
            assert_eq!(
                actor.motion_phase,
                if transitioned { 0 } else { phase as u8 + 1 }
            );
            assert_eq!(actor.state, if transitioned { next } else { state });
        }
        // Consecutive updates use phases 0..14, then change state after movement.
        let mut actor = Actor::default();
        actor.state = state;
        actor.position.y = 100;
        let mut y = 100u16;
        for (phase, distance) in samples[..15].iter().copied().enumerate() {
            tick(&mut actor, phase as u64 * 4).unwrap();
            y = y.wrapping_add_signed(direction * distance);
            assert_eq!(actor.position.y, y);
            assert_eq!(actor.state, if phase == 14 { next } else { state });
        }
    }
}

#[test]
fn cadence_timer_and_native_position_wrap() {
    for state in [3, 4] {
        let mut actor = Actor::default();
        actor.state = state;
        actor.timer = 255;
        actor.sprite = 0xFE;
        actor.motion_phase = 2;
        for frame in [253, 254, 255] {
            tick(&mut actor, frame).unwrap();
            assert_eq!(
                (actor.timer, actor.motion_phase, actor.position.y),
                (255, 2, 0)
            );
        }
        assert_eq!(actor.sprite, 0xFF);
        tick(&mut actor, 256).unwrap();
        assert_eq!(
            (actor.timer, actor.motion_phase, actor.sprite),
            (254, 2, 0xFE)
        );
        actor.timer = 1;
        tick(&mut actor, 260).unwrap();
        assert_eq!(
            (actor.timer, actor.motion_phase, actor.position.y),
            (0, 2, 0)
        );
        tick(&mut actor, 264).unwrap();
        assert_eq!(actor.position.y, if state == 3 { 1 } else { 0xFFFF });
    }
    for (state, y, expected) in [(1, 1, 0xFFFE), (2, 0xFFFE, 1)] {
        let mut actor = Actor::default();
        actor.state = state;
        actor.position.y = y;
        tick(&mut actor, 0).unwrap();
        assert_eq!((actor.position.y, actor.motion_phase), (expected, 1));
    }
}

#[test]
fn unknown_state_and_missing_profile_preserve_animation_prefix() {
    for state in [5, 99, 255] {
        let mut actor = Actor::default();
        actor.state = state;
        actor.sprite = 0xFE;
        assert_eq!(tick(&mut actor, 2), Ok(BobberEffect::None));
        assert_eq!(actor.sprite, 0xFF);
        assert_eq!(tick(&mut actor, 4), Err(BobberError::UnknownState(state)));
        assert_eq!(actor.sprite, 0xFE);
    }
    for (state, phase, len) in [(1, 16, 16), (2, 255, 16), (3, 22, 22), (4, 255, 22)] {
        let mut actor = Actor::default();
        actor.state = state;
        actor.motion_phase = phase;
        actor.sprite = 0xFE;
        let mut expected = actor;
        expected.sprite ^= 1;
        assert_eq!(
            tick(&mut actor, 4),
            Err(BobberError::MotionProfile(MotionProfileError {
                phase,
                cap: 15,
                profile_len: len,
            }))
        );
        assert_eq!(actor, expected);
    }
}
