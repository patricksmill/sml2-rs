//! Unit tests for native third-form Wario (anim 26, ROM `$0AD93`).

use super::form3::{update, Context, UpdateError};
use crate::actor::{
    Actor, AnimationFrameError, ContactKind, EndingBobError, Facing, FrameTick, SpriteFlipFlags,
};
use crate::objects::boss_spawn::BossSpawnRequest;
use crate::objects::boss_stun::HitStunContext;

const DUMMY_PICKER_PROFILE: [u8; 38] = [
    0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x03, 0x02, 0x03, 0x03, 0x03,
    0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x02,
    0x03, 0x03, 0x02, 0x03, 0x03, 0x00,
];

fn default_context() -> Context {
    Context {
        frame: FrameTick(0),
        spawn_flip: SpriteFlipFlags::from_bits(0),
        hit_processed: false,
        stomp_latched: false,
        stun_reload: 64,
        stun: HitStunContext {
            removal_pending: false,
            definition_sprite: 92,
            saved_sprite: 92,
            saved_state: 4,
            saved_contact_kind: ContactKind::NONE,
        },
        fade_timer: 0,
        intro_active: false,
        child_landed: false,
        level_timer_low: 0,
        player_screen_x: 80,
        player_powerup: 0,
        saved_power_state: 0xFE,
        child_sprite: 14,
    }
}

#[test]
fn state0_spawns_child25_and_enters_state12() {
    let mut actor = Actor::default();
    actor.state = 0;
    let ctx = default_context();

    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("update");
    assert_eq!(actor.state, 12);
    assert_eq!(actor.timer, 32);
    assert_eq!(
        effects.spawn_child,
        Some(BossSpawnRequest {
            animation: 25,
            sprite: 14,
            flip: SpriteFlipFlags::from_bits(0),
        })
    );
}

#[test]
fn state12_waits_for_child_latch_then_enters_state11() {
    let mut actor = Actor::default();
    actor.state = 12;
    actor.timer = 2;

    let mut ctx = default_context();
    ctx.child_landed = false;
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("waiting for child");
    assert_eq!(actor.state, 12);
    assert_eq!(actor.timer, 2, "timer does not tick without child landed");
    assert!(!effects.consume_child_latch);

    ctx.child_landed = true;
    update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("tick 1");
    assert_eq!(actor.timer, 1);
    assert_eq!(actor.state, 12);

    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("tick 2");
    assert_eq!(actor.state, 11);
    assert_eq!(actor.sprite, 116);
    assert!(effects.consume_child_latch);
}

#[test]
fn state11_intro_fade_timer_countdown_arms_fight() {
    let mut actor = Actor::default();
    actor.state = 11;

    let mut ctx = default_context();
    ctx.fade_timer = 97;
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("fade 97");
    assert_eq!(effects.sound1, Some(1));
    assert_eq!(effects.intro_active, Some(true));

    ctx.fade_timer = 0;
    ctx.intro_active = true;
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("fade 0");
    assert_eq!(effects.intro_active, Some(false));
    assert_eq!(actor.state, 3);
    assert_eq!(actor.contact_kind, ContactKind::from_raw(65));
}

#[test]
fn state3_faces_player_spawns_child27_and_sets_walk_direction() {
    let mut actor = Actor::default();
    actor.state = 3;
    actor.screen.x = 80;

    // Player to the left (50 < 80) -> faces left, state = 4, flip = 0
    let mut ctx = default_context();
    ctx.player_screen_x = 50;
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("left throw");
    assert_eq!(effects.facing, Some(Facing::LEFT));
    assert_eq!(actor.sprite_flip, SpriteFlipFlags::from_bits(0));
    assert_eq!(actor.state, 4);
    assert_eq!(actor.sprite, 92);
    assert_eq!(actor.timer, 80);
    assert_eq!(effects.sound2, Some(8));
    assert_eq!(
        effects.spawn_child,
        Some(BossSpawnRequest {
            animation: 27,
            sprite: 14,
            flip: SpriteFlipFlags::from_bits(0),
        })
    );

    // Player to the right (100 > 80) -> faces right, state = 5, flip = 32
    actor.state = 3;
    ctx.player_screen_x = 100;
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("right throw");
    assert_eq!(effects.facing, Some(Facing::RIGHT));
    assert_eq!(actor.sprite_flip, SpriteFlipFlags::from_bits(32));
    assert_eq!(actor.state, 5);
}

#[test]
fn state4_and_5_walk_and_wall_bounce() {
    let mut actor = Actor::default();
    actor.state = 4;
    actor.screen.x = 24;
    actor.timer = 0;

    let mut ctx = default_context();
    ctx.player_screen_x = 0; // dist_x = 22 < 48

    // Walking left hits left wall (x < 24)
    update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("bounce left wall");
    assert_eq!(actor.state, 5, "toggled to state 5");
    assert_eq!(
        actor.sprite_flip,
        SpriteFlipFlags::from_bits(32),
        "flipped right"
    );

    // Walking right hits right wall (x >= 152)
    actor.screen.x = 151;
    update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("bounce right wall");
    assert_eq!(actor.state, 4, "toggled back to state 4");
    assert_eq!(
        actor.sprite_flip,
        SpriteFlipFlags::from_bits(0),
        "flipped left"
    );
}

#[test]
fn proximity_under_48_px_triggers_jump_states() {
    let mut actor = Actor::default();
    actor.state = 4;
    actor.screen.x = 80;
    actor.timer = 0;

    let mut ctx = default_context();
    ctx.player_screen_x = 100; // abs_diff = 22 < 48 (after move_x(-2): 78 vs 100 is 22)

    update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("jump trigger");
    assert_eq!(actor.state, 6, "state 4 + 2 -> 6");
    assert_eq!(actor.sprite, 91);
}

#[test]
fn jump_uses_saved_power_state_and_returns_to_walk() {
    // Nonzero saved byte jump (Low profile, cap 15)
    let mut actor = Actor::default();
    actor.state = 6;
    actor.screen.x = 80;
    actor.screen.y = 80;
    actor.motion_phase = 0;
    actor.motion_leg = 0;

    let mut ctx = default_context();
    ctx.saved_power_state = 0xFE; // power 0 minus 2

    for _ in 0..15 {
        update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("rise step");
    }
    assert_eq!(actor.motion_leg, 1, "leg 0 completed, switched to leg 1");
    assert_eq!(actor.motion_phase, 0);

    for _ in 0..15 {
        update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("fall step");
    }
    assert_eq!(actor.state, 4, "jump completed -> state 4");
    assert_eq!(actor.sprite, 88);

    // Zero saved byte jump (High profile, cap 27)
    actor.state = 7;
    actor.screen.x = 80;
    actor.screen.y = 80;
    actor.motion_phase = 0;
    actor.motion_leg = 0;
    ctx.saved_power_state = 0; // power 2 minus 2; source makes no Fire claim

    for _ in 0..27 {
        update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("high rise step");
    }
    assert_eq!(actor.motion_leg, 1, "leg 0 completed at cap 27");
    assert_eq!(actor.motion_phase, 0);

    for _ in 0..27 {
        update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("high fall step");
    }
    assert_eq!(actor.state, 5, "jump completed -> state 5");
    assert_eq!(actor.sprite, 88);
}

#[test]
fn defeat_cascade_to_ending_exit() {
    let mut actor = Actor::default();
    actor.state = 2; // defeat fall
    actor.screen.y = 137;

    let ctx = default_context();
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("defeat fall step");
    assert!(effects.boss_clear_flag, "sets boss_clear_flag = true");
    assert_eq!(actor.screen.y, 137);

    // Land on ground at y >= 138
    actor.screen.y = 138;
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("defeat landing");
    assert_eq!(actor.state, 13);
    assert_eq!(actor.timer, 128);
    assert_eq!(actor.motion_phase, 2);
    assert_eq!(effects.music_request, Some(24));

    // State 13: 256 frame pause
    actor.timer = 1;
    actor.motion_phase = 1;
    update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("state 13 expiry");
    assert_eq!(actor.state, 8);
    assert_eq!(actor.timer, 255);
    assert_eq!(actor.sprite, 100);

    // State 8: crying countdown to child 28 spawn
    actor.timer = 1;
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("spawn child 28");
    assert_eq!(actor.state, 9);
    assert_eq!(actor.timer, 48);
    assert_eq!(actor.sprite, 99);
    assert_eq!(
        effects.spawn_child,
        Some(BossSpawnRequest {
            animation: 28,
            sprite: 108,
            flip: actor.sprite_flip,
        })
    );

    // State 9: pause to state 10
    actor.timer = 1;
    update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).expect("state 9 expiry");
    assert_eq!(actor.state, 10);
    assert_eq!(actor.sprite, 102);
    assert_eq!(actor.timer, 48);

    // State 10: runs offscreen at x >= 192
    actor.timer = 0;
    actor.screen.x = 191;
    let mut ctx_even = default_context();
    ctx_even.frame = FrameTick(0); // even frame
    let effects = update(&mut actor, ctx_even, &DUMMY_PICKER_PROFILE).expect("run offscreen");
    assert_eq!(actor.to_raw_slot(), [0; 32]);
    assert!(effects.done, "slot done / blank");
    assert_eq!(effects.fight_phase, Some(3), "boss fight phase = 3");
    // The actual phase event consumer clears scene state (integration test).
}

#[test]
fn source_regression_wrapped_proximity_24_to_255_enters_jump() {
    let mut actor = Actor::default();
    actor.state = 4;
    actor.screen.x = 26; // AF15 moves left two before B2DB: 24 -> 255 = -25.
    let mut ctx = default_context();
    ctx.frame = FrameTick(1);
    ctx.player_screen_x = 255;
    update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).unwrap();
    assert_eq!((actor.state, actor.screen.x, actor.sprite), (6, 24, 91));
}

#[test]
fn source_regression_malformed_bob_phase_does_not_reset_above_four() {
    for state in [4, 10] {
        let mut actor = Actor::default();
        actor.state = state;
        actor.screen.x = 80;
        actor.sprite = 102;
        actor.animation_frame = 4;
        let expected_error = if state == 4 {
            UpdateError::InvalidWalkAnimation(AnimationFrameError { frame: 5 })
        } else {
            UpdateError::InvalidEndingAnimation(EndingBobError { lookup_index: 5 })
        };
        assert_eq!(
            update(&mut actor, default_context(), &DUMMY_PICKER_PROFILE),
            Err(expected_error)
        );
        assert_eq!(
            (actor.animation_frame, actor.sprite, actor.screen.x),
            (5, 102, 80)
        );
    }
}

#[test]
fn source_regression_ending_reset_indexes_with_cleared_flip_byte() {
    let mut actor = Actor::default();
    actor.state = 10;
    actor.timer = 2;
    actor.sprite = 102;
    actor.animation_frame = 3;
    actor.sprite_flip = SpriteFlipFlags::from_bits(0x21);
    update(&mut actor, default_context(), &DUMMY_PICKER_PROFILE).unwrap();
    // B36A clears bit5, leaving A=1 for B370's lookup: delta +1, not delta -1.
    assert_eq!(
        (
            actor.animation_frame,
            actor.sprite,
            actor.sprite_flip.bits()
        ),
        (0, 103, 1)
    );
}

#[test]
fn source_regression_domain_ending_clears_every_slot_byte() {
    let mut actor = Actor::from_raw_slot(&[0xA5; 32]);
    actor.state = 10;
    actor.timer = 0;
    actor.screen.x = 191;
    let mut ctx = default_context();
    ctx.frame = FrameTick(2); // movement gate true; animation gate false.
    let effects = update(&mut actor, ctx, &DUMMY_PICKER_PROFILE).unwrap();
    assert!(effects.done);
    assert_eq!(actor.to_raw_slot(), [0; 32]);
}

#[test]
fn form3_ending_reset_propagates_flip_lookup_error_after_phase_and_flip_writes() {
    let mut actor = Actor::default();
    actor.state = 10;
    actor.timer = 2;
    actor.animation_frame = 3;
    actor.sprite_flip = SpriteFlipFlags::from_bits(0xA0);
    actor.sprite = 102;
    assert_eq!(
        update(&mut actor, default_context(), &DUMMY_PICKER_PROFILE),
        Err(UpdateError::InvalidEndingAnimation(EndingBobError {
            lookup_index: 0x80
        }))
    );
    assert_eq!(
        (
            actor.animation_frame,
            actor.sprite_flip.bits(),
            actor.sprite,
            actor.timer
        ),
        (0, 0x80, 102, 2)
    );
}
