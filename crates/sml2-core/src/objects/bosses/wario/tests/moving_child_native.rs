use super::{update_child, ChildContext, ChildEffects, ChildError, RespawnEdge};
use crate::actor::{Actor, ContactKind, Facing, FlashFlags, FrameTick, MotionProfileError};

fn context(frame: u64) -> ChildContext {
    ChildContext {
        frame: FrameTick::new(frame),
        transition_pending: true,
        respawn_edge: RespawnEdge::Left,
    }
}

#[test]
fn native_child_initialize_defers_pending_transition() {
    let mut actor = Actor::default();
    assert_eq!(
        update_child(&mut actor, context(0), &[]),
        Ok(ChildEffects::default())
    );
    assert_eq!(actor.state, 1);
    assert_eq!(actor.contact_kind, ContactKind::NONE);
}

#[test]
fn native_child_patrol_moves_before_consuming_transition() {
    for (direction, x, expected_x, expected_direction) in [
        (Facing::LEFT, 2, 255, Facing::RIGHT),
        (Facing::LEFT, 178, 175, Facing::LEFT),
        (Facing::RIGHT, 173, 176, Facing::LEFT),
        (Facing::RIGHT, 254, 1, Facing::RIGHT),
        (Facing::from_raw(0xA2), 173, 176, Facing::from_raw(0xA3)),
    ] {
        let mut actor = Actor::default();
        actor.state = 1;
        actor.screen.x = x;
        actor.travel_direction = direction;
        assert_eq!(
            update_child(&mut actor, context(4), &[]),
            Ok(ChildEffects {
                consume_transition: true
            })
        );
        assert_eq!(
            (actor.screen.x, actor.travel_direction),
            (expected_x, expected_direction)
        );
        assert_eq!(
            (actor.state, actor.contact_kind),
            (2, ContactKind::WARIO_CHILD)
        );
    }
    let mut actor = Actor::default();
    actor.state = 1;
    actor.screen.x = 2;
    let effects = update_child(&mut actor, context(5), &[]).unwrap();
    assert!(effects.consume_transition);
    assert_eq!(actor.screen.x, 2);
    actor.state = 1;
    let mut no_transition = context(8);
    no_transition.transition_pending = false;
    assert_eq!(
        update_child(&mut actor, no_transition, &[]),
        Ok(ChildEffects::default())
    );
    assert_eq!((actor.state, actor.screen.x), (1, 255));
}

#[test]
fn native_child_fall_reads_current_phase_then_caps_and_clears_all_flash() {
    let profile: Vec<u8> = (0..=37).collect();
    for (phase, frame, expected_phase, state, flash) in [
        (14, 4, 15, 2, 0xB5),
        (34, 5, 35, 2, 0xA5),
        (35, 4, 0, 3, 0),
        (36, 5, 0, 3, 0),
        (37, 4, 38, 2, 0xB5),
    ] {
        let mut actor = Actor::default();
        actor.state = 2;
        actor.motion_phase = phase;
        actor.screen.y = 250;
        actor.flash = FlashFlags::from_bits(0xA5);
        assert_eq!(
            update_child(&mut actor, context(frame), &profile),
            Ok(ChildEffects::default())
        );
        assert_eq!(actor.screen.y, 250u8.wrapping_add(phase));
        assert_eq!(
            (actor.motion_phase, actor.state, actor.flash.bits()),
            (expected_phase, state, flash)
        );
    }
}

#[test]
fn native_child_respawns_on_new_sprite_completion_at_sampled_edge() {
    for (edge, x, facing) in [
        (RespawnEdge::Left, 0, Facing::RIGHT),
        (RespawnEdge::Right, 167, Facing::LEFT),
    ] {
        let mut actor = Actor::default();
        actor.state = 3;
        actor.sprite = 98;
        actor.contact_kind = ContactKind::WARIO_CHILD;
        actor.travel_direction = Facing::from_raw(0xA3);
        let mut tick = context(8);
        tick.respawn_edge = edge;
        assert_eq!(
            update_child(&mut actor, tick, &[]),
            Ok(ChildEffects::default())
        );
        assert_eq!((actor.screen.x, actor.screen.y, actor.sprite), (x, 56, 96));
        assert_eq!(
            (actor.state, actor.contact_kind, actor.travel_direction),
            (0, ContactKind::NONE, facing)
        );
    }
}

#[test]
fn native_child_sprite_cadence_and_wrapping_do_not_respawn_early() {
    for (frame, sprite, expected) in [(7, 98, 98), (8, 96, 97), (8, 99, 100), (8, 255, 0)] {
        let mut actor = Actor::default();
        actor.state = 3;
        actor.sprite = sprite;
        actor.screen.y = 17;
        update_child(&mut actor, context(frame), &[]).unwrap();
        assert_eq!(
            (actor.sprite, actor.state, actor.screen.y),
            (expected, 3, 17)
        );
    }
}

#[test]
fn native_child_unsupported_state_is_explicit_and_preserves_actor() {
    let mut actor = Actor::default();
    actor.state = 254;
    let before = actor;
    assert_eq!(
        update_child(&mut actor, context(0), &[]),
        Err(ChildError::UnsupportedState(254))
    );
    assert_eq!(actor, before);
}

#[test]
fn native_child_invalid_profile_reports_data_instead_of_inventing_movement() {
    for (phase, len) in [(0, 0), (0, 36), (37, 37)] {
        let mut actor = Actor::default();
        actor.state = 2;
        actor.motion_phase = phase;
        actor.screen.y = 17;
        assert_eq!(
            update_child(&mut actor, context(1), &vec![3; len]),
            Err(ChildError::InvalidFallProfile(MotionProfileError {
                phase,
                cap: 36,
                profile_len: len
            }))
        );
        assert_eq!(
            (actor.state, actor.motion_phase, actor.screen.y),
            (2, phase, 17)
        );
    }
}

#[test]
fn native_child_adapter_reports_missing_profile_and_keeps_probe_scratch() {
    use crate::objects::{ObjectSim, ObjectTables, Staging};
    let mut tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    tables.boss_picker_profile.clear();
    let mut sim = ObjectSim::new(Vec::new(), 0);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.state = 2;
    actor.screen.y = 17;
    let mut staging = Staging {
        b: actor.to_raw_slot(),
        px: 0x1234,
        py: 0x5678,
    };
    sim.handler_wario21(&mut staging, &tables);
    sim.handler_wario21(&mut staging, &tables);
    assert_eq!(staging.b, actor.to_raw_slot());
    assert_eq!((staging.px, staging.py), (0x1234, 0x5678));
    assert_eq!(sim.unported, vec![0xAAA5]);
}
