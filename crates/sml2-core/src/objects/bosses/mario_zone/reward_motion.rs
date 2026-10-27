//! Shared boss reward arcs: `$9642/$9698`, bank 2.
use crate::actor::{Actor, FrameTick, RiseProfile};

#[derive(Clone, Copy)]
pub(super) enum Part {
    Main,
    Child,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Effects {
    pub spawn_child: bool,
    pub clear: bool,
}

pub(super) fn update(
    actor: &mut Actor,
    part: Part,
    frame: FrameTick,
    fall: &[u8],
) -> Result<Effects, ()> {
    let mut effects = Effects::default();
    if actor.state == 0 {
        match part {
            Part::Main => {
                actor.state = 1;
                effects.spawn_child = true;
            }
            Part::Child => {
                actor.timer = actor.timer.wrapping_add(1);
                if actor.timer == 8 {
                    actor.timer = 0;
                    actor.state = 1;
                    actor.screen.move_y(24);
                }
            }
        }
        return Ok(effects);
    }
    match part {
        Part::Main => {
            let previous = actor.animation_frame;
            // ROM `$B3B4`: FF 01 01 00 FF, indexed after advancing.
            actor
                .animate_sprite_delta_cycle(frame, 2, &[-1, 1, 1, 0, -1])
                .map_err(|_| ())?;
            if frame.every(2) && (previous == 2 || previous == 4) {
                actor.sprite_flip.toggle_horizontal();
            }
        }
        Part::Child => actor.animate_two_frame_toggle(frame, 2),
    }
    if !frame.every(2) {
        return Ok(effects);
    }
    match actor.state {
        1 => {
            let dy = actor.rise_step(RiseProfile::Medium).map_err(|_| ())?;
            actor.screen.move_y(-i16::from(dy));
            if actor.motion_phase == 21 {
                actor.motion_phase = 0;
                actor.state = 2;
                actor.sprite_flip.toggle_vertical();
                if matches!(part, Part::Child) {
                    actor.screen.move_y(-16);
                }
            }
        }
        2 => {
            let dy = actor.motion_step(fall, 21).map_err(|_| ())?;
            actor.screen.move_y(i16::from(dy));
            if actor.motion_phase == 21 {
                actor.motion_phase = 0;
                actor.state = 3;
                actor.sprite_flip.toggle_vertical();
                if matches!(part, Part::Child) {
                    actor.screen.move_y(16);
                }
            }
        }
        3 => {
            actor.screen.move_y(-4);
            effects.clear = actor.screen.y < 8;
        }
        _ => return Err(()),
    }
    Ok(effects)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FALL: [u8; 22] = [
        0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3, 3, 2, 3, 3, 2, 3, 3,
    ];

    #[test]
    fn reward_init_returns_without_animation_or_motion_and_preserves_flip() {
        let mut raw = [0xA5; 32];
        raw[15] = 0;
        let mut actor = Actor::from_raw_slot(&raw);
        assert_eq!(
            update(&mut actor, Part::Main, FrameTick::new(0), &FALL),
            Ok(Effects {
                spawn_child: true,
                clear: false
            })
        );
        raw[15] = 1;
        assert_eq!(actor.to_raw_slot(), raw);
    }

    #[test]
    fn reward_child_init_uses_exact_eight_not_greater_than() {
        for timer in [0u8, 6, 7, 8, 254, 255] {
            let mut raw = [0xA5; 32];
            raw[15] = 0;
            raw[28] = timer;
            let mut actor = Actor::from_raw_slot(&raw);
            update(&mut actor, Part::Child, FrameTick::new(0), &FALL).unwrap();
            raw[28] = timer.wrapping_add(1);
            if timer == 7 {
                raw[15] = 1;
                raw[28] = 0;
                raw[9] = 0xBD;
            }
            assert_eq!(actor.to_raw_slot(), raw);
        }
    }

    #[test]
    fn reward_main_animation_literal_deltas_and_both_flip_gates() {
        for (previous, next, delta, flip) in [
            (0, 1, 1, false),
            (1, 2, 1, false),
            (2, 3, 0, true),
            (3, 4, -1, false),
            (4, 0, -1, true),
            (255, 0, -1, false),
        ] {
            let mut raw = [0xA5; 32];
            raw[15] = 3;
            raw[9] = 100;
            raw[11] = 0;
            raw[26] = previous;
            let mut actor = Actor::from_raw_slot(&raw);
            update(&mut actor, Part::Main, FrameTick::new(1), &FALL).unwrap();
            assert_eq!(actor.to_raw_slot(), raw);
            update(&mut actor, Part::Main, FrameTick::new(2), &FALL).unwrap();
            raw[9] = 96;
            raw[26] = next;
            raw[11] = delta as u8;
            if flip {
                raw[13] ^= 0x20;
            }
            assert_eq!(actor.to_raw_slot(), raw);
        }
    }

    #[test]
    fn reward_arc_direction_cadence_transition_and_child_offsets() {
        for part in [Part::Main, Part::Child] {
            for (state, phase, dy, next, offset) in [
                (1, 0, -3, 1, 0),
                (1, 20, 0, 2, -16),
                (2, 0, 0, 2, 0),
                (2, 20, 3, 3, 16),
            ] {
                let mut raw = [0; 32];
                raw[15] = state;
                raw[16] = phase;
                raw[9] = 100;
                let mut actor = Actor::from_raw_slot(&raw);
                update(&mut actor, part, FrameTick::new(1), &FALL).unwrap();
                assert_eq!(actor.to_raw_slot(), raw);
                update(&mut actor, part, FrameTick::new(2), &FALL).unwrap();
                raw[9] = (100i16
                    + dy
                    + if matches!(part, Part::Child) {
                        offset
                    } else {
                        0
                    }) as u8;
                raw[15] = next;
                raw[16] = if state != next { 0 } else { phase + 1 };
                if state != next {
                    raw[13] = 0x40;
                }
                raw[11] = 1;
                if matches!(part, Part::Main) {
                    raw[26] = 1;
                }
                assert_eq!(actor.to_raw_slot(), raw);
            }
        }
    }

    #[test]
    fn reward_terminal_event_preserves_source_state_and_wrapping_y() {
        for (y, expected, clear) in [
            (0, 252, false),
            (7, 3, true),
            (8, 4, true),
            (11, 7, true),
            (12, 8, false),
        ] {
            let mut actor = Actor::default();
            actor.state = 3;
            actor.screen.y = y;
            assert_eq!(
                update(&mut actor, Part::Child, FrameTick::new(1), &FALL),
                Ok(Effects::default())
            );
            let effects = update(&mut actor, Part::Child, FrameTick::new(2), &FALL).unwrap();
            assert_eq!(
                (actor.screen.y, actor.state, effects.clear),
                (expected, 3, clear)
            );
        }
    }
}
