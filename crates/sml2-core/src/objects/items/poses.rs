//! Native item poses and markers, bank $16: the block item that rises out
//! (`$58755`/`$58758`), the touch-to-spin poses 86/87 (`$587BD`/`$587D7`), the
//! fixed contact markers (`$586E1`/`$586EE`) and the rising pickup (`$586F5`).
//! Slot mapping and slot removal live in the `items` adapter.
use super::shell_behavior::animate_spin_wobble;
use crate::actor::{Actor, ContactKind, FrameTick};

/// Pixels the block item rises before it settles.
const RISE_HEIGHT: u8 = 8;
/// The marker byte of an item whose block was already struck.
const STRUCK_MARKER: u8 = 0x10;
/// The struck item drops back into its block this far.
const STRUCK_DROP: i16 = 8;
/// Struck items arm `definition + 0x70`.
const STRUCK_CONTACT_BASE: u8 = 0x70;
/// Markers arm `definition + 0x66` (`$586EE`, `$586EB`).
const MARKER_CONTACT_BASE: u8 = 0x66;
/// Pixels the pickup rises before it is removed.
const PICKUP_RISE: u8 = 0x20;
/// Pixels the pickup rises per update.
const PICKUP_SPEED: u8 = 2;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum PoseEffect {
    None,
    /// The item waits in its block: the host marks the slot done.
    Parked,
    /// The host removes the slot in remove mode 2.
    Remove,
}

/// `$58755`: flash every other frame, then [`update_block_item`].
pub(super) fn update_flashing_block_item(actor: &mut Actor, frame: FrameTick) -> PoseEffect {
    actor.flash.flash_palette_on_tick(frame, 2);
    update_block_item(actor, frame)
}

/// `$58758`: wait in the block until struck, rise out on even frames, settle.
pub(super) fn update_block_item(actor: &mut Actor, frame: FrameTick) -> PoseEffect {
    match actor.state {
        // `$58787`
        1 => {
            if !frame.every(2) {
                return PoseEffect::None;
            }
            actor.position.move_y(-1);
            actor.motion_leg = actor.motion_leg.wrapping_add(1);
            if actor.motion_leg == RISE_HEIGHT {
                actor.state = 2;
                actor.defeat_mode = 0;
            }
            PoseEffect::None
        }
        2 => PoseEffect::None,
        _ => {
            actor.state = 2;
            if actor.marker == STRUCK_MARKER {
                // `$58777`: the activated pose.
                actor.contact_kind =
                    ContactKind::from_raw(actor.definition.wrapping_add(STRUCK_CONTACT_BASE));
                actor.position.move_y(STRUCK_DROP);
                return PoseEffect::None;
            }
            actor.contact_kind = ContactKind::from_raw(0x80);
            // `$58769`
            actor.defeat_mode = frame.revision().parked_priority();
            PoseEffect::Parked
        }
    }
}

/// Wait with `contact` armed until touched, then spin harmlessly.
fn wait_for_touch(actor: &mut Actor, contact: u8) {
    actor.contact_kind = ContactKind::from_raw(contact);
    start_spin_if_touched(actor);
}

fn start_spin_if_touched(actor: &mut Actor) {
    if actor.interactions.touched() {
        actor.state = 1;
        actor.contact_kind = ContactKind::NONE;
    }
}

/// `$587BD`: pose 86.
pub(super) fn update_pose86(actor: &mut Actor, frame: FrameTick) {
    if actor.state == 1 {
        animate_spin_wobble(actor, frame);
        return;
    }
    wait_for_touch(actor, 0x86);
}

/// `$587D7`: pose 87, which never appears on the secret route.
pub(super) fn update_pose87(actor: &mut Actor, frame: FrameTick, secret_route: bool) -> PoseEffect {
    match actor.state {
        1 => animate_spin_wobble(actor, frame),
        2 => start_spin_if_touched(actor),
        // `$587FB`
        _ if secret_route => return PoseEffect::Remove,
        _ => {
            actor.contact_kind = ContactKind::from_raw(0x87);
            actor.state = 2;
        }
    }
    PoseEffect::None
}

/// `$586E1`.
pub(super) fn update_marker85(actor: &mut Actor) {
    actor.contact_kind = ContactKind::from_raw(0x85);
}

/// `$586EE`, also the tail of `$586EB`.
pub(super) fn update_marker66(actor: &mut Actor) {
    actor.contact_kind = ContactKind::from_raw(actor.definition.wrapping_add(MARKER_CONTACT_BASE));
}

/// `$586F5`: flash and rise two pixels per update, removed after 32.
pub(super) fn update_pickup(actor: &mut Actor, frame: FrameTick) -> PoseEffect {
    actor.state = 1;
    actor.flash.flash_palette_on_tick(frame, 2);
    actor.position.move_y(-i16::from(PICKUP_SPEED));
    actor.motion_leg = actor.motion_leg.wrapping_add(PICKUP_SPEED);
    if actor.motion_leg == PICKUP_RISE {
        return PoseEffect::Remove;
    }
    PoseEffect::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::InteractionFlags;

    #[test]
    fn dx_block_item_parks_without_the_behind_background_bit() {
        let mut actor = Actor::default();
        let frame = FrameTick::new(0).with_revision(crate::actor::ObjectRevision::Dx);
        assert_eq!(update_block_item(&mut actor, frame), PoseEffect::Parked);
        assert_eq!((actor.contact_kind.raw(), actor.defeat_mode), (0x80, 0));
    }

    #[test]
    fn block_item_parks_then_rises_eight_pixels_on_even_frames() {
        let mut actor = Actor::default();
        actor.position.y = 0x100;
        assert_eq!(
            update_block_item(&mut actor, FrameTick::new(0)),
            PoseEffect::Parked
        );
        assert_eq!(
            (actor.state, actor.contact_kind.raw(), actor.defeat_mode),
            (2, 0x80, 0x80)
        );
        actor.state = 1;
        update_block_item(&mut actor, FrameTick::new(1));
        assert_eq!(actor.position.y, 0x100);
        for frame in 0..8 {
            update_block_item(&mut actor, FrameTick::new(frame * 2));
        }
        assert_eq!(
            (actor.position.y, actor.state, actor.defeat_mode),
            (0x100 - 8, 2, 0)
        );
    }

    #[test]
    fn struck_block_item_drops_and_arms_its_definition_contact() {
        let mut actor = Actor::default();
        actor.marker = 0x10;
        actor.definition = 3;
        assert_eq!(
            update_block_item(&mut actor, FrameTick::new(0)),
            PoseEffect::None
        );
        assert_eq!(
            (actor.state, actor.contact_kind.raw(), actor.position.y),
            (2, 0x73, 8)
        );
    }

    #[test]
    fn poses_spin_once_touched_and_87_skips_the_secret_route() {
        let mut actor = Actor::default();
        update_pose86(&mut actor, FrameTick::new(0));
        assert_eq!((actor.state, actor.contact_kind.raw()), (0, 0x86));
        actor.interactions = InteractionFlags::from_bits(0x10);
        update_pose86(&mut actor, FrameTick::new(0));
        assert_eq!((actor.state, actor.contact_kind.raw()), (1, 0));

        let mut actor = Actor::default();
        assert_eq!(
            update_pose87(&mut actor, FrameTick::new(0), true),
            PoseEffect::Remove
        );
        update_pose87(&mut actor, FrameTick::new(0), false);
        assert_eq!((actor.state, actor.contact_kind.raw()), (2, 0x87));
        actor.interactions = InteractionFlags::from_bits(0x10);
        update_pose87(&mut actor, FrameTick::new(0), false);
        assert_eq!((actor.state, actor.contact_kind.raw()), (1, 0));
    }

    #[test]
    fn pickup_rises_sixteen_updates_then_is_removed() {
        let mut actor = Actor::default();
        actor.position.y = 0x80;
        for _ in 0..15 {
            assert_eq!(update_pickup(&mut actor, FrameTick::new(1)), PoseEffect::None);
        }
        assert_eq!(update_pickup(&mut actor, FrameTick::new(1)), PoseEffect::Remove);
        assert_eq!(actor.position.y, 0x80 - 32);
    }
}
