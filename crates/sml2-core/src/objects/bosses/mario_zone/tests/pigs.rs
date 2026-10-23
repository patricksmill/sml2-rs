//! Native Mario Zone boss behavior the whole-slot integration tests leave open.
//! Constants are read from ROM bytes: `$B402` tests `$FFF0 & $0F`; Pig 2
//! (`$9D4F`) and the controller (`$9E07`) count `$FFEC` up to `$30`; only Pig 1
//! (`$9C84`) calls the fade test `$B14E`.
use super::pigs::{update_controller, update_part, Context, Part};
use crate::actor::{Actor, ContactKind, FrameTick};
use crate::objects::boss_stun::HitStunContext;

fn context(frame: u64, fade_idle: bool, definitions_left: u8) -> Context {
    Context {
        frame: FrameTick(frame),
        hit_processed: false,
        stomp_latched: false,
        stun_reload: 0,
        stun: HitStunContext {
            removal_pending: false,
            definition_sprite: 0,
            saved_sprite: 0,
            saved_state: 0,
            saved_contact_kind: ContactKind::NONE,
        },
        fade_idle,
        definitions_left,
        hits_remaining: 8,
    }
}

const PROFILE: [u8; 38] = [0; 38];

#[test]
fn intro_blink_flips_only_on_every_sixteenth_frame() {
    for frame in 0..64u64 {
        let mut actor = Actor::default();
        update_part(&mut actor, Part::Straw, context(frame, false, 3), &PROFILE).unwrap();
        assert_eq!(
            actor.sprite_flip.horizontal(),
            frame % 16 == 0,
            "frame {frame}"
        );
    }
}

#[test]
fn pig2_intro_counts_to_0x30_without_waiting_for_the_fade() {
    let mut actor = Actor::default();
    for tick in 1..0x30u8 {
        let effects = update_part(&mut actor, Part::Wood, context(1, false, 1), &PROFILE).unwrap();
        assert!(effects.release_remove_mode);
        assert_eq!((actor.state, actor.timer), (0, tick));
    }
    let effects = update_part(&mut actor, Part::Wood, context(1, false, 1), &PROFILE).unwrap();
    assert_eq!(
        (actor.state, actor.sprite, effects.sound1),
        (3, 0x20, Some(4))
    );
    assert_eq!(actor.contact_kind, ContactKind::from_raw(0x45));
}

#[test]
fn pig1_intro_waits_for_the_fade() {
    let mut actor = Actor::default();
    update_part(&mut actor, Part::Straw, context(1, false, 3), &PROFILE).unwrap();
    assert_eq!(actor.state, 0);
    update_part(&mut actor, Part::Straw, context(1, true, 3), &PROFILE).unwrap();
    assert_eq!(actor.state, 3);
}

#[test]
fn controller_intro_spawns_its_house_after_0x30_frames() {
    let mut actor = Actor::default();
    for _ in 1..0x30 {
        let effects =
            update_controller(&mut actor, context(1, false, 0), &PROFILE, &PROFILE).unwrap();
        assert_eq!(effects.spawn_child, None);
    }
    let effects = update_controller(&mut actor, context(1, false, 0), &PROFILE, &PROFILE).unwrap();
    assert_eq!(effects.spawn_child, Some((0x0D, 0x0E)));
    assert_eq!(actor.state, 3);
}
