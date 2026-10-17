//! Expected byte writes independently transcribed from `$AB8B..$AD90`.
use super::form2::{update, Context};
use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags};
use crate::objects::boss_stun::HitStunContext;
const FORWARD: [u8; 25] = [
    0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3,
];
fn context() -> Context {
    Context {
        frame: FrameTick(0),
        spawn_flip: SpriteFlipFlags::from_bits(0x93),
        hit_processed: false,
        stomp_latched: false,
        stun_reload: 17,
        stun: HitStunContext {
            removal_pending: false,
            definition_sprite: 19,
            saved_sprite: 77,
            saved_state: 241,
            saved_contact_kind: ContactKind::from_raw(239),
        },
        fade_timer: 1,
        intro_active: false,
        child_landed: true,
        level_timer_low: 0,
        player_screen_x: 80,
    }
}
fn slot(state: u8) -> [u8; 32] {
    let mut raw = [0xa5; 32];
    raw[15] = state;
    raw[16] = 0;
    raw[24] = 0;
    raw[26] = 0;
    raw[28] = 1;
    raw[9] = 100;
    raw[10] = 80;
    raw[11] = 86;
    raw
}
fn run(raw: [u8; 32], c: Context) -> ([u8; 32], super::form2::Effects) {
    let mut actor = Actor::from_raw_slot(&raw);
    let e = update(&mut actor, c, &FORWARD).unwrap();
    (actor.to_raw_slot(), e)
}
#[test]
fn handler_0x0ab8b_all_twelve_states_exact_slot_bytes() {
    for state in 0..12 {
        let before = slot(state);
        let mut expected = before;
        match state {
            0 => {
                expected[15] = 7;
                expected[28] = 32;
            }
            1 => {
                expected[14] = 0;
                expected[11] = 77;
                expected[15] = 241;
                expected[18] = 239;
                expected[28] = 0;
            }
            2 => {
                expected[14] ^= 16;
                expected[16] = 1;
            }
            3 => {
                expected[16] = 1;
                expected[9] = 97;
            }
            4 => {
                expected[11] ^= 1;
                expected[10] = 79;
                expected[15] = 10;
                expected[11] = 86;
                expected[28] = 32;
            }
            5 => {
                expected[10] = 81;
                expected[15] = 10;
                expected[11] = 86;
                expected[28] = 32;
            }
            6 => {
                expected[14] = 0;
                expected[28] = 0;
                expected[26] = 0;
                expected[15] = 8;
                expected[11] = 83;
                expected[13] = 32;
            }
            7 => {
                expected[28] = 0;
                expected[15] = 11;
                expected[11] = 114;
            }
            8 => {}
            9 | 10 => expected[28] = 0,
            11 => {}
            _ => unreachable!(),
        }
        let (actual, e) = run(before, context());
        assert_eq!(actual, expected, "state {state}");
        assert_eq!(e.spawn_child.is_some(), state == 0);
        assert_eq!(e.shared_exit, state == 8);
        assert_eq!(e.consume_child_latch, state == 7);
        assert_eq!(e.fade_low, if state == 6 { Some(5) } else { None });
    }
}
#[test]
fn handler_0x0ab8b_hit_prologue_saves_before_substitution_and_stun_defeat() {
    for stomp in [false, true] {
        let mut c = context();
        c.hit_processed = true;
        c.stomp_latched = stomp;
        let before = slot(5);
        let mut expected = before;
        expected[15] = 1;
        expected[18] = 0;
        if stomp {
            expected[11] = 82;
            expected[28] = 17;
        }
        let (actual, e) = run(before, c);
        assert_eq!(actual, expected);
        assert_eq!(e.save_hit, Some((86, 5, ContactKind::from_raw(0xa5))));
    }
    for timer in [0, 1, 2, 255] {
        let mut c = context();
        c.stun.removal_pending = true;
        let mut before = slot(1);
        before[28] = timer;
        let mut expected = before;
        expected[14] ^= 16;
        if timer <= 1 {
            expected[28] = 32;
            expected[15] = 2;
            expected[11] = 19;
        } else {
            expected[28] = timer - 1;
        }
        assert_eq!(run(before, c).0, expected);
    }
}
#[test]
fn handler_0x0ab8b_terminal_wait_and_wrapping_timer_rules() {
    for state in [6, 7, 9, 10] {
        for timer in [0u8, 1, 2, 255] {
            let mut before = slot(state);
            before[28] = timer;
            let (actual, _) = run(before, context());
            let expected_timer = if matches!(state, 9 | 10) && timer == 0 {
                0
            } else {
                timer.wrapping_sub(1)
            };
            assert_eq!(actual[28], expected_timer, "state {state}, timer {timer}");
            if matches!(state, 9 | 10) && timer != 0 {
                let mut expected = before;
                expected[28] -= 1;
                assert_eq!(actual, expected);
            }
        }
    }
    let mut c = context();
    c.child_landed = false;
    assert_eq!(run(slot(7), c).0, slot(7));
}
#[test]
fn handler_0x0ab8b_full_rise_faces_left_right_and_equal_with_full_flip_assignment() {
    for (target, state, flip, facing) in [
        (79, 4, 0, 255),
        (80, 5, 32, 1),
        (81, 5, 32, 1),
        (255, 5, 32, 1),
        (0, 4, 0, 255),
    ] {
        let mut c = context();
        c.player_screen_x = target;
        let mut before = slot(3);
        before[16] = 35;
        let mut expected = before;
        expected[16] = 0;
        expected[15] = state;
        expected[13] = flip;
        let (actual, e) = run(before, c);
        assert_eq!(actual, expected);
        assert_eq!(
            e.facing.map(|f| if f.is_right() { 1 } else { 255 }),
            Some(facing)
        );
        assert_eq!(e.sound2, Some(2));
    }
}
#[test]
fn handler_0x0ab8b_profiles_match_independent_rom_distances_and_compare_after_advance() {
    let rise = [
        3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 3, 2, 3, 2, 1, 2, 2, 1,
        2, 1, 1, 1, 0, 0, 0,
    ];
    for (state, leg, start, cap) in [
        (3, 0, 0, 36),
        (9, 0, 24, 12),
        (9, 1, 0, 12),
        (2, 0, 0, 21),
        (10, 0, 0, 21),
    ] {
        for phase in 0..=cap {
            let mut before = slot(state);
            before[16] = phase;
            before[24] = leg;
            before[28] = 0;
            let mut expected = before;
            if state == 2 {
                expected[14] ^= 16;
            }
            let upward = state == 3 || (state == 9 && leg == 0);
            let step = if upward {
                rise[start + usize::from(phase)]
            } else {
                FORWARD[usize::from(phase)]
            };
            expected[9] = if upward { 100 - step } else { 100 + step };
            expected[16] = if phase == cap { cap } else { phase + 1 };
            if expected[16] == cap && state == 3 {
                expected[16] = 0;
                expected[15] = 5;
                expected[13] = 32;
            }
            if expected[16] == cap && state == 9 {
                expected[16] = 0;
                expected[26] = 0;
                expected[11] = 83;
                expected[24] ^= 1;
                if leg == 1 {
                    expected[15] = 3;
                    expected[11] = 86;
                }
            }
            assert_eq!(
                run(before, context()).0,
                expected,
                "state {state}, leg {leg}, phase {phase}"
            );
        }
    }
}
#[test]
fn handler_0x0ab8b_walk_order_bounds_and_frame_timer_gates() {
    for (state, x, next_x, next_state) in [
        (4, 32, 31, 5),
        (4, 0, 255, 10),
        (5, 143, 144, 4),
        (5, 255, 0, 10),
    ] {
        let mut c = context();
        c.player_screen_x = next_x;
        let mut before = slot(state);
        before[10] = x;
        let (actual, e) = run(before, c);
        assert_eq!((actual[10], actual[15]), (next_x, next_state));
        if matches!((state, x), (4, 32) | (5, 143)) {
            assert_eq!(actual[13], before[13] ^ 32);
            assert_eq!(e.sound2, Some(2));
        }
    }
    for frame in 0..16 {
        for timer in 0..4 {
            let mut c = context();
            c.frame = FrameTick(frame);
            c.level_timer_low = timer;
            let mut before = slot(4);
            before[10] = 113;
            c.player_screen_x = 80;
            let (actual, _) = run(before, c);
            // Source moves to112 before testing the strict <32 proximity bound.
            assert_eq!(actual[15], 4);
            assert_eq!(actual[9], 100 + u8::from(frame % 8 == 0));
            before[10] = 112;
            assert_eq!(
                run(before, c).0[15],
                if frame % 8 == 0 && timer == 0 { 10 } else { 4 }
            );
        }
    }
}
#[test]
fn handler_0x0ab8b_intro_fade_and_invalid_data_diagnostics() {
    for (fade, active, sprite, state) in [
        (97, false, 87, 11),
        (0, false, 86, 3),
        (1, false, 86, 11),
        (1, true, 87, 11),
    ] {
        let mut c = context();
        c.fade_timer = fade;
        c.intro_active = active;
        let (actual, e) = run(slot(11), c);
        assert_eq!((actual[11], actual[15]), (sprite, state));
        assert_eq!(e.sound1, if fade == 97 { Some(1) } else { None });
        if fade == 0 {
            assert_eq!(actual[18], 65);
            assert_eq!(e.intro_active, Some(false));
        }
    }
    for state in [12, 255] {
        let mut actor = Actor::from_raw_slot(&slot(state));
        assert!(update(&mut actor, context(), &FORWARD).is_err());
    }
    let mut actor = Actor::from_raw_slot(&slot(2));
    assert!(update(&mut actor, context(), &[]).is_err());
}
#[test]
fn handler_0x0ab8b_ground_thresholds_screen_wrap_and_unknown_motion_leg() {
    for state in [2, 10] {
        for (y, phase, next_y, next_state) in [
            (136, 2, 137, state),
            (137, 2, 138, if state == 2 { 6 } else { 9 }),
            (255, 2, 0, state),
        ] {
            let mut raw = slot(state);
            raw[9] = y;
            raw[16] = phase;
            raw[28] = 0;
            let (actual, effects) = run(raw, context());
            assert_eq!((actual[9], actual[15]), (next_y, next_state));
            assert_eq!(effects.camera_impact, state == 10 && next_state == 9);
            if next_state != state {
                assert_eq!(actual[28], if state == 2 { 128 } else { 48 });
            }
        }
    }
    let mut raw = slot(3);
    raw[9] = 0;
    assert_eq!(run(raw, context()).0[9], 253);
    let mut raw = slot(9);
    raw[24] = 0xa4;
    raw[16] = 11;
    raw[28] = 0;
    let (actual, _) = run(raw, context());
    assert_eq!((actual[24], actual[15], actual[11]), (0xa5, 9, 83));
    for frame in 0..16 {
        let mut c = context();
        c.frame = FrameTick(frame);
        let mut raw = slot(7);
        raw[28] = 2;
        assert_eq!(run(raw, c).0[11], if frame % 8 == 0 { 86 ^ 31 } else { 86 });
    }
}
