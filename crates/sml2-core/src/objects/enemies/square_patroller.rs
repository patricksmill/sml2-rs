//! ROM `$5B0D5` (mode B anim 90): a square patrol. State 0 arms contact `$4D`,
//! faces Mario and latches that side in `motion_phase`; states 1/2 walk
//! left/right and 3/4 move down/up one pixel per tick. Each side lasts
//! `travel_span()` ticks, reloaded from `motion_leg`; a latched side of 1
//! loops `1 -> 3 -> 2 -> 4`, anything else `1 -> 4 -> 2 -> 3`.
use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags};
use std::cmp::Ordering;

const CONTACT: u8 = 0x4D;
/// The first side is half length.
const FIRST_SIDE: u8 = 0x18;
const SIDE: u8 = 0x30;
const SPRITE_WALK: u8 = 0x74;
const SPRITE_DOWN: u8 = 0x78;
const SPRITE_UP: u8 = 0x76;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidState(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Faced(Ordering),
    Continue,
}

pub(super) fn update(actor: &mut Actor, frame: FrameTick, mario_x: u16) -> Result<Effect, Error> {
    // `$5B521`: the mask-1 sprite toggle on even frames.
    actor.animate_two_frame_toggle(frame, 2);
    let clockwise = actor.motion_phase == 1;
    match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(CONTACT);
            actor.set_travel_span(FIRST_SIDE);
            actor.motion_leg = SIDE;
            let side = actor.face_toward(mario_x);
            actor.motion_phase = actor.state;
            return Ok(Effect::Faced(side));
        }
        1 => {
            actor.position.move_x(-1);
            if side_finished(actor) {
                if clockwise {
                    go_down(actor)
                } else {
                    go_up(actor)
                }
            }
        }
        2 => {
            actor.position.move_x(1);
            if side_finished(actor) {
                actor.sprite_flip = SpriteFlipFlags::from_bits(0);
                if clockwise {
                    go_up(actor)
                } else {
                    go_down(actor)
                }
            }
        }
        3 => {
            actor.position.move_y(1);
            if side_finished(actor) {
                actor.sprite = SPRITE_WALK;
                if clockwise {
                    walk_right(actor)
                } else {
                    actor.state = 1
                }
            }
        }
        4 => {
            actor.position.move_y(-1);
            if side_finished(actor) {
                actor.sprite = SPRITE_WALK;
                if clockwise {
                    actor.state = 1
                } else {
                    walk_right(actor)
                }
            }
        }
        state => return Err(Error::InvalidState(state)),
    }
    Ok(Effect::Continue)
}

/// `$5B0FD`: count the side down; on expiry reload it and report the turn.
fn side_finished(actor: &mut Actor) -> bool {
    let left = actor.travel_span().wrapping_sub(1);
    if left != 0 {
        actor.set_travel_span(left);
        return false;
    }
    actor.set_travel_span(actor.motion_leg);
    true
}

/// `$7115`.
fn go_down(actor: &mut Actor) {
    actor.state = 3;
    actor.sprite = SPRITE_DOWN;
}

/// `$710C`.
fn go_up(actor: &mut Actor) {
    actor.state = 4;
    actor.sprite = SPRITE_UP;
}

/// `$7154`.
fn walk_right(actor: &mut Actor) {
    actor.sprite_flip = SpriteFlipFlags::from_bits(0x20);
    actor.state = 2;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_latched_side_of_one_patrols_left_down_right_up() {
        let mut actor = Actor::default();
        actor.position.x = 0x80;
        assert_eq!(
            update(&mut actor, FrameTick::new(1), 0x10),
            Ok(Effect::Faced(Ordering::Less))
        );
        let mut order = vec![actor.state];
        for _ in 0..(0x18 + 3 * 0x30) {
            update(&mut actor, FrameTick::new(1), 0x10).unwrap();
            if order.last() != Some(&actor.state) {
                order.push(actor.state);
            }
        }
        assert_eq!(order, [1, 3, 2, 4, 1]);
        assert_eq!(actor.position.x, 0x80 - 0x18 + 0x30);
    }
}
