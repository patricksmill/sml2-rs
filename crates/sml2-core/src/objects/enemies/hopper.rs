//! ROM `$58712`: the hopper. Parked inside its block as a `$80` item pose,
//! it rises eight pixels once released, then walks the `$5821E` legs from
//! states 2..=5 (one above the walker's numbering).
use super::walker::{step_walk, WalkLeg, WalkerWorld};
use crate::actor::{Actor, ContactKind, ObjectRevision};

/// The parked item pose, which the draw and the contact pass skip.
const PARKED_CONTACT: u8 = 0x80;
/// Pixels it rises out of the block.
const RISE: u8 = 8;

#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    /// State 0 parked the hopper; the host marks the slot handled.
    Parked,
}

pub fn update<W: WalkerWorld>(actor: &mut Actor, world: &mut W, revision: ObjectRevision) -> Effect {
    let leg = match actor.state {
        // `$58734`: rise one pixel per call, then walk right.
        1 => {
            actor.position.move_y(-1);
            actor.motion_leg = actor.motion_leg.wrapping_add(1);
            if actor.motion_leg == RISE {
                actor.state = 3;
                actor.defeat_mode = 0;
            }
            return Effect::None;
        }
        2 => WalkLeg::WalkLeft,
        3 => WalkLeg::WalkRight,
        4 => WalkLeg::FallLeft,
        5 => WalkLeg::FallRight,
        _ => {
            actor.contact_kind = ContactKind::from_raw(PARKED_CONTACT);
            actor.motion_leg = 0;
            // `$5872A`: the defeat-mode byte, still inside the block.
            actor.defeat_mode = revision.parked_priority();
            return Effect::Parked;
        }
    };
    step_walk(actor, leg, world);
    Effect::None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Open;

    impl WalkerWorld for Open {
        fn probe_left_wall(&mut self, _actor: &Actor) -> bool {
            false
        }
        fn probe_right_wall(&mut self, _actor: &Actor) -> bool {
            false
        }
        fn probe_ground(&mut self, _actor: &Actor) -> bool {
            true
        }
        fn step_fall(&mut self, _actor: &mut Actor) -> u8 {
            0
        }
    }

    #[test]
    fn dx_hopper_parks_without_the_behind_background_bit() {
        let mut actor = Actor::default();
        assert_eq!(update(&mut actor, &mut Open, ObjectRevision::Dx), Effect::Parked);
        assert_eq!((actor.contact_kind.raw(), actor.defeat_mode), (0x80, 0));
    }

    #[test]
    fn hopper_parks_rises_eight_pixels_then_walks_right() {
        let mut actor = Actor::default();
        actor.position.y = 0x100;
        assert_eq!(update(&mut actor, &mut Open, ObjectRevision::Original), Effect::Parked);
        assert_eq!((actor.contact_kind.raw(), actor.defeat_mode), (0x80, 0x80));
        actor.state = 1;
        for _ in 0..8 {
            update(&mut actor, &mut Open, ObjectRevision::Original);
        }
        assert_eq!(
            (actor.state, actor.defeat_mode, actor.position.y),
            (3, 0, 0xF8)
        );
        update(&mut actor, &mut Open, ObjectRevision::Original);
        assert_eq!((actor.state, actor.position.x), (3, 1));
    }
}
