//! ROM `$59270` (anim 61): a rectangular patrol platform.
//!
//! State 0 arms contact 32, flash 16, faces Mario by X (`$5B3C6`) and sets
//! `+24` = 20. `$5B5B6` cycles the sprite on every non-zero state. States 1/2
//! move X every call and then, on even frames only, move Y and advance the
//! `+16` counter (`+24` frames) into state 3/5. States 3/5 rise one pixel on
//! every call and advance the counter to `2*+24`, then `state += 1` and toggle
//! the flip on an even new state. States 4/6 repeat the horizontal+down leg on
//! even frames and advance the same `2*+24` counter.
use crate::actor::{Actor, AnimationFrameError, ContactKind, FlashFlags, FrameTick};
use std::cmp::Ordering;

/// ROM `$75D3`: the three sprite deltas for `$5B5B6`.
const SPRITE_DELTAS: [i8; 3] = [-2, 1, 1];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
    Animation(AnimationFrameError),
}
impl From<AnimationFrameError> for Error {
    fn from(error: AnimationFrameError) -> Self {
        Self::Animation(error)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Initialized {
        facing: Ordering,
    },
    Moved {
        carry_dx: Option<i16>,
        carry_dy: Option<i16>,
    },
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, player_x: u16) -> Result<Effect, Error> {
    if actor.state == 0 {
        actor.contact_kind = ContactKind::from_raw(32);
        actor.flash = FlashFlags::from_bits(16);
        let relation = actor.face_toward(player_x);
        actor.motion_leg = 20;
        return Ok(Effect::Initialized { facing: relation });
    }
    actor.animate_sprite_delta_cycle(frame, 2, &SPRITE_DELTAS)?;
    match actor.state {
        // $59298: left + down; X on every call, Y and the counter on even frames.
        1 => {
            actor.position.move_x(-1);
            let carry_dx = actor.interactions.stood_on().then_some(-1);
            if !frame.every(2) {
                return Ok(Effect::Moved {
                    carry_dx,
                    carry_dy: None,
                });
            }
            actor.position.move_y(1);
            let carry_dy = actor.interactions.stood_on().then_some(1);
            if tick(actor, actor.motion_leg) {
                actor.state = 3;
            }
            Ok(Effect::Moved { carry_dx, carry_dy })
        }
        // $592BF: right + down; same cadence, into state 5.
        2 => {
            actor.position.move_x(1);
            let carry_dx = actor.interactions.stood_on().then_some(1);
            if !frame.every(2) {
                return Ok(Effect::Moved {
                    carry_dx,
                    carry_dy: None,
                });
            }
            actor.position.move_y(1);
            let carry_dy = actor.interactions.stood_on().then_some(1);
            if tick(actor, actor.motion_leg) {
                actor.state = 5;
            }
            Ok(Effect::Moved { carry_dx, carry_dy })
        }
        // $592E6: rise; on the doubled counter, advance and maybe flip.
        3 => {
            actor.position.move_y(-1);
            let carry_dy = actor.interactions.stood_on().then_some(-1);
            let cap = actor.motion_leg.wrapping_mul(2);
            if tick(actor, cap) {
                advance(actor);
            }
            Ok(Effect::Moved {
                carry_dx: None,
                carry_dy,
            })
        }
        // $530B: right + down on even frames, into state 5.
        4 => {
            actor.position.move_x(1);
            let carry_dx = actor.interactions.stood_on().then_some(1);
            if !frame.every(2) {
                return Ok(Effect::Moved {
                    carry_dx,
                    carry_dy: None,
                });
            }
            actor.position.move_y(1);
            let carry_dy = actor.interactions.stood_on().then_some(1);
            let cap = actor.motion_leg.wrapping_mul(2);
            if tick(actor, cap) {
                advance(actor);
            }
            Ok(Effect::Moved { carry_dx, carry_dy })
        }
        // $592E6 again (state 5): rise and advance into state 6.
        5 => {
            actor.position.move_y(-1);
            let carry_dy = actor.interactions.stood_on().then_some(-1);
            let cap = actor.motion_leg.wrapping_mul(2);
            if tick(actor, cap) {
                advance(actor);
            }
            Ok(Effect::Moved {
                carry_dx: None,
                carry_dy,
            })
        }
        // $59322: left + down on even frames, back into state 3.
        6 => {
            actor.position.move_x(-1);
            let carry_dx = actor.interactions.stood_on().then_some(-1);
            if !frame.every(2) {
                return Ok(Effect::Moved {
                    carry_dx,
                    carry_dy: None,
                });
            }
            actor.position.move_y(1);
            let carry_dy = actor.interactions.stood_on().then_some(1);
            let cap = actor.motion_leg.wrapping_mul(2);
            if tick(actor, cap) {
                actor.state = 3;
            }
            Ok(Effect::Moved { carry_dx, carry_dy })
        }
        state => Err(Error::InvalidState(state)),
    }
}

/// Advance `+16`; reset it and report the counter's cap when it matches.
fn tick(actor: &mut Actor, cap: u8) -> bool {
    actor.motion_phase = actor.motion_phase.wrapping_add(1);
    let done = actor.motion_phase == cap;
    if done {
        actor.motion_phase = 0;
    }
    done
}

/// `$592FC`: `state += 1`, then toggle the flip only when the new state is even.
fn advance(actor: &mut Actor) {
    actor.state = actor.state.wrapping_add(1);
    if actor.state & 1 == 0 {
        actor.sprite_flip.toggle_horizontal();
    }
}
