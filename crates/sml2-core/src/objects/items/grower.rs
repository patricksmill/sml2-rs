//! Native grower, anim 92 (`$5B1C8`): it bobs three 3-pixel segments while
//! armed, then grows its sprite from 122 to 124, holds, and shrinks back to
//! start over. Slot mapping lives in the `items` adapter.
use crate::actor::{Actor, ContactKind, Facing, FrameTick};

/// Contact kind while bobbing.
const ARMED_CONTACT: u8 = 0x4C;
/// Pixels per bob segment, also the segment count.
const SEGMENT: u8 = 3;
const SMALL_SPRITE: u8 = 0x7A;
const GROWN_SPRITE: u8 = 0x7C;
/// Updates the grown sprite holds before shrinking.
const GROWN_HOLD: u8 = 0x20;

pub(super) fn update(actor: &mut Actor, frame: FrameTick) {
    match actor.state {
        1 => bob(actor),
        // `$7213`: grow one sprite step every fourth frame.
        2 => {
            if !frame.every(4) {
                return;
            }
            actor.sprite = actor.sprite.wrapping_add(1);
            if actor.sprite == GROWN_SPRITE {
                actor.timer = GROWN_HOLD;
                actor.state = 3;
            }
        }
        // `$7229`: hold, then shrink back and start over.
        3 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                return;
            }
            if !frame.every(4) {
                return;
            }
            actor.sprite = actor.sprite.wrapping_sub(1);
            if actor.sprite == SMALL_SPRITE {
                arm(actor);
            }
        }
        _ => arm(actor),
    }
}

/// `$71D3`
fn arm(actor: &mut Actor) {
    actor.contact_kind = ContactKind::from_raw(ARMED_CONTACT);
    actor.state = 1;
    actor.motion_leg = SEGMENT;
    actor.travel_direction = Facing::from_raw(SEGMENT);
    actor.animation_frame = SEGMENT;
}

/// `$71E4`: one pixel up (phase 0) or down; `motion_leg` counts the segment's
/// pixels, reloaded from `travel_direction`; `animation_frame` the segments.
fn bob(actor: &mut Actor) {
    actor
        .position
        .move_y(if actor.motion_phase == 0 { -1 } else { 1 });
    actor.motion_leg = actor.motion_leg.wrapping_sub(1);
    if actor.motion_leg != 0 {
        return;
    }
    actor.animation_frame = actor.animation_frame.wrapping_sub(1);
    if actor.animation_frame == 0 {
        // `$720B`: the last segment; harmless from here on.
        actor.contact_kind = ContactKind::NONE;
        actor.state = 2;
        return;
    }
    actor.motion_leg = actor.travel_direction.raw();
    actor.motion_phase ^= 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grower_bobs_three_segments_then_grows_holds_and_rearms() {
        let mut actor = Actor::default();
        actor.position.y = 0x80;
        update(&mut actor, FrameTick(0));
        assert_eq!((actor.state, actor.contact_kind.raw()), (1, 0x4C));
        for _ in 0..9 {
            update(&mut actor, FrameTick(0));
        }
        // Up 3, down 3, up 3.
        assert_eq!(
            (actor.position.y, actor.state, actor.contact_kind),
            (0x80 - 3, 2, ContactKind::NONE)
        );
        actor.sprite = SMALL_SPRITE;
        update(&mut actor, FrameTick(1));
        assert_eq!(actor.sprite, SMALL_SPRITE);
        update(&mut actor, FrameTick(0));
        update(&mut actor, FrameTick(4));
        assert_eq!(
            (actor.sprite, actor.state, actor.timer),
            (GROWN_SPRITE, 3, 0x20)
        );
        for _ in 0..0x20 {
            update(&mut actor, FrameTick(0));
        }
        assert_eq!(actor.sprite, GROWN_SPRITE);
        update(&mut actor, FrameTick(0));
        update(&mut actor, FrameTick(0));
        assert_eq!((actor.sprite, actor.state), (SMALL_SPRITE, 1));
    }
}
